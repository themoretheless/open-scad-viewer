import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdir, readFile, writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import {chromium} from 'playwright'
import {createServer} from 'vite'

const output = resolve(process.argv[2] || 'output/qualification/candidate-direct-api')
// Candidate overrides are confined to this Vite process and private output.
const candidate = resolve(process.argv[3])
const raw = await readFile(candidate)
const stage = resolve(output, 'private-pack')
await mkdir(resolve(stage, 'generated'), {recursive:true})
await mkdir(resolve(stage, 'public'), {recursive:true})
const {packGeometryKernel} = await import('../../scripts/pack-geometry-kernel.mjs')
packGeometryKernel(raw, resolve(stage,'generated'), resolve(stage,'public'))
const protectedPaths=['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/identity.ts','src/generated/geometry-kernels/bytes.ts']
const digest = bytes => createHash('sha256').update(bytes).digest('hex')
const before = await Promise.all(protectedPaths.map(async path => digest(await readFile(path))))
function candidatePlugin() {
  return {name:'isolated-candidate-only',enforce:'pre',
    async load(id) {
      for(const file of ['identity.ts','bytes.ts']) {
        if(id.split('?')[0]===resolve('src/generated/geometry-kernels',file))
          return await readFile(resolve(stage,'generated',file),'utf8')
      }
    },
    configureServer(server) {
      server.middlewares.use((request,response,next)=>{
        if(request.url?.split('?')[0]!=='/wasm/geometry-kernel.wasm')return next()
        response.setHeader('Content-Type','application/wasm');response.end(raw)
      })
    }
  }
}
const identity = {sha256: createHash('sha256').update(raw).digest('hex'), byteLength: raw.length}
let server, browser
try {
  server = await createServer({plugins:[candidatePlugin()],worker:{plugins:()=>[candidatePlugin()]},server: {host: '127.0.0.1', port: 0}})
  await server.listen()
  const address = server.httpServer.address()
  assert(address && typeof address !== 'string')
  browser = await chromium.launch({headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath: process.env.CHROMIUM_EXECUTABLE} : {})})
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route('**/__candidate-direct-api', route => route.fulfill({
    contentType: 'text/html', body: '<!doctype html><title>Candidate API probe</title>'}))
  await page.goto(`http://127.0.0.1:${address.port}/__candidate-direct-api`)
  const result = await page.evaluate(async identity => {
    const response = await fetch('/wasm/geometry-kernel.wasm')
    if (!response.ok) throw new Error('Candidate WASM fetch failed')
    const bytes = await response.arrayBuffer()
    const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
      .map(byte => byte.toString(16).padStart(2, '0')).join('')
    if (digest !== identity.sha256 || bytes.byteLength !== identity.byteLength) throw new Error('Candidate mismatch')
    const kernel = await import('/src/services/geometry/kernel.ts')
    await kernel.warmGeometryKernel()
    const fixtures = await import('/tools/browser-qualification/candidate-direct-api-fixtures.ts')
    const main = fixtures.runCandidateFixtures()
    const module = await WebAssembly.compile(bytes)
    const worker = new Worker('/tools/browser-qualification/candidate-direct-api-worker.ts', {type: 'module'})
    try {
      const thread = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('Candidate worker timeout')), 30000)
        worker.onerror = event => {clearTimeout(timer); reject(new Error(event.message))}
        worker.onmessage = event => {clearTimeout(timer); resolve(event.data)}
        worker.postMessage({module, identity})
      })
      return {main, thread}
    } finally {worker.terminate()}
  }, identity)
  assert.equal(result.main.passed, true)
  assert.equal(result.thread.ok, true, JSON.stringify(result.thread))
  assert.equal(result.thread.result.passed, true)
  assert.deepEqual(result.thread.identity, identity)
  assert.deepEqual(errors, [])
  assert.deepEqual(await Promise.all(protectedPaths.map(async path => digest(await readFile(path)))),before)
  const report = {productionFilesUnchanged:true, candidate, schema: 'candidate-direct-browser-api/1', identity, browserVersion: browser.version(),
    scope: 'Finite Chromium development-module API probes on main thread and qualification worker. No provider admission, production packaging or whole-UI qualification.',
    qualificationClaim: 'none', result, pageErrors: errors, passed: true}
  await mkdir(output, {recursive: true})
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', {flag: 'wx'})
  console.log(JSON.stringify(report))
} finally {
  await browser?.close()
  await server?.close()
}
