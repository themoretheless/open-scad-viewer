import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import { parseArgs } from 'node:util'
import { CadGeometryKernel } from '../src/services/cadGeometryKernel'
import { callGeometryRust, withCadMesh } from '../src/services/geometry/kernel'
import { geometryAssetId } from '../src/core/scene'
import { analyzeSolidInKernel, buildBvhInKernel, extractSemanticEdgesInKernel, renderMeshInKernel } from '../src/services/geometry/meshAnalysis'

const { values } = parseArgs({ options: {
  out: { type: 'string' }, samples: { type: 'string', default: '9' }, warmups: { type: 'string', default: '3' },
} })
const sampleCount = Number(values.samples), warmups = Number(values.warmups)
assert(Number.isInteger(sampleCount) && sampleCount >= 1 && sampleCount <= 100)
assert(Number.isInteger(warmups) && warmups >= 0 && warmups <= 20)
const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const paths = [
  'benchmarks/solid-analysis.mts', 'src/services/geometry/kernel.ts', 'src/services/geometry/module.ts',
  'src/services/geometry/meshAnalysis.ts', 'src/services/cadGeometryKernel.ts',
  'crates/geometry-bridge/src/mesh.rs', 'crates/geometry-bridge/src/mesh_render.rs', 'crates/geometry-bridge/src/mesh_analysis.rs',
  'crates/polygon-core/src/solid/bvh.rs', 'crates/polygon-core/src/solid/edges.rs',
  'crates/polygon-core/src/lib.rs', 'crates/polygon-core/src/mesh_topology.rs', 'src/core/scene.ts',
  'src/generated/geometry-kernels/kernel_bg.wasm', 'src/generated/geometry-kernels/bytes.ts',
  'src/generated/wasm-brotli/bytes.ts', 'crates/Cargo.toml', 'crates/Cargo.lock',
]
const fingerprints = () => paths.map(path => ({ path, sha256: existsSync(path) ? hash(readFileSync(path)) : null }))
const files = fingerprints()
const signature = (value: unknown): string => {
  const digest = createHash('sha256')
  const visit = (value: unknown) => {
    if (ArrayBuffer.isView(value)) {
      digest.update(`${value.constructor.name}:${value.byteLength}:`)
      digest.update(new Uint8Array(value.buffer, value.byteOffset, value.byteLength))
    } else if (value !== null && typeof value === 'object') {
      for (const [key, item] of Object.entries(value)) { digest.update(`${key}:`); visit(item) }
    } else digest.update(JSON.stringify(value))
  }
  visit(value)
  return digest.digest('hex')
}
const cosine = Math.cos(52.5 * Math.PI / 180)
const edgeCosine = Math.cos(30 * Math.PI / 180)
const rows: object[] = []
for (const id of ['cube', 'sphere-32', 'sphere-128', 'three-spheres-128', 'cylinder-128']) {
  const session = await new CadGeometryKernel().openSession()
  try {
    const { CadSolid } = session.module
    const solid = id === 'cube' ? CadSolid.cube([2, 3, 4], true)
      : id === 'cylinder-128' ? CadSolid.cylinder(8, 3, 3, 128, true)
      : id === 'three-spheres-128' ? CadSolid.union([0, 90, 180].map(x => CadSolid.sphere(30, 128).translate([x, 0, 0])))
      : CadSolid.sphere(30, id === 'sphere-32' ? 32 : 128)
    const exportMesh = () => withCadMesh(solid.handle, mesh => ({ positions: mesh.positions.slice(), indices: mesh.indices.slice(), faceIds: mesh.faceIds.slice() }))
    const inputSha256 = signature(exportMesh())
    const display = renderMeshInKernel(solid.handle, cosine)
    const bvh = () => buildBvhInKernel(display.vertices, display.indices, 6, 8)
    const edges = () => extractSemanticEdgesInKernel(display.vertices, display.indices, display.mergeFrom, display.mergeTo, false, edgeCosine)
    const combined = analyzeSolidInKernel(solid.handle)
    assert.equal(signature(combined.mesh), signature(display))
    const tree = bvh()
    assert.equal(signature(tree.bounds), signature(combined.bvh.bounds))
    assert.equal(signature(tree.nodes), signature(combined.bvh.nodes))
    assert.equal(signature(tree.triangles), signature(combined.bvh.triangles))
    assert.equal(signature(edges()), signature(combined.semanticEdges))
    const phases = {
      inspect: () => callGeometryRust('cad', { action: 'inspect', id: solid.handle }),
      export: exportMesh, render: () => renderMeshInKernel(solid.handle, cosine), bvh, edges,
      combined: () => analyzeSolidInKernel(solid.handle),
      assetHash: () => geometryAssetId(display.vertices, display.indices),
    }
    for (const [phase, run] of Object.entries(phases)) {
      const outputSha256 = signature(run())
      const samples: number[] = []
      for (let i = 0; i < warmups + sampleCount; i++) {
        const start = performance.now()
        const output = run()
        const elapsed = performance.now() - start
        assert.equal(signature(output), outputSha256, `${id}/${phase}: nondeterministic bytes`)
        if (i >= warmups) samples.push(elapsed)
      }
      const sorted = samples.toSorted((a, b) => a - b), middle = Math.floor(sorted.length / 2)
      const p50 = sorted.length % 2 ? sorted[middle]! : (sorted[middle - 1]! + sorted[middle]!) / 2
      const row = { id, phase, inputSha256, outputSha256, triangles: display.indices.length / 3, samples, min: sorted[0], p50, max: sorted.at(-1) }
      rows.push(row)
      console.log(`${id}/${phase}: ${p50.toFixed(3)} ms`)
    }
  } finally { session.dispose() }
}
assert.deepEqual(fingerprints(), files, 'Benchmark inputs changed during run')
const report = {
  kind: 'solid-analysis', schemaVersion: 2, recordedAt: new Date().toISOString(), warmups, sampleCount,
  source: { head: execFileSync('git', ['-c', 'core.fsmonitor=false', 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), files },
  environment: { node: process.version, v8: process.versions.v8, platform: process.platform, arch: process.arch, release: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] },
  boundaries: [
    'Warm production WASM on retained solid handles, plus separate uncached inspection and host asset-hash probes. Excludes creation, kernel startup, Worker transport, provenance and GPU.',
    'Inspection calls the actual uncached kernel command; the CadSolid wrapper caches this response for subsequent metric reads. Hashing is timed only in the explicit assetHash phase.',
    'Individual phases include their own ABI and owned host copies; BVH/edges also upload inputs. Replays are not an additive decomposition of the combined call.',
    'Sequential samples, no profiling/forced GC/CPU isolation. Run without other builds/tests. Validation and byte hashing are outside timing.',
    'All phases verify deterministic bytes, and combined output matches separate calls. Selected source/artifact fingerprints are not a full build attestation.',
  ], rows,
}
if (values.out) {
  mkdirSync(dirname(resolve(values.out)), { recursive: true })
  writeFileSync(values.out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
} else console.log(JSON.stringify(report, null, 2))
