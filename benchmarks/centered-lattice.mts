import assert from 'node:assert/strict'
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import ts from 'typescript'
import {CadGeometryKernel} from '../src/services/cadGeometryKernel'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {extrudeDirectSketch, type DirectBody} from '../src/services/directModeling'
import {spatialGraph, type LighteningOptions} from '../src/services/solidLightening'

// Pin the pre-port implementation. Only its pure spatialGraph export is called;
// external geometry imports are unavailable in the reference module. Evaluate
// in the same JS realm so a VM context does not bias the host timing.
const referenceCommit = '7b3afccf379aead6151c7ceaf9b633e40eb62924'
const source = execFileSync('git', ['show', `${referenceCommit}:src/services/solidLightening.ts`], {encoding: 'utf8'})
const referenceExports: {spatialGraph?: typeof spatialGraph} = {}
const evaluateReference = new Function('exports', 'require', ts.transpileModule(source, {compilerOptions: {
 target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
}}).outputText)
evaluateReference(referenceExports, () => ({}))
assert.equal(typeof referenceExports.spatialGraph, 'function')
const reference = (body: DirectBody, options: LighteningOptions) =>
 JSON.parse(JSON.stringify(referenceExports.spatialGraph!(body, options))) as ReturnType<typeof spatialGraph>
const artifact = readFileSync('public/wasm/geometry-kernel.wasm')
let loaded = false
setOptionalWasmCompiler(async url => {
 if (url !== '/wasm/geometry-kernel.wasm') return null
 loaded = true
 return WebAssembly.compile(artifact)
})
const session = await new CadGeometryKernel().openSession()
assert.equal(loaded, true)
const options: LighteningOptions = {pattern: 'bcc', cell: 10, rib: 3.2, axis: 'z',
 rim: 0, bottom: 0, top: 0, seed: 42, jitter: 0.5, lineWidth: 0.45, perimeters: 3}
const body = (size: number[], offset = 0) => extrudeDirectSketch({id: 's', name: 'Box', closed: true,
 points: [[offset, offset], [offset + size[0], offset],
  [offset + size[0], offset + size[1]], [offset, offset + size[1]]]}, size[2], '0')
let accepted = 0, refused = 0
const results = []
try {
 for (const pattern of ['bcc', 'octet'] as const) {
  for (let x = 1; x <= 4; x++) for (let y = 1; y <= 4; y++) for (let z = 1; z <= 4; z++) {
   for (const offset of [0, -7.125]) {
    const b = body([x * 9.91, y * 9.91, z * 9.91], offset), o = {...options, pattern}
    let expected: ReturnType<typeof spatialGraph>
    try { expected = reference(b, o) }
    catch {
     assert.throws(() => spatialGraph(b, o), /125 nodes/)
     refused++
     continue
    }
    assert.deepEqual(spatialGraph(b, o), expected)
    accepted++
   }
  }
  for (const size of [[10, 10, 10], [30, 20, 20]]) {
   const b = body(size), o = {...options, pattern}, expected = reference(b, o)
   // No serialization/normalization in the timed reference call.
   const calls = [() => referenceExports.spatialGraph!(b, o), () => spatialGraph(b, o)]
   for (let i = 0; i < 20; i++) for (const call of calls) call()
   const samples: number[][] = [[], []]
   for (let i = 0; i < 31; i++) for (const index of i % 2 ? [1, 0] : [0, 1]) {
    const start = performance.now()
    const actual = calls[index]()
    samples[index].push(performance.now() - start)
    assert.deepEqual(JSON.parse(JSON.stringify(actual)), expected)
   }
   const median = (values: number[]) => [...values].sort((a, b) => a - b)[15]
   results.push({pattern, size, nodes: expected.nodes.length, edges: expected.edges.length,
    referenceMs: median(samples[0]), wasmBoundaryMs: median(samples[1]), samples})
  }
 }
} finally {session.dispose()}
console.log(JSON.stringify({referenceCommit, accepted, refused, node: process.version,
 platform: process.platform, arch: process.arch,
 artifactSha256: createHash('sha256').update(artifact).digest('hex'),
 scope: 'Warm pure TS graph vs complete Rust graph ABI call; includes mesh transfer/validation. No full lightening or UI timing.',
 results}, null, 2))
