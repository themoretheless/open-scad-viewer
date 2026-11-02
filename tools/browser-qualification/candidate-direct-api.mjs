import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdir, readFile, writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import {chromium} from 'playwright'
import {createServer} from 'vite'

const output = resolve(process.argv[2] || 'output/qualification/candidate-direct-api')
const raw = await readFile('public/wasm/geometry-kernel.wasm')
const identity = {sha256: createHash('sha256').update(raw).digest('hex'), byteLength: raw.length}
let server, browser
try {
  server = await createServer({server: {host: '127.0.0.1', port: 0}})
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
  const report = {schema: 'candidate-direct-browser-api/1', identity, browserVersion: browser.version(),
    scope: 'Finite Chromium development-module API probes on main thread and qualification worker. No provider admission, production packaging or whole-UI qualification.',
    qualificationClaim: 'none', result, pageErrors: errors, passed: true}
  await mkdir(output, {recursive: true})
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', {flag: 'wx'})
  console.log(JSON.stringify(report))
} finally {
  await browser?.close()
  await server?.close()
}
