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
  const result = await page.evaluate(async ({entry, origin, kind, benchmark}) => {
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
        let transfers = 0, transferredBytes = 0, detachedTransfers = 0;
        const post = self.postMessage.bind(self);
        self.postMessage = (message, options) => {
          const buffers = (Array.isArray(options) ? options : options?.transfer ?? []).filter(value => value instanceof ArrayBuffer);
          const bytes = buffers.reduce((sum, buffer) => sum + buffer.byteLength, 0);
          post(message, options);
          transfers += buffers.length;
          transferredBytes += bytes;
          detachedTransfers += buffers.filter(buffer => buffer.byteLength === 0).length;
        };
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
            self.postMessage({__stats: true, digests, synchronousLargeModules, transfers, transferredBytes, detachedTransfers});
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
      : {kind: 'parse', gcode: 'G1 X0 Y0 Z0.2 F600\nM83\nM220 S50\nG1 X10 E1\n'}}
    const run = async (worker, id, transfer = true, job = request.job) => {
      const pending = wait(worker, data => data.version === 1 && data.id === id)
      worker.postMessage({...request, id, job, ...(kind === 'gcode' && transfer ? {responseFormat: 'f64-moves-v1'} : {})})
      const response = await pending
      if (kind === 'gcode' && response.ok && transfer) {
        if (response.responseFormat !== 'f64-moves-v1') throw new Error('Missing transfer format')
        const {moveRows, ...metadata} = response.result.preview
        if (!(moveRows instanceof Float64Array) || moveRows.length % 7 !== 0 || 'moves' in metadata) throw new Error('Invalid transferred moves')
        const moves = new Array(moveRows.length / 7)
        for (let i = 0, j = 0; i < moveRows.length; i += 7, j++) moves[j] = {
          x: moveRows[i], y: moveRows[i + 1], z: moveRows[i + 2], e: moveRows[i + 3],
          feedrateMmS: moveRows[i + 4], layerIndex: moveRows[i + 5], extruded: moveRows[i + 6] === 1,
        }
        response.result.preview = {...metadata, moves}
      }
      return response
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
      const legacy = await run(worker, 4, false)
      let timings
      if (benchmark && kind === 'gcode') {
        let gcode = 'G90\nM83\nG1 X0 Y0 Z0.2 F600\n'
        for (let i = 0; i < 80_000; i++) gcode += `G1 X${(i + 1) % 2} Y0 E0.01 F600\n`
        const job = {kind: 'parse', gcode}, samples = {legacy: [], transfer: []}
        let id = 10
        const expected = await run(worker, id++, false, job)
        if (!expected.ok || expected.result.preview.moves.length !== 80_001) throw new Error('Invalid large fixture')
        if (JSON.stringify((await run(worker, id++, true, job)).result) !== JSON.stringify(expected.result)) throw new Error('Large transport mismatch')
        for (let i = 0; i < 10; i++) { await run(worker, id++, false, job); await run(worker, id++, true, job) }
        for (let i = 0; i < 31; i++) for (const transfer of i % 2 ? [true, false] : [false, true]) {
          const start = performance.now(), value = await run(worker, id++, transfer, job), elapsed = performance.now() - start
          if (!value.ok || value.result.preview.moves.length !== 80_001 || value.result.preview.estimatedTimeS !== expected.result.preview.estimatedTimeS) throw new Error('Invalid benchmark result')
          samples[transfer ? 'transfer' : 'legacy'].push(elapsed)
        }
        timings = Object.fromEntries(Object.entries(samples).map(([name, samplesMs]) => {
          const sorted = [...samplesMs].sort((a, b) => a - b)
          return [name, {p50Ms: sorted[15], p95Ms: sorted[29], samplesMs}]
        }))
      }
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
      return {cold, warm, legacy, first, second, recovered, late, timings}
    } finally { workers.forEach(worker => worker.terminate()); urls.forEach(url => URL.revokeObjectURL(url)) }
  }, {entry, origin, kind, benchmark: process.env.GCODE_WORKER_BENCH === '1'})
  for (const response of [result.cold, result.warm, result.legacy, result.recovered]) {
    assert.equal(response.ok, true)
    if (kind === 'svg') {
      assert.ok(Math.abs(response.result.widthMm - 20) < 0.001)
      assert.ok(Math.abs(response.result.heightMm - 10) < 0.001)
      assert.ok(response.result.svg.includes('<path'))
    } else {
      assert.equal(response.result.preview.extrusionMm, 1)
      assert.equal(response.result.preview.estimatedTimeS, 2)
      assert.equal(response.result.preview.moves.at(-1).feedrateMmS, 5)
      assert.equal(response.result.preview.printDistanceMm, 10)
    }
  }
  assert.deepEqual(result.cold.result, result.warm.result)
  assert.deepEqual(result.cold.result, result.legacy.result)
  assert.deepEqual(result.cold.result, result.recovered.result)
  for (const [index, stats] of [result.first, result.second].entries()) {
    const transfers = kind === 'gcode' ? index + 1 : 0
    assert.deepEqual(stats, {__stats: true, digests: 1, synchronousLargeModules: 0,
      transfers, transferredBytes: transfers * 2 * 7 * 8, detachedTransfers: transfers})
  }
  assert.equal(result.late, 0)
  console.log(JSON.stringify({browser: browser.version(), kind, entry,
    scope: 'Production worker with real embedded WASM, instrumented WebCrypto and buffer detachment; optional warm timings include independent reconstruction, not the panel client or rendering',
    timings: result.timings,
    first: result.first, second: result.second, recovered: result.recovered.ok, lateResponsesAfterTermination: result.late}, null, 2))
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)) }
