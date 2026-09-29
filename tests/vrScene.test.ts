import { describe, expect, it } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { prepareVrScene, prepareVrPolygons } from '../src/services/vrScene'

function mesh(offset = 0): MeshData {
  return {
    vertices: new Float32Array([0,0,0,0,0,1, 100,0,0,0,0,1, 0,0,100,0,0,1]),
    indices: new Uint32Array([0,1,2]),
    transform: new Float32Array([1,0,0,offset, 0,1,0,0, 0,0,1,0, 0,0,0,1]),
    color: [0.4,0.5,0.6,1],
  } as MeshData
}
describe('VR scene snapshot', () => {
  it('preserves transformed instance spacing, fits 60 cm, and maps Z up to Y up', () => {
    const source = [mesh(), mesh(100)]
    const result = prepareVrScene(source, [], false, null)
    expect(result[0].positions[0]).toBeCloseTo(-0.3)
    expect(result[1].positions[3]).toBeCloseTo(0.3)
    expect(result[0].positions[7]).toBeCloseTo(0.15)
    expect(result[0].positions[1]).toBeCloseTo(-0.15)
    expect(source[0].vertices[3]).toBe(0)
    expect(result[0].indices).not.toBe(source[0].indices)
  })
  it('excludes hidden and non-isolated objects from geometry and bounds', () => {
    expect(prepareVrScene([mesh(), mesh(10000)], [true, false], false, null)).toHaveLength(1)
    const result = prepareVrScene([mesh(), mesh(10000)], [], true, 1)
    expect(result).toHaveLength(1)
    expect(result[0].positions[0]).toBeCloseTo(-0.3)
  })
  it('adapts polygon workspace publications without changing their coordinates', () => {
    const positions = [0,0,0, 100,0,0, 0,0,100]
    const result = prepareVrPolygons([{ positions, indices: [0,1,2], color: [1,0,0] }])
    expect(result[0].positions[7]).toBeCloseTo(0.3)
    expect(result[0].color).toEqual([1,0,0,1])
    expect(positions[8]).toBe(100)
  })
  it('refuses an empty visible scene and invalid geometry', () => {
    expect(() => prepareVrScene([mesh()], [false], false, null)).toThrow('No visible')
    const bad = mesh(); bad.vertices[0] = NaN
    expect(() => prepareVrScene([bad], [], false, null)).toThrow('Invalid')
  })
})


describe('Rust VR WASM boundary', () => {
  it('rejects invalid indices and truncated triangles without poisoning later calls', () => {
    const bad = mesh(); bad.indices[0] = 0xffffffff
    expect(() => prepareVrScene([bad], [], false, null)).toThrow('Invalid')
    bad.indices = new Uint32Array([0, 1])
    expect(() => prepareVrScene([bad], [], false, null)).toThrow('Invalid')
    expect(prepareVrScene([mesh()], [], false, null)[0].positions[0]).toBeCloseTo(-0.3)
  })
  it('survives memory growth and returns owned snapshots', () => {
    const before = prepareVrScene([mesh()], [], false, null)[0]
    const large = mesh(); large.vertices = new Float32Array(600_000)
    large.vertices[6] = 100; large.vertices[14] = 100
    for (let i = 0; i < 3; i++) {
      const result = prepareVrScene([large], [], false, null)[0]
      expect(result.positions.length).toBe(300_000)
      expect(result.positions[3]).toBeCloseTo(0.3)
    }
    expect(before.positions[0]).toBeCloseTo(-0.3)
  })
})
