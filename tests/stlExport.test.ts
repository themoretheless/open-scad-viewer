import { describe, it, expect } from 'vitest'
import { buildSTLBuffer } from '../src/services/stlExport'
import { parseOpenSCAD, type MeshData } from '../src/services/openscadParser'

function totalTriangles(meshes: MeshData[]): number {
  return meshes.reduce((acc, m) => acc + m.indices.length / 3, 0)
}

describe('binary STL export', () => {
  it('exports a cube to a buffer of the expected size', () => {
    const meshes = parseOpenSCAD('cube([10,10,10]);')
    const buffer = buildSTLBuffer(meshes)
    expect(buffer).toBeInstanceOf(ArrayBuffer)

    const tris = totalTriangles(meshes)
    // Binary STL: 80-byte header + 4-byte count + 50 bytes per triangle.
    const expectedSize = 84 + tris * 50
    expect(buffer.byteLength).toBe(expectedSize)
  })

  it('writes a triangle count in the header that matches the mesh triangle count', () => {
    const meshes = parseOpenSCAD('cube([10,10,10]);')
    const buffer = buildSTLBuffer(meshes)
    const view = new DataView(buffer)
    // Triangle count is a little-endian uint32 at byte offset 80.
    const headerCount = view.getUint32(80, true)
    expect(headerCount).toBe(totalTriangles(meshes))
  })

  it('handles multiple meshes (sums triangles across them)', () => {
    const meshes = parseOpenSCAD('cube(4); translate([10,0,0]) sphere(3);')
    expect(meshes.length).toBeGreaterThanOrEqual(2)
    const buffer = buildSTLBuffer(meshes)
    const view = new DataView(buffer)
    const headerCount = view.getUint32(80, true)
    expect(headerCount).toBe(totalTriangles(meshes))
    expect(buffer.byteLength).toBe(84 + totalTriangles(meshes) * 50)
  })

  it('produces an 84-byte buffer for an empty mesh list', () => {
    const buffer = buildSTLBuffer([])
    expect(buffer.byteLength).toBe(84)
    const view = new DataView(buffer)
    expect(view.getUint32(80, true)).toBe(0)
  })
})
