import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import { parseArgs } from 'node:util'
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads'
import { deflateRawSync } from 'node:zlib'
import { register } from 'tsx/esm/api'

const hash = bytes => createHash('sha256').update(bytes).digest('hex')
if (isMainThread) {
  const { values } = parseArgs({ options: {
    decoder: { type: 'string', multiple: true }, out: { type: 'string' }, samples: { type: 'string', default: '9' },
  } })
  const count = Number(values.samples)
  assert.ok(Number.isInteger(count) && count >= 1 && count <= 50, 'samples must be 1..50')
  const paths = (values.decoder ?? ['crates/target/wasm32-unknown-unknown/release/wasm_brotli.wasm']).map(path => resolve(path))
  assert.ok(paths.length <= 4, 'at most four decoder variants')
  const inputs = []
  for (const path of paths) inputs.push({ path, sha256: hash(await readFile(path)) })
  const hostPaths = ['benchmarks/wasm-bootstrap.mjs', 'src/services/wasmPacking.ts', 'src/generated/geometry-kernels/bytes.ts', 'src/generated/geometry-kernels/kernel_bg.wasm']
  const hostInputs = []
  for (const path of hostPaths) hostInputs.push({ path, sha256: hash(await readFile(path)) })
  const samples = []
  const startedAt = new Date().toISOString()
  for (let iteration = 0; iteration < count; iteration++) {
    // Alternate order to avoid always measuring one candidate later in the run.
    for (const input of iteration % 2 ? [...inputs].reverse() : inputs) {
      const worker = new Worker(new URL(import.meta.url), { workerData: { path: input.path }, env: {} })
      let timer
      try {
        const sample = await new Promise((resolve, reject) => {
          timer = setTimeout(() => reject(new Error('Bootstrap benchmark exceeded 10 seconds')), 10_000)
          worker.once('error', reject)
          worker.once('exit', code => reject(new Error(`Worker exited without a sample (${code})`)))
          worker.once('message', resolve)
        })
        assert.equal(sample.decoderSha256, input.sha256)
        samples.push({ iteration, path: input.path, ...sample })
      } finally { clearTimeout(timer); await worker.terminate() }
    }
  }
  for (const input of [...inputs, ...hostInputs]) assert.equal(hash(await readFile(input.path)), input.sha256, `${input.path} changed`)
  const phases = ['inflateMs', 'compileMs', 'instantiateMs', 'uploadMs', 'decodeMs', 'copyMs', 'totalMs']
  const summary = inputs.map(({ path }) => {
    const selected = samples.filter(sample => sample.path === path)
    const result = { path, wasmBytes: selected[0].wasmBytes, literalBytes: selected[0].literalBytes }
    for (const phase of phases) {
      const sorted = selected.map(sample => sample[phase]).sort((a, b) => a - b)
      result[phase] = { n: sorted.length, min: sorted[0], p50: sorted[Math.ceil(sorted.length / 2) - 1], max: sorted.at(-1) }
    }
    return result
  })
  const report = {
    startedAt, inputs, hostInputs,
    environment: { node: process.version, v8: process.versions.v8, platform: process.platform, arch: process.arch, release: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] },
    boundaries: [
      'Fresh worker/isolate and decoder instance for each sample; process-wide V8/OS caches may warm. No discarded warmups.',
      'Same generated geometry package, exact decoded bytes asserted against kernel_bg.wasm outside timings.',
      'Measures synchronous decoder bootstrap, upload, decoding and output copy, not geometry compilation or whole-job latency.',
      'Worker creation, module imports, fixture reads, decoder packaging, verification and join excluded from timing.',
      'Single worker at a time, alternating variant order. Run without other builds/tests/profilers. No timing threshold.',
    ], samples, summary,
  }
  if (values.out) {
    await mkdir(dirname(resolve(values.out)), { recursive: true })
    await writeFile(values.out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
    console.log(JSON.stringify({ summary, report: values.out }, null, 2))
  } else console.log(JSON.stringify(report, null, 2))
} else {
  register()
  const { default: packedGeometry } = await import('../src/generated/geometry-kernels/bytes.ts')
  const { unpackWasmBase64 } = await import('../src/services/wasmPacking.ts')
  assert.ok(packedGeometry.startsWith('b85:'), 'benchmark expects generated base85 geometry')
  const wasm = await readFile(workerData.path)
  const header = Buffer.alloc(4)
  header.writeUInt32LE(wasm.length)
  const literal = Buffer.concat([header, deflateRawSync(wasm, { level: 9 })]).toString('base64')
  const start = performance.now()
  const decoded = unpackWasmBase64(literal)
  const inflated = performance.now()
  const module = new WebAssembly.Module(decoded)
  const compiled = performance.now()
  const decoder = new WebAssembly.Instance(module).exports
  const instantiated = performance.now()
  const encoded = new TextEncoder().encode(packedGeometry.slice(4))
  const pointer = decoder.prepare_encoded(encoded.length)
  assert.ok(pointer)
  new Uint8Array(decoder.memory.buffer, pointer, encoded.length).set(encoded)
  const uploaded = performance.now()
  const result = decoder.decode_encoded()
  const decompressed = performance.now()
  assert.notEqual(result, 0n)
  const bytes = new Uint8Array(decoder.memory.buffer, Number(result & 0xffffffffn), Number(result >> 32n)).slice()
  const copied = performance.now()
  const original = await readFile(new URL('../src/generated/geometry-kernels/kernel_bg.wasm', import.meta.url))
  assert.equal(Buffer.compare(original, bytes), 0, 'decoded geometry bytes changed')
  parentPort.on('message', () => {})
  parentPort.postMessage({
    decoderSha256: hash(wasm), geometrySha256: hash(bytes), wasmBytes: wasm.length, literalBytes: literal.length,
    inflateMs: inflated - start, compileMs: compiled - inflated, instantiateMs: instantiated - compiled,
    uploadMs: uploaded - instantiated, decodeMs: decompressed - uploaded, copyMs: copied - decompressed, totalMs: copied - start,
  })
}
