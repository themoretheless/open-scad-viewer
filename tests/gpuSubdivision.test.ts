import { describe, it, expect } from 'vitest'
import {
  referenceRefineSubdivision,
  runGpuSubdivisionRefine,
  referenceBrushSubdivision,
  runGpuBrushSubdivision,
  CATMULL_CLARK_WGSL,
} from '../src/services/gpuSubdivision'
import type { SubdivisionCage } from '../src/services/geometry/subdivision'

function createUnitCubeCage(): SubdivisionCage {
  return {
    vertices: [
      [0, 0, 0], // 0
      [2, 0, 0], // 1
      [2, 2, 0], // 2
      [0, 2, 0], // 3
      [0, 0, 2], // 4
      [2, 0, 2], // 5
      [2, 2, 2], // 6
      [0, 2, 2], // 7
    ],
    faces: [
      [3, 2, 1, 0], // bottom
      [4, 5, 6, 7], // top
      [0, 1, 5, 4], // front
      [1, 2, 6, 5], // right
      [2, 3, 7, 6], // back
      [3, 0, 4, 7], // left
    ],
  }
}

describe('WebGPU Compute Catmull–Clark Subdivision & Brush Sculpting', () => {
  it('applies exact boundary rule (6P + B0 + B1)/8 and preserves face identity on open quad', () => {
    const square: SubdivisionCage = {
      vertices: [
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 0],
        [0, 1, 0],
      ],
      faces: [[0, 1, 2, 3]],
    }

    const r1 = referenceRefineSubdivision(square, 1)
    expect(r1.cage.faces.length).toBe(4)
    expect(r1.cage.vertices[0]).toEqual([0.125, 0.125, 0])

    const r2 = referenceRefineSubdivision(square, 2)
    expect(r2.cage.faces.length).toBe(16)
    expect(r2.faceIds.every((id) => id === 0)).toBe(true)
  })

  it('refines a closed manifold cube cage via runGpuSubdivisionRefine', async () => {
    const cube = createUnitCubeCage()
    const refined = await runGpuSubdivisionRefine(cube, 2)

    // 6 original quad faces * 4^2 = 96 quad faces
    expect(refined.cage.faces.length).toBe(96)
    expect(refined.faceIds.length).toBe(96)
    // Euler characteristic for closed quad mesh: V - E + F = 2 => V = F + 2 = 98
    expect(refined.cage.vertices.length).toBe(98)

    // Original corner [0, 0, 0] after 1 step of Catmull-Clark on [0,2]^3 cube:
    // F = (1,1,0 + 1,0,1 + 0,1,1)/3 = (2/3, 2/3, 2/3)
    // 2R = 2 * (1,0,0 + 0,1,0 + 0,0,1)/3 = (2/3, 2/3, 2/3)
    // (n-3)P = 0
    // Q = (F + 2R)/3 = (4/9, 4/9, 4/9)
    const step1 = referenceRefineSubdivision(cube, 1)
    expect(step1.cage.vertices[0]![0]).toBeCloseTo(4 / 9, 6)
    expect(step1.cage.vertices[0]![1]).toBeCloseTo(4 / 9, 6)
    expect(step1.cage.vertices[0]![2]).toBeCloseTo(4 / 9, 6)
  })

  it('displaces vertices locally with smoothstep falloff via runGpuBrushSubdivision', async () => {
    const cube = createUnitCubeCage()
    const brushed = await runGpuBrushSubdivision(cube, {
      center: [2, 2, 2],
      radius: 0.5,
      displacement: [0, 0, 1],
    })

    expect(brushed.faces).toEqual(cube.faces)
    expect(brushed.vertices[6]).toEqual([2, 2, 3])
    expect(brushed.vertices[0]).toEqual([0, 0, 0])
  })

  it('declares valid WGSL compute shader with face, edge, vertex, and brush modes', () => {
    expect(CATMULL_CLARK_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(CATMULL_CLARK_WGSL).toContain('eval_falloff')
    expect(CATMULL_CLARK_WGSL).toContain('params.mode == 2u')
  })
})
