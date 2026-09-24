import { describe, expect, it } from 'vitest'
import { CadGeometryKernel } from '../src/services/cadGeometryKernel'
import { withCadMesh, type CadMeshViews } from '../src/services/geometry/kernel'
import { renderMeshInKernel, type KernelRenderMesh } from '../src/services/geometry/meshAnalysis'

type Vec3 = [number, number, number]
// Independent host oracle for the former triangle-order, rounded-normal contract.
function reference(mesh: CadMeshViews, cosine: number): KernelRenderMesh {
  const point = (id: number): Vec3 => [mesh.positions[id * 3]!, mesh.positions[id * 3 + 1]!, mesh.positions[id * 3 + 2]!]
  const normalize = (v: Vec3): Vec3 => { const length = Math.hypot(...v); return length ? v.map(x => x / length) as Vec3 : [0, 0, 0] }
  const adjacent: number[][] = Array.from({ length: mesh.positions.length / 3 }, () => [])
  const normals: Vec3[] = []
  for (let i = 0; i < mesh.indices.length; i += 3) {
    const p = point(mesh.indices[i]!), q = point(mesh.indices[i + 1]!), r = point(mesh.indices[i + 2]!)
    const a = q.map((x, k) => x - p[k]!) as Vec3, b = r.map((x, k) => x - p[k]!) as Vec3
    normals.push(normalize([a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]))
    for (let j = 0; j < 3; j++) adjacent[mesh.indices[i + j]!]!.push(i / 3)
  }
  const vertices: number[] = [], indices: number[] = [], rawIds: number[] = []
  const unique = new Map<string, number>()
  for (let i = 0; i < mesh.indices.length; i++) {
    const id = mesh.indices[i]!, face = normals[Math.floor(i / 3)]!
    const sum: Vec3 = [0, 0, 0]
    for (const t of adjacent[id]!) {
      const n = normals[t]!
      if (n[0] * face[0] + n[1] * face[1] + n[2] * face[2] >= cosine - 1e-10) {
        for (let k = 0; k < 3; k++) sum[k]! += n[k]!
      }
    }
    const normal = normalize(sum), key = `${id}:${normal.map(x => Math.round(x * 1e7)).join(',')}`
    let index = unique.get(key)
    if (index === undefined) {
      index = vertices.length / 6
      unique.set(key, index)
      vertices.push(...point(id), ...normal)
      rawIds.push(id)
    }
    indices.push(index)
  }
  const first = new Map<number, number>(), mergeFrom: number[] = [], mergeTo: number[] = []
  rawIds.forEach((id, index) => {
    const previous = first.get(id)
    if (previous === undefined) first.set(id, index)
    else { mergeFrom.push(index); mergeTo.push(previous) }
  })
  return { vertices: Float32Array.from(vertices), indices: Uint32Array.from(indices), mergeFrom: Uint32Array.from(mergeFrom), mergeTo: Uint32Array.from(mergeTo), faceIds: mesh.faceIds.slice() }
}

describe('display mesh normal compatibility', () => {
  it.each(['cube', 'sphere', 'dense', 'cylinder', 'transformed', 'empty'])(
    'preserves the host reference buffer contract: %s', async fixture => {
      const session = await new CadGeometryKernel().openSession()
      try {
        const { CadSolid } = session.module
        const solid = fixture === 'cube' ? CadSolid.cube([2, 3, 4], true)
          : fixture === 'sphere' ? CadSolid.sphere(3, 24)
          : fixture === 'dense' ? CadSolid.sphere(30, 128)
          : fixture === 'cylinder' ? CadSolid.cylinder(4, 1, 0.5, 64, true)
          : fixture === 'empty' ? CadSolid.union([])
          : CadSolid.cube([2, 3, 4], true).scale([-1, 2, 0.25]).rotate([30, 40, 50]).translate([1e5, -5, 3])
        for (const angle of fixture === 'dense' ? [52.5] : [0, 52.5, 90, 180]) {
          const cosine = Math.cos(angle * Math.PI / 180)
          const expected = withCadMesh(solid.handle, mesh => reference(mesh, cosine))
          const actual = renderMeshInKernel(solid.handle, cosine)
          // Structure stays byte-exact; computed floats get a tolerance
          // because the Rust kernel evaluates positions/normals in f32 while
          // the JS oracle uses f64 (residuals up to ~1.4e-16 near zero).
          // 1e-6 is orders of magnitude above that noise yet still catches
          // real position and normal length/direction regressions.
          const bytes = (value: ArrayBufferView) => Buffer.from(value.buffer, value.byteOffset, value.byteLength)
          for (const key of ['indices', 'mergeFrom', 'mergeTo', 'faceIds'] as const) {
            expect(bytes(actual[key]).equals(bytes(expected[key])), `${fixture}/${angle}/${key}`).toBe(true)
          }
          expect(actual.vertices.length, `${fixture}/${angle}/vertices.length`).toBe(expected.vertices.length)
          for (let i = 0; i < expected.vertices.length; i++) {
            const a = actual.vertices[i]!, e = expected.vertices[i]!
            const slot = i % 6 < 3 ? 'position' : 'normal'
            expect(Math.abs(a - e), `${fixture}/${angle}/vertices[${i}] ${slot}`).toBeLessThanOrEqual(1e-6)
          }
        }
      } finally { session.dispose() }
    },
  )
})
