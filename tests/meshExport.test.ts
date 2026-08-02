import { describe, expect, it } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { buildBinaryStl, buildObj } from '../src/services/meshExport'
import { identity, scale, translate } from '../src/services/math3d'
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

  it('truncates multibyte STL header names by bytes, not characters', () => {
    // 80 Cyrillic chars encode to 160 UTF-8 bytes; must not overflow the
    // 80-byte header slot (previously a RangeError for small scenes).
    const data = buildBinaryStl([], '\u044f'.repeat(80))
    expect(data.byteLength).toBe(84)
  })

  it('reverses mirrored winding consistently in STL and OBJ', () => {
    const mesh = triangleMesh()
    mesh.transform = scale(identity(), [-1, 1, 1])
    const stl = buildBinaryStl([mesh])
    const view = new DataView(stl.buffer)
    expect(view.getFloat32(92, true)).toBeCloseTo(1)
    expect(buildObj([mesh])).toContain('f 1 3 2')
  })

  it('omits degenerate triangles and rejects malformed mesh structure', () => {
    const mesh = triangleMesh()
    mesh.transform = identity()
    mesh.indices = new Uint32Array([0, 0, 1])
    expect(new DataView(buildBinaryStl([mesh]).buffer).getUint32(80, true)).toBe(0)
    expect(buildObj([mesh])).not.toContain('\nf ')

    mesh.indices = new Uint32Array([0, 1, 99])
    expect(() => buildBinaryStl([mesh])).toThrow(/out-of-range/)
    mesh.indices = new Uint32Array([0, 1])
    expect(() => buildObj([mesh])).toThrow(/finite affine triangle data/)

    const nonFinite = triangleMesh()
    nonFinite.transform = identity()
    nonFinite.vertices[0] = Number.NaN
    expect(() => buildBinaryStl([nonFinite])).toThrow(/finite affine triangle data/)
    expect(() => buildObj([nonFinite])).toThrow(/finite affine triangle data/)
  })

  it('preserves micro-geometry and omits unreferenced OBJ vertices', () => {
    const mesh = triangleMesh()
    mesh.transform = identity()
    mesh.vertices = new Float32Array([
      0, 0, 0, 0, 0, 1,
      1e-11, 0, 0, 0, 0, 1,
      0, 1e-11, 0, 0, 0, 1,
      999, 999, 999, 0, 0, 1,
    ])
    expect(new DataView(buildBinaryStl([mesh]).buffer).getUint32(80, true)).toBe(1)
    const obj = buildObj([mesh])
    expect(obj.match(/^v /gm)).toHaveLength(3)
    expect(obj).not.toContain('v 999 999 999')
  })
})
