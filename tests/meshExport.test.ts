import { describe, expect, it } from 'vitest'
import { buildBinaryStl, buildObj } from '../src/services/meshExport'
import { identity, translate } from '../src/services/math3d'
import type { MeshData } from '../src/services/openscadParser'
import { buildMeshBvh } from '../src/services/meshBvh'

function triangleMesh(): MeshData {
  const vertices = new Float32Array([0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1])
  const indices = new Uint32Array([0, 1, 2])
  return {
    vertices,
    indices,
    bvh: buildMeshBvh(vertices, indices),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    color: [1, 1, 1, 1],
    transform: translate(identity(), [2, 3, 4]),
    faceIds: new Uint32Array([0]),
    provenance: [],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

describe('mesh export', () => {
  it('writes a transformed binary STL triangle', () => {
    const stl = buildBinaryStl([triangleMesh()])
    expect(stl.byteLength).toBe(134)
    const view = new DataView(stl.buffer)
    expect(view.getUint32(80, true)).toBe(1)
    expect(view.getFloat32(96, true)).toBe(2)
    expect(view.getFloat32(100, true)).toBe(3)
    expect(view.getFloat32(104, true)).toBe(4)
  })

  it('uses globally offset OBJ indices', () => {
    const mesh = triangleMesh()
    mesh.transform = identity()
    expect(buildObj([mesh, mesh])).toContain('f 4 5 6')
  })
})
