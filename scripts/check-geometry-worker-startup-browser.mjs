import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile, readdir} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const root = path.resolve('dist')
const kind = process.argv[2] ?? 'gcode'
assert.ok(kind === 'gcode' || kind === 'svg', 'Expected gcode or svg')
const entryPattern = kind === 'svg' ? /^svg\.worker-.*\.js$/ : /^gcodePreview\.worker-.*\.js$/
const entry = (await readdir(path.join(root, 'assets'))).find(name => entryPattern.test(name))
assert.ok(entry, 'Build the application before checking the production worker')
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://localhost')
    if (url.pathname === '/__probe') {
      response.setHeader('Content-Type', 'text/html')
      response.end('<!doctype html><title>Geometry worker startup check</title>')
      return
    }
    const file = path.resolve(root, `.${decodeURIComponent(url.pathname)}`)
    if (!file.startsWith(`${root}${path.sep}`)) { response.writeHead(403).end(); return }
    response.setHeader('Content-Type', file.endsWith('.js') ? 'text/javascript' : 'application/octet-stream')
    response.end(await readFile(file))
  } catch { response.writeHead(404).end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
let browser
try {
  const {playwright} = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath: process.env.CHROMIUM_EXECUTABLE} : {})})
  const page = await browser.newPage()
  const origin = `http://127.0.0.1:${server.address().port}`
  await page.goto(`${origin}/__probe`)
  const result = await page.evaluate(async ({entry, origin, kind}) => {
    const workers = [], urls = []
    const wait = (worker, accept) => new Promise((resolve, reject) => {
      const timer = setTimeout(() => { cleanup(); reject(new Error('Worker probe timeout')) }, 10000)
      const message = event => { if (accept(event.data)) { cleanup(); resolve(event.data) } }
      const error = event => { cleanup(); reject(new Error(event.message)) }
      function cleanup() { clearTimeout(timer); worker.removeEventListener('message', message); worker.removeEventListener('error', error) }
      worker.addEventListener('message', message); worker.addEventListener('error', error)
    })
    const make = async block => {
      const url = URL.createObjectURL(new Blob([`
        let digests = 0, synchronousLargeModules = 0;
        const digest = crypto.subtle.digest.bind(crypto.subtle);
        crypto.subtle.digest = (...args) => {
          digests++;
          if (${JSON.stringify(block)}) {
            self.postMessage({__digestStarted: true});
            return new Promise(() => {});
          }
          return digest(...args);
        };
        WebAssembly.Module = new Proxy(WebAssembly.Module, {
          construct(target, args) {
            if (args[0].byteLength > 1024 * 1024) synchronousLargeModules++;
            return Reflect.construct(target, args);
          }
        });
        self.addEventListener('message', event => {
          if (event.data.__stats) {
            event.stopImmediatePropagation();
            self.postMessage({__stats: true, digests, synchronousLargeModules});
          }
        });
        await import(${JSON.stringify(`${origin}/assets/${entry}`)});
        self.postMessage({__ready: true});
      `], {type: 'text/javascript'}))
      urls.push(url)
      const worker = new Worker(url, {type: 'module'})
      workers.push(worker)
      await wait(worker, data => data.__ready)
      return worker
    }
    const request = {version: 1, id: 1, job: kind === 'svg'
      ? {kind: 'preview', svg: '<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><rect width="20" height="10" fill="red"/></svg>'}
      : {kind: 'parse', gcode: 'G1 X0 Y0 Z0.2 F600\nM83\nG1 X10 E1\n'}}
    const run = async (worker, id) => {
      const pending = wait(worker, data => data.version === 1 && data.id === id)
      worker.postMessage({...request, id})
      return pending
    }
    const stats = async worker => {
      const pending = wait(worker, data => data.__stats)
      worker.postMessage({__stats: true})
      return pending
    }
    try {
      const worker = await make(false)
      const cold = await run(worker, 1), first = await stats(worker)
      const warm = await run(worker, 2), second = await stats(worker)
      const blocked = await make(true)
      const entered = wait(blocked, data => data.__digestStarted)
      blocked.postMessage(request)
      await entered
      let late = 0
      blocked.addEventListener('message', () => late++)
      blocked.terminate()
      const replacement = await make(false)
      const recovered = await run(replacement, 3)
      await new Promise(resolve => setTimeout(resolve, 50))
      return {cold, warm, first, second, recovered, late}
    } finally { workers.forEach(worker => worker.terminate()); urls.forEach(url => URL.revokeObjectURL(url)) }
  }, {entry, origin, kind})
  for (const response of [result.cold, result.warm, result.recovered]) {
    assert.equal(response.ok, true)
    if (kind === 'svg') {
      assert.ok(Math.abs(response.result.widthMm - 20) < 0.001)
      assert.ok(Math.abs(response.result.heightMm - 10) < 0.001)
      assert.ok(response.result.svg.includes('<path'))
    } else {
      assert.equal(response.result.preview.extrusionMm, 1)
      assert.equal(response.result.preview.printDistanceMm, 10)
    }
  }
  assert.deepEqual(result.cold.result, result.warm.result)
  assert.deepEqual(result.cold.result, result.recovered.result)
  assert.deepEqual(result.first, {__stats: true, digests: 1, synchronousLargeModules: 0})
  assert.deepEqual(result.second, result.first)
  assert.equal(result.late, 0)
  console.log(JSON.stringify({browser: browser.version(), kind, entry,
    scope: 'Production worker with real embedded WASM, instrumented WebCrypto and module construction; no latency claim',
    first: result.first, second: result.second, recovered: result.recovered.ok, lateResponsesAfterTermination: result.late}, null, 2))
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)) }
