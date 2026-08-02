import { describe, expect, it } from 'vitest'
import { assertGeometryScene, geometrySceneFromMeshes } from '../src/core/scene'
import { meshTransferables } from '../src/core/mesh'
import {
  binaryStlToMeshData,
  importedStlToMeshData,
  MAX_STL_IMPORT_TRIANGLES,
  parseBinaryStl,
  StlImportError,
} from '../src/services/stlImport'

type Triangle = readonly [
  number, number, number,
  number, number, number,
  number, number, number,
]

function binaryStl(triangles: readonly Triangle[], header = ''): ArrayBuffer {
  const buffer = new ArrayBuffer(84 + triangles.length * 50)
  const bytes = new Uint8Array(buffer)
  bytes.set(new TextEncoder().encode(header).subarray(0, 80))
  const view = new DataView(buffer)
  view.setUint32(80, triangles.length, true)
  triangles.forEach((triangle, triangleIndex) => {
    const record = 84 + triangleIndex * 50
    // Deliberately wrong stored normal: the importer must not trust it.
    view.setFloat32(record, 0, true)
    view.setFloat32(record + 4, 0, true)
    view.setFloat32(record + 8, -1, true)
    triangle.forEach((value, index) => view.setFloat32(record + 12 + index * 4, value, true))
    view.setUint16(record + 48, 0xbeef, true)
  })
  return buffer
}

const UNIT_TRIANGLE: Triangle = [0, 0, 0, 1, 0, 0, 0, 1, 0]

describe('binary STL import', () => {
  it('decodes little-endian records and ignores advisory normals and attributes', () => {
    const imported = parseBinaryStl(binaryStl([UNIT_TRIANGLE]))
    expect(imported.triangleCount).toBe(1)
    expect([...imported.positions]).toEqual(UNIT_TRIANGLE)

    const mesh = importedStlToMeshData(imported)
    expect([...mesh.vertices.slice(3, 6)]).toEqual([0, 0, 1])
    expect([...mesh.indices]).toEqual([0, 1, 2])
    expect([...mesh.faceIds]).toEqual([0])
    expect(mesh.provenance).toEqual([
      { triangleStart: 0, triangleEnd: 1, source: null, backside: false },
    ])
  })

  it('treats an exact binary layout as binary even when its header starts with solid', () => {
    expect(parseBinaryStl(binaryStl([UNIT_TRIANGLE], 'solid binary fixture')).triangleCount).toBe(1)
  })

  it.each([
    new Uint8Array([...new TextEncoder().encode('solid ascii\nendsolid')]).buffer,
    new Uint8Array([...new TextEncoder().encode('  SOLID ascii')]).buffer,
    new Uint8Array([0xef, 0xbb, 0xbf, ...new TextEncoder().encode('\nsolid ascii')]).buffer,
  ])('classifies text-shaped size mismatches as unsupported ASCII', buffer => {
    expect(() => parseBinaryStl(buffer)).toThrowError(StlImportError)
    try { parseBinaryStl(buffer) } catch (error) {
      expect((error as StlImportError).code).toBe('unsupported-ascii')
    }
  })

  it('accepts a structurally valid empty container but refuses to publish an empty mesh', () => {
    const imported = parseBinaryStl(binaryStl([]))
    expect(imported).toEqual({ triangleCount: 0, positions: new Float32Array() })
    expect(() => importedStlToMeshData(imported)).toThrowError(StlImportError)
  })

  it('rejects short, truncated, trailing, excessive and non-finite inputs with typed errors', () => {
    expect(() => parseBinaryStl(new ArrayBuffer(83))).toThrowError(StlImportError)

    const valid = binaryStl([UNIT_TRIANGLE])
    expect(() => parseBinaryStl(valid.slice(0, valid.byteLength - 1))).toThrowError(StlImportError)
    const trailing = new Uint8Array(valid.byteLength + 1)
    trailing.set(new Uint8Array(valid))
    expect(() => parseBinaryStl(trailing.buffer)).toThrowError(StlImportError)

    const excessive = new ArrayBuffer(84)
    new DataView(excessive).setUint32(80, MAX_STL_IMPORT_TRIANGLES + 1, true)
    try { parseBinaryStl(excessive) } catch (error) {
      expect((error as StlImportError).code).toBe('too-many-triangles')
    }

    const nonFinite = binaryStl([UNIT_TRIANGLE])
    new DataView(nonFinite).setFloat32(96, Number.NaN, true)
    try { parseBinaryStl(nonFinite) } catch (error) {
      expect((error as StlImportError).code).toBe('non-finite-coordinate')
    }
  })

  it('filters exact degenerates without deleting unitless micro-geometry', () => {
    const tiny = Math.fround(1e-20)
    const mesh = binaryStlToMeshData(binaryStl([
      [0, 0, 0, 0, 0, 0, 1, 0, 0],
      [0, 0, 0, tiny, 0, 0, 0, tiny, 0],
    ]))
    expect(mesh.indices.length).toBe(3)
    expect(mesh.topology.degenerate).toBe(1)
    expect(mesh.vertices.slice(3, 6).every(Number.isFinite)).toBe(true)
  })

  it('suppresses a shared coplanar diagonal and satisfies the complete scene contract', () => {
    const mesh = binaryStlToMeshData(binaryStl([
      UNIT_TRIANGLE,
      [1, 0, 0, 1, 1, 0, 0, 1, 0],
    ]))
    expect(mesh.edgeIndices.length).toBe(8)
    expect(mesh.topology).toMatchObject({ boundary: 4, crease: 0, nonManifold: 0 })
    expect(mesh.bvh.nodeCount).toBeGreaterThan(0)
    expect(mesh.entityId).toMatch(/^entity:import\/stl\//)
    expect(mesh.geometryAssetId).toMatch(/^asset:v1:/)

    const scene = geometrySceneFromMeshes([mesh])
    expect(() => assertGeometryScene(scene)).not.toThrow()
    expect(new Set(meshTransferables([mesh])).size).toBe(meshTransferables([mesh]).length)
  })

  it('rejects invalid adapter color without creating a protocol-invalid mesh', () => {
    expect(() => importedStlToMeshData(parseBinaryStl(binaryStl([UNIT_TRIANGLE])), [1, 1, 1, 2]))
      .toThrow(RangeError)
  })
})
