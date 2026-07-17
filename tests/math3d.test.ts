import { describe, expect, it } from 'vitest'
import {
  identity,
  invert,
  lookAt,
  orthographic,
  perspective,
  rayAabbDistance,
  rayIndexedMeshDistance,
  rayTriangleDistance,
  translate,
  unprojectRay,
} from '../src/services/math3d'

function project(m: Float32Array, x: number, y: number, z: number) {
  const w = m[12]*x + m[13]*y + m[14]*z + m[15]
  return [
    (m[0]*x + m[1]*y + m[2]*z + m[3]) / w,
    (m[4]*x + m[5]*y + m[6]*z + m[7]) / w,
    (m[8]*x + m[9]*y + m[10]*z + m[11]) / w,
  ]
}

describe('WebGPU projection matrices', () => {
  it('maps perspective near/far planes to the WebGPU 0..1 depth range', () => {
    const m = perspective(Math.PI / 2, 1, 0.1, 100)
    expect(project(m, 0, 0, -0.1)[2]).toBeCloseTo(0, 6)
    expect(project(m, 0, 0, -100)[2]).toBeCloseTo(1, 6)
  })

  it('maps orthographic bounds and depth planes to normalized coordinates', () => {
    const m = orthographic(-2, 6, -4, 2, 0.5, 50)
    expect(project(m, -2, -4, -0.5)[0]).toBeCloseTo(-1, 6)
    expect(project(m, -2, -4, -0.5)[1]).toBeCloseTo(-1, 6)
    expect(project(m, 6, 2, -50)[0]).toBeCloseTo(1, 6)
    expect(project(m, 6, 2, -50)[1]).toBeCloseTo(1, 6)
    expect(project(m, 0, 0, -0.5)[2]).toBeCloseTo(0, 6)
    expect(project(m, 0, 0, -50)[2]).toBeCloseTo(1, 6)
  })
})

describe('lookAt', () => {
  it('stays finite for a Z-up top view', () => {
    const m = lookAt([0, 0, 10], [0, 0, 0], [0, 0, 1])
    expect([...m].every(Number.isFinite)).toBe(true)
    expect(project(m, 0, 0, 0)[2]).toBeCloseTo(-10, 6)
  })
})

describe('viewport picking math', () => {
  it('unprojects the center of a perspective viewport into a normalized ray', () => {
    const ray = unprojectRay(invert(perspective(Math.PI / 2, 1, 1, 10)), 0, 0)
    expect(ray).not.toBeNull()
    expect(ray!.origin[0]).toBeCloseTo(0, 6)
    expect(ray!.origin[1]).toBeCloseTo(0, 6)
    expect(ray!.origin[2]).toBeCloseTo(-1, 6)
    expect(ray!.direction).toEqual([0, 0, -1])
  })

  it('rejects a parallel AABB miss and returns the entry distance on a hit', () => {
    const bounds = { min: [-1, -1, -1] as [number, number, number], max: [1, 1, 1] as [number, number, number] }
    expect(rayAabbDistance({ origin: [2, 0, 0], direction: [0, 0, -1] }, bounds)).toBeNull()
    expect(rayAabbDistance({ origin: [0, 0, 5], direction: [0, 0, -1] }, bounds)).toBeCloseTo(4, 6)
  })

  it('intersects triangles from either side and rejects points outside them', () => {
    const a: [number, number, number] = [-1, -1, 0]
    const b: [number, number, number] = [1, -1, 0]
    const c: [number, number, number] = [0, 1, 0]
    expect(rayTriangleDistance({ origin: [0, 0, 2], direction: [0, 0, -1] }, a, b, c)).toBeCloseTo(2, 6)
    expect(rayTriangleDistance({ origin: [0, 0,-2], direction: [0, 0, 1] }, a, b, c)).toBeCloseTo(2, 6)
    expect(rayTriangleDistance({ origin: [2, 2, 2], direction: [0, 0,-1] }, a, b, c)).toBeNull()
  })

  it('picks an indexed mesh through its inverse object transform', () => {
    const vertices = new Float32Array([
      -1,-1,0, 0,0,1,
       1,-1,0, 0,0,1,
       0, 1,0, 0,0,1,
    ])
    const indices = new Uint32Array([0, 1, 2])
    const model = translate(identity(), [0, 0, -3])
    const distance = rayIndexedMeshDistance(
      { origin: [0, 0, 2], direction: [0, 0, -1] },
      vertices,
      indices,
      invert(model),
      { min: [-1,-1,0], max: [1,1,0] },
    )
    expect(distance).toBeCloseTo(5, 6)
    expect(rayIndexedMeshDistance(
      { origin: [0, 0, 2], direction: [0, 0, -1] },
      vertices,
      indices,
      invert(model),
      { min: [-1,-1,0], max: [1,1,0] },
      4,
    )).toBeNull()
  })
})
