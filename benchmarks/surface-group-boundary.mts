import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {CadGeometryKernel} from '../src/services/cadGeometryKernel'
import {surfaceGroupsInKernel} from '../src/services/geometry/meshAnalysis'
import {inferSurfaceIds} from '../src/services/meshSurfaceGroups'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {compileWasmArtifact} from '../src/services/wasmArtifact'

const artifactPath = process.argv[2] ?? 'public/wasm/geometry-kernel.wasm'
const artifact = readFileSync(artifactPath)
let loaded = false
setOptionalWasmCompiler(async (url, identity) => {
  if (url !== '/wasm/geometry-kernel.wasm') return null
  loaded = true
  assert.ok(identity, 'Historical artifacts require their matching generated build identity')
  return compileWasmArtifact(artifact, identity)
})
const session = await new CadGeometryKernel().openSession()
assert.equal(loaded, true, 'Benchmark did not load the selected WASM artifact')
const results = []
try {
  const fixtures: {name: string; vertices: Float32Array; indices: Uint32Array}[] = []
  for (const triangles of [128, 8192, 65536]) {
    const vertices = new Float32Array((triangles / 2 + 1) * 12)
    const indices = new Uint32Array(triangles * 3)
    for (let i = 0; i <= triangles / 2; i++) {
      vertices.set([i, 0, 0, 0, 0, 1, i, 1, 0, 0, 0, 1], i * 12)
      if (i < triangles / 2) indices.set([i*2, i*2+2, i*2+1, i*2+1, i*2+2, i*2+3], i*6)
    }
    fixtures.push({name: `strip-${triangles}`, vertices, indices})
  }
  const {CadSolid} = session.module
  for (const [name, solid] of [
    ['sphere-128', CadSolid.sphere(10, 128)],
    ['cylinder-256', CadSolid.cylinder(10, 5, 5, 256)],
  ] as const) {
    const mesh = solid.calculateNormals(0, 52.5).getMesh()
    fixtures.push({name, vertices: mesh.vertProperties, indices: mesh.triVerts})
    solid.delete()
  }
  for (const {name, vertices, indices} of fixtures) {
    const expected = inferSurfaceIds(vertices, indices)
    const run = [() => inferSurfaceIds(vertices, indices), () => surfaceGroupsInKernel(vertices, indices)]
    for (let i = 0; i < 5; i++) for (const call of run) assert.deepEqual(call(), expected)
    const samples: number[][] = [[], []]
    for (let i = 0; i < 15; i++) for (const index of i % 2 ? [1, 0] : [0, 1]) {
      const start = performance.now()
      const actual = run[index]()
      samples[index].push(performance.now() - start)
      assert.deepEqual(actual, expected)
    }
    const p50 = (values: number[]) => [...values].sort((a,b) => a-b)[Math.floor(values.length / 2)]
    const inputSha256 = createHash('sha256')
      .update(new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
      .update(new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength)).digest('hex')
    results.push({name, triangles: indices.length / 3, inputSha256, hostMs: p50(samples[0]), wasmBoundaryMs: p50(samples[1]), samples})
  }
} finally { session.dispose() }
const hashes = Object.fromEntries([
  'src/services/meshSurfaceGroups.ts',
  'crates/geometry-bridge/src/mesh_surface_groups.rs', 'benchmarks/surface-group-boundary.mts',
].map(path => [path, createHash('sha256').update(readFileSync(path)).digest('hex')]))
console.log(JSON.stringify({node: process.version, arch: process.arch, platform: process.platform,
  artifactPath, artifactSha256: createHash('sha256').update(artifact).digest('hex'),
  sourceHashesDescribe: 'working tree, not necessarily the selected historical artifact',
  scope: 'warm standalone grouping, including WASM uploads/copy/free; not complete scene publication or cache hits',
  warmups: 5, samples: 15, hashes, results}, null, 2))
