import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { chromium } from 'playwright'
import { createServer } from 'vite'

const paths = process.argv.slice(2)
assert.equal(paths.length, 2, 'Pass baseline.wasm and candidate.wasm')
const binaries = await Promise.all(paths.map(path => readFile(path)))
const output = resolve(process.env.EXPORT_BENCH_OUTPUT || 'tmp/performance/3mf-browser')
let server, browser
try {
  server = await createServer({ server: { host: '127.0.0.1', port: 4184, strictPort: true } })
  await server.listen()
  browser = await chromium.launch({ headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? { executablePath: process.env.CHROMIUM_EXECUTABLE } : {}),
  })
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route('**/__export-bench', route => route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>3MF benchmark</title>' }))
  for (let index = 0; index < 2; index++) await page.route(`**/__kernel-${index}.wasm`, route => route.fulfill({ contentType: 'application/wasm', body: binaries[index] }))
  await page.goto('http://127.0.0.1:4184/__export-bench')
  const samples = await page.evaluate(async () => {
    const { encodeBinary } = await import('/src/services/valueBinaryCodec.ts')
    const { decodePacked, writeLinear } = await import('/src/services/wasmHost.ts')
    const kernels = []
    for (let index = 0; index < 2; index++) {
      const { instance } = await WebAssembly.instantiate(await (await fetch(`/__kernel-${index}.wasm`)).arrayBuffer())
      kernels.push(instance.exports)
    }
    const results = []
    const unit = [0,0,0, 1,0,0, 1,1,0, 0,1,0, 0,0,1, 1,0,1, 1,1,1, 0,1,1]
    const faces = [0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]
    for (const count of [256, 2048]) {
      const positions = [], indices = []
      for (let cube = 0; cube < count; cube++) {
        for (let i = 0; i < unit.length; i += 3) positions.push(unit[i] + (cube % 64) * 2, unit[i+1] + Math.floor(cube / 64) * 2, unit[i+2])
        indices.push(...faces.map(index => index + cube * 8))
      }
      for (const compressed of [false, true]) {
        const request = encodeBinary({ op: 'mesh_export_3mf', mesh: { positions, indices }, parts: [], compressed })
        const inputDigest = await crypto.subtle.digest('SHA-256', request)
        const inputSha256 = Array.from(new Uint8Array(inputDigest), b => b.toString(16).padStart(2, '0')).join('')
        const run = w => {
          const pointer = writeLinear(w.memory, size => w.abi_alloc(size), request)
          if (!pointer) throw Error('Request allocation failed')
          try {
            const response = decodePacked(w.memory, (p, n) => w.abi_free(p, n), w.abi_request(0, pointer, request.length))
            if (!response.ok) throw Error(JSON.stringify(response))
            const handle = response.value
            try { return new Uint8Array(w.memory.buffer, w.abi_array_field(handle, 0), w.abi_array_field(handle, 1)).slice() }
            finally { w.abi_array_free(handle) }
          } finally { w.abi_free(pointer, request.length) }
        }
        const expected = run(kernels[0])
        for (let variant = 0; variant < 2; variant++) {
          for (let warm = 0; warm < 3; warm++) run(kernels[variant])
          const times = []
          for (let sample = 0; sample < 9; sample++) {
            const start = performance.now()
            const bytes = run(kernels[variant])
            times.push(performance.now() - start)
            if (bytes.length !== expected.length || bytes.some((byte, i) => byte !== expected[i])) throw Error('Archive bytes changed')
          }
          const digest = await crypto.subtle.digest('SHA-256', expected)
          results.push({ count, compressed, variant, inputSha256, requestBytes: request.length, outputBytes: expected.length,
            outputSha256: Array.from(new Uint8Array(digest), b => b.toString(16).padStart(2, '0')).join(''),
            samplesMs: times, medianMs: [...times].sort((a, b) => a - b)[4] })
        }
      }
    }
    return results
  })
  assert.deepEqual(errors, [])
  await mkdir(output, { recursive: true })
  const report = { browser: browser.version(), platform: process.platform, arch: process.arch,
    method: 'Warm full 3MF WASM ABI export; 3 warmups, 9 samples; fetch/compile/fixture creation/byte comparison/hash excluded.',
    artifacts: paths.map((path, i) => ({ path, sha256: createHash('sha256').update(binaries[i]).digest('hex') })), samples, errors }
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n')
  console.log(JSON.stringify(samples.map(({ count, compressed, variant, medianMs }) => ({ count, compressed, variant, medianMs }))))
} finally {
  try { await browser?.close() } finally { await server?.close() }
}
