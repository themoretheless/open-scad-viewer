import { describe, expect, it } from 'vitest'
import { identity, translate } from '../src/services/math3d'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

interface MeasuredBounds {
  local: { min: number[]; max: number[] }
  world: { center: number[]; radius: number; min: number[]; max: number[] }
}

function boundMeasure(renderer: WebGPURenderer) {
  const internal = renderer as unknown as {
    measureMeshBounds: (assetId: string | null, vertices: Float32Array, transform: Float32Array) => MeasuredBounds | null
  }
  // The cache is keyed by content identity; the tests key it per buffer object.
  const ids = new WeakMap<Float32Array, string>()
  let next = 0
  return (vertices: Float32Array, transform: Float32Array) => {
    let id = ids.get(vertices)
    if (!id) {
      id = `test:${next++}`
      ids.set(vertices, id)
    }
    return internal.measureMeshBounds(id, vertices, transform)
  }
}

const vertices = new Float32Array([
  0, 0, 0, 0, 0, 1,
  1, 2, 3, 0, 0, 1,
  -1, 4, 2, 0, 0, 1,
])

describe('WebGPURenderer mesh bounds cache', () => {
  it('computes exact local and world bounds with a scalar scan', () => {
    const measure = boundMeasure(new WebGPURenderer())
    const result = measure(vertices, translate(identity(), [10, 20, 30]))

    expect(result).not.toBeNull()
    expect(result!.local).toEqual({ min: [-1, 0, 0], max: [1, 4, 3] })
    expect(result!.world.min).toEqual([9, 20, 30])
    expect(result!.world.max).toEqual([11, 24, 33])
    expect(result!.world.center).toEqual([10, 22, 31.5])
    expect(result!.world.radius).toBeCloseTo(Math.hypot(2, 4, 3) / 2, 12)
  })

  it('reuses the cached result for the same buffer and transform references', () => {
    const measure = boundMeasure(new WebGPURenderer())
    const transform = translate(identity(), [1, 1, 1])

    const first = measure(vertices, transform)
    const second = measure(vertices, transform)
    expect(second).toBe(first)
    expect(second!.world).toBe(first!.world)
  })

  it('recomputes when the transform reference changes', () => {
    const measure = boundMeasure(new WebGPURenderer())
    const first = measure(vertices, identity())
    expect(first!.world.min).toEqual([-1, 0, 0])

    const moved = measure(vertices, translate(identity(), [5, 0, 0]))
    expect(moved).not.toBe(first)
    expect(moved!.world.min).toEqual([4, 0, 0])
    expect(moved!.world.max).toEqual([6, 4, 3])
    // The local extents are transform-independent.
    expect(moved!.local).toEqual(first!.local)

    // A fresh but equal transform now hits the content-keyed cache and
    // returns the same result object.
    const again = measure(vertices, identity())
    expect(again).toBe(first)
    expect(again!.world).toEqual(first!.world)
  })

  it('retains separate bounds for instances sharing a geometry buffer', () => {
    const measure = boundMeasure(new WebGPURenderer())
    const left = translate(identity(), [-5, 0, 0])
    const right = translate(identity(), [5, 0, 0])
    const first = measure(vertices, left)
    const second = measure(vertices, right)
    expect(first!.world.min).toEqual([-6, 0, 0])
    expect(second!.world.min).toEqual([4, 0, 0])
    expect(measure(vertices, left)).toBe(first)
    expect(measure(vertices, right)).toBe(second)
  })

  it('invalidates an instance when its transform changes in place', () => {
    const measure = boundMeasure(new WebGPURenderer())
    const transform = identity()
    const first = measure(vertices, transform)
    transform[3] = 10
    const moved = measure(vertices, transform)
    expect(moved).not.toBe(first)
    expect(moved!.world.min).toEqual([9, 0, 0])
    expect(first!.world.min).toEqual([-1, 0, 0])
    expect(measure(vertices, transform)).toBe(moved)
  })

  it('skips non-finite vertices and reports fully invalid buffers as null', () => {
    const measure = boundMeasure(new WebGPURenderer())
    const withNaN = new Float32Array([
      NaN, 0, 0, 0, 0, 1,
      1, 1, 1, 0, 0, 1,
    ])
    expect(measure(withNaN, identity())!.local).toEqual({ min: [1, 1, 1], max: [1, 1, 1] })

    const allBad = new Float32Array([NaN, NaN, NaN, 0, 0, 1])
    expect(measure(allBad, identity())).toBeNull()
  })
})
