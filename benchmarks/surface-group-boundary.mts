import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {CadGeometryKernel} from '../src/services/cadGeometryKernel'
import {surfaceGroupsInKernel} from '../src/services/geometry/meshAnalysis'
import {inferSurfaceIds} from '../src/services/meshSurfaceGroups'

const session = await new CadGeometryKernel().openSession()
const results = []
try {
  for (const triangles of [128, 8192, 65536]) {
    const vertices = new Float32Array((triangles / 2 + 1) * 12)
    const indices = new Uint32Array(triangles * 3)
    for (let i = 0; i <= triangles / 2; i++) {
      vertices.set([i, 0, 0, 0, 0, 1, i, 1, 0, 0, 0, 1], i * 12)
      if (i < triangles / 2) indices.set([i*2, i*2+2, i*2+1, i*2+1, i*2+2, i*2+3], i*6)
    }
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
    results.push({triangles, hostMs: p50(samples[0]), wasmBoundaryMs: p50(samples[1]), samples})
  }
} finally { session.dispose() }
const hashes = Object.fromEntries([
  'public/wasm/geometry-kernel.wasm', 'src/services/meshSurfaceGroups.ts',
  'crates/geometry-bridge/src/mesh_surface_groups.rs', 'benchmarks/surface-group-boundary.mts',
].map(path => [path, createHash('sha256').update(readFileSync(path)).digest('hex')]))
console.log(JSON.stringify({node: process.version, arch: process.arch, platform: process.platform,
  scope: 'warm standalone grouping, including WASM uploads/copy/free; not complete scene publication or cache hits',
  warmups: 5, samples: 15, hashes, results}, null, 2))
