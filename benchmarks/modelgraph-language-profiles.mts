import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { parseArgs } from 'node:util'
import { compileWasmArtifact } from '../src/services/wasmArtifact'
import { encodeBinary } from '../src/services/valueBinaryCodec'
import { decodePacked, writeLinear } from '../src/services/wasmHost'
import { prepareGraphRust, warmLanguageKernel } from '../src/services/languages/kernel'

const { values } = parseArgs({ options: { baseline: { type: 'string' }, candidate: { type: 'string' } } })
assert.ok(values.baseline && values.candidate, 'Pass --baseline and --candidate optimized language WASM paths')
const hash = (value: Uint8Array | string) => createHash('sha256').update(value).digest('hex')
interface Kernel {
  memory: WebAssembly.Memory
  abi_alloc(length: number): number
  abi_free(pointer: number, length: number): void
  abi_request(op: number, pointer: number, length: number): bigint
}
const variants = []
for (const [name, path] of [['baseline', values.baseline], ['candidate', values.candidate]] as const) {
  const bytes = new Uint8Array(readFileSync(path))
  const identity = { sha256: hash(bytes), byteLength: bytes.length }
  const module = await compileWasmArtifact(bytes, identity)
  assert.deepEqual(WebAssembly.Module.imports(module), [])
  const kernel = (await WebAssembly.instantiate(module)).exports as unknown as Kernel
  const run = (source: string) => {
    const input = encodeBinary(source)
    const pointer = writeLinear(kernel.memory, length => kernel.abi_alloc(length), input)
    assert.ok(pointer, 'Request allocation failed')
    try {
      return decodePacked(kernel.memory, (ptr, len) => kernel.abi_free(ptr, len), kernel.abi_request(4, pointer, input.length))
    } finally { kernel.abi_free(pointer, input.length) }
  }
  variants.push({ name, path, identity, run })
}
const fixtures = ['ring-pattern', 'indented-functions', 'skadis-box-linq', 'planetary-spinner'].map(name => ({
  name, source: readFileSync(new URL(`../examples/modelgraph-text/${name}.mg`, import.meta.url), 'utf8'),
}))
for (const count of [16, 128]) fixtures.push({ name: `repeat-${count}`, source:
  `// @modelgraph-text/1\nparts = repeat(${count}, i => box([2mm,3mm,4mm]).move([i*5mm,0,0]))\nshow parts` })

await warmLanguageKernel()
const results = []
for (const { name, source } of fixtures) {
  assert.ok(source.length <= 262144)
  const expected = prepareGraphRust('text', source)
  assert.equal(expected.ok, true, `${name}: ${JSON.stringify(expected)}`)
  for (let i = 0; i < 20; i++) for (const variant of variants) assert.deepEqual(variant.run(source), expected)
  const samples: Record<string, number[]> = { baseline: [], candidate: [] }
  for (let i = 0; i < 31; i++) for (const variant of i % 2 ? [...variants].reverse() : variants) {
    const start = performance.now(), response = variant.run(source)
    samples[variant.name].push(performance.now() - start)
    assert.deepEqual(response, expected)
  }
  results.push({ name, sourceSha256: hash(source), responseSha256: hash(JSON.stringify(expected)),
    results: variants.map(({ name }) => ({ name, medianMs: [...samples[name]].sort((a, b) => a - b)[15], samplesMs: samples[name] })) })
}
console.log(JSON.stringify({ node: process.version, arch: process.arch, platform: process.platform,
  scope: 'ModelGraph Text ABI op 4: source parsing, canonical graph preparation, request encoding, response decoding and frees. Valid-input boundary only; excludes compilation/startup, geometry execution, worker transport and UI. Separate instances, 20 warmups and 31 alternating paired samples per fixture. Exact full-response equality against the current product wrapper checked outside timing.',
  artifacts: variants.map(({ name, path, identity }) => ({ name, path, ...identity })),
  harnessSha256: hash(readFileSync(new URL(import.meta.url))), results }, null, 2))
