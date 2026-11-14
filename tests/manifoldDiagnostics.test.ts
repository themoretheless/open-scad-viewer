import { describe, expect, it } from 'vitest'
import { diagnoseManifoldInKernel } from '../src/kernel/geometry/manifoldAnalysis'

/** Outward axis-aligned box: 8 vertices, 12 triangles, vertex offset `base`. */
function box(lo: number[], hi: number[], base = 0): { positions: number[]; indices: number[] } {
  const positions: number[] = []
  for (let i = 0; i < 8; i++) {
    positions.push(i & 1 ? hi[0] : lo[0], i & 2 ? hi[1] : lo[1], i & 4 ? hi[2] : lo[2])
  }
  const faces = [
    0, 2, 3, 0, 3, 1, 4, 5, 7, 4, 7, 6, 0, 1, 5, 0, 5, 4, 2, 6, 7, 2, 7, 3, 0, 4, 6, 0, 6, 2, 1, 3, 7, 1, 7, 5,
  ]
  return { positions, indices: faces.map(v => v + base) }
}

function mesh(...parts: [number[], number[]][]) {
  const positions: number[] = []
  const indices: number[] = []
  for (const [lo, hi] of parts) {
    const b = box(lo, hi, positions.length / 3)
    positions.push(...b.positions)
    indices.push(...b.indices)
  }
  return { vertices: new Float64Array(positions), indices: new Uint32Array(indices) }
}

describe('diagnoseManifoldInKernel', () => {
  it('reports thickness, clearance and density-scaled mass of separate boxes', () => {
    const { vertices, indices } = mesh(
      [
        [0, 0, 0],
        [2, 4, 0.5],
      ],
      [
        [2.1, 0, 0],
        [3, 1, 1],
      ],
    )
    const r = diagnoseManifoldInKernel(vertices, indices, { density: 2, minThickness: 0.8, clearanceThreshold: 0.2 })
    expect(r.selfIntersectionPairs).toHaveLength(0)
    expect(r.selfIntersectionTruncated).toBe(false)
    expect(r.minThickness).toBeCloseTo(0.5, 9)
    expect(r.thinTriangles.length).toBe(4)
    expect(r.thicknessSamples).toBe(24)
    expect(r.minGap).toBeCloseTo(0.1, 9)
    expect(r.minGapTriangle).not.toBeNull()
    expect(r.componentCount).toBe(2)
    expect(r.clearancePairs).toHaveLength(1)
    expect(r.clearancePairs[0]).toMatchObject({ componentA: 0, componentB: 1 })
    expect(r.clearancePairs[0].distance).toBeCloseTo(0.1, 9)
    expect(r.mass?.volume).toBeCloseTo(4 + 0.9, 9)
    expect(r.mass?.mass).toBeCloseTo(2 * 4.9, 9)
    expect(r.mass?.inertia).toHaveLength(9)
  })

  it('finds crossing triangle pairs of overlapping shells', () => {
    const { vertices, indices } = mesh(
      [
        [0, 0, 0],
        [2, 2, 2],
      ],
      [
        [1, 1, 1],
        [3, 3, 3],
      ],
    )
    const r = diagnoseManifoldInKernel(vertices, indices)
    expect(r.selfIntersectionPairs.length).toBeGreaterThan(0)
    for (let p = 0; p < r.selfIntersectionPairs.length; p += 2) {
      expect(r.selfIntersectionPairs[p]).toBeLessThan(12)
      expect(r.selfIntersectionPairs[p + 1]).toBeGreaterThanOrEqual(12)
    }
    expect(r.clearancePairs[0].distance).toBe(0)
  })

  it('omits mass for an open mesh and rejects an exhausted budget with a typed error', () => {
    const open = diagnoseManifoldInKernel(new Float64Array([0, 0, 0, 1, 0, 0, 0, 1, 0]), new Uint32Array([0, 1, 2]))
    expect(open.mass).toBeNull()
    const { vertices, indices } = mesh(
      [
        [0, 0, 0],
        [2, 2, 2],
      ],
      [
        [1, 1, 1],
        [3, 3, 3],
      ],
    )
    expect(() => diagnoseManifoldInKernel(vertices, indices, { maxWork: 1 })).toThrow(
      expect.objectContaining({ code: 'GEOMETRY_RESOURCE_LIMIT' }),
    )
    expect(() => diagnoseManifoldInKernel(vertices, indices, { maxWork: 0 })).toThrow(RangeError)
  })
})
