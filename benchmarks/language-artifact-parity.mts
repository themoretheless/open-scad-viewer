import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync } from 'node:fs'
import { encodeBinary } from '../src/services/valueBinaryCodec'
import { decodePacked, writeLinear } from '../src/services/wasmHost'

interface Exports extends WebAssembly.Exports {
  memory: WebAssembly.Memory
  abi_alloc(size: number): number
  abi_free(pointer: number, size: number): void
  abi_request(op: number, pointer: number, size: number): bigint
}
const hash = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const paths = process.argv.slice(2)
assert.equal(paths.length, 2, 'Pass reference.wasm and candidate.wasm')
const artifacts = paths.map(path => {
  const bytes = readFileSync(path)
  const module = new WebAssembly.Module(bytes)
  assert.deepEqual(WebAssembly.Module.imports(module), [])
  return { path, sha256: hash(bytes), wasm: new WebAssembly.Instance(module).exports as Exports }
})
function request(wasm: Exports, op: number, value: unknown) {
  const bytes = encodeBinary(value)
  const pointer = writeLinear(wasm.memory, size => wasm.abi_alloc(size), bytes)
  assert.ok(pointer, 'Request allocation refused')
  try {
    return decodePacked(wasm.memory, (p, size) => wasm.abi_free(p, size), wasm.abi_request(op, pointer, bytes.length))
  } finally { wasm.abi_free(pointer, bytes.length) }
}
const cases: { name: string; op: number; value: unknown }[] = []
for (const name of readdirSync('examples/modelgraph-text').filter(name => name.endsWith('.mg')).sort()) {
  const source = readFileSync(`examples/modelgraph-text/${name}`, 'utf8')
  for (const op of [1, 4]) cases.push({ name, op, value: source })
}
for (const source of ['cube([1,2,3]);', 'for(i=[0:3]) translate([i*2,0,0]) sphere(1);',
  'module box(s=2){cube(s);} box();', 'assert(false,"invalid model"); cube(1);', 'cube(;']) {
  for (const profile of ['openscad-viewer-subset@1', 'openscad/stable-2021.01']) {
    for (const op of [10, 11]) cases.push({ name: `${profile}:${source}`, op, value: { source, profile } })
  }
}
const results = cases.map(({ name, op, value }) => {
  const expected = request(artifacts[0].wasm, op, value)
  const actual = request(artifacts[1].wasm, op, value)
  assert.deepEqual(actual, expected, `${name}, ABI op ${op}`)
  return { name, op, ok: (actual as { ok: boolean }).ok, inputSha256: hash(encodeBinary(value)), outputSha256: hash(encodeBinary(actual)) }
})
for (const op of [1, 4, 10, 11]) assert.ok(results.some(result => result.op === op && result.ok), `No successful ABI op ${op}`)
assert.ok(results.some(result => result.ok === false), 'No refusal coverage')
console.log(JSON.stringify({ schema: 1, artifacts: artifacts.map(({ path, sha256 }) => ({ path, sha256 })), cases: results }, null, 2))
