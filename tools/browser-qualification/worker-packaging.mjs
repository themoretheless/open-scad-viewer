import assert from 'node:assert/strict'
import { mkdir, readdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { chromium } from 'playwright'
import { preview } from 'vite'

const output = resolve(process.env.WORKER_PACKAGING_OUTPUT || 'tmp/performance/worker-packaging')
const assets = await readdir('dist/assets')
const workerFile = name => {
  const matches = assets.filter(file => file.startsWith(`${name}.worker-`) && file.endsWith('.js'))
  assert.equal(matches.length, 1, `Expected one production ${name} worker`)
  return matches[0]
}
const files = { svg: workerFile('svg'), gcode: workerFile('gcodePreview') }
let server, browser
try {
  server = await preview({ preview: { host: '127.0.0.1', port: 4183, strictPort: true } })
  browser = await chromium.launch({ headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? { executablePath: process.env.CHROMIUM_EXECUTABLE } : {}),
  })
  const page = await browser.newPage()
  const pageErrors = []
  page.on('pageerror', error => pageErrors.push(error.message))
  // A same-origin inert document avoids unrelated application initialization.
  await page.route('**/__worker-packaging', route => route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>Worker packaging</title>' }))
  await page.goto('http://127.0.0.1:4183/__worker-packaging')
  const result = await page.evaluate(async files => {
    const workers = Object.fromEntries(Object.entries(files).map(([name, file]) => [name, new Worker(`/assets/${file}`, { type: 'module' })]))
    let id = 0
    const run = (worker, job) => new Promise((resolve, reject) => {
      const requestId = ++id
      const finish = (error, response) => {
        clearTimeout(timer)
        worker.removeEventListener('message', message)
        worker.removeEventListener('error', failure)
        worker.removeEventListener('messageerror', failure)
        error ? reject(error) : resolve(response)
      }
      const message = event => {
        if (event.data.id !== requestId) return
        if (event.data.version !== 1) return finish(Error('Invalid response version'))
        finish(null, event.data)
      }
      const failure = event => finish(Error(event.message || 'Worker transport error'))
      const timer = setTimeout(() => finish(Error('Worker request timeout')), 30000)
      worker.addEventListener('message', message)
      worker.addEventListener('error', failure)
      worker.addEventListener('messageerror', failure)
      worker.postMessage({ version: 1, id: requestId, job })
    })
    try {
      const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><rect width="20" height="10"/></svg>'
      const gcode = 'G21\nG90\nM82\nG92 E0\nG1 X0 Y0 Z0.2 F1200\nG1 X10 Y0 E1\nG1 X10 Y10 E2\n'
      const preview = await run(workers.svg, { kind: 'preview', svg, options: {} })
      const extrusion = await run(workers.svg, { kind: 'extrude', svg, options: {}, height: 3 })
      const parsed = await run(workers.gcode, { kind: 'parse', gcode })
      const invalid = await run(workers.gcode, { kind: 'unknown' })
      const recovered = await run(workers.gcode, { kind: 'parse', gcode })
      return { preview, extrusion, parsed, invalid, recovered }
    } finally { Object.values(workers).forEach(worker => worker.terminate()) }
  }, files)
  for (const name of ['preview', 'extrusion', 'parsed', 'recovered']) assert.equal(result[name].ok, true, JSON.stringify(result[name]))
  assert.equal(result.preview.result.widthMm, 20)
  assert.equal(result.preview.result.heightMm, 10)
  assert.equal(result.extrusion.result.source, 'linear_extrude(height=3) polygon(points=[[0,0],[20,0],[20,10],[0,10]],paths=[[0,1,2,3]]);')
  assert.deepEqual(result.parsed.result.preview.moves.map(({ x, y, z, e }) => [x, y, z, e]), [[0, 0, .2, 0], [10, 0, .2, 1], [10, 10, .2, 2]])
  assert.equal(result.parsed.result.preview.extrusionMm, 2)
  assert.equal(result.parsed.result.preview.printDistanceMm, 20)
  assert.equal(result.parsed.result.preview.layers, 1)
  assert.equal(result.invalid.ok, false)
  assert.deepEqual(result.recovered.result, result.parsed.result)
  assert.deepEqual(pageErrors, [])
  await mkdir(output, { recursive: true })
  await writeFile(resolve(output, 'report.json'), JSON.stringify({ browser: browser.version(), files, result, pageErrors }, null, 2) + '\n')
  console.log('Production SVG and G-code workers passed: cold initialization, warm operations, refusal and recovery.')
} finally {
  try { await browser?.close() } finally { await server?.close() }
}
