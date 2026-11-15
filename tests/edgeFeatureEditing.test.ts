import { describe, expect, it } from 'vitest'
import { dragDimensionValue, recallEdgeFeature, rememberEdgeFeature } from '../src/app/edgeFeatureEditing'

describe('radius by mouse', () => {
  it('maps pixels to millimetres at the view scale, snaps and clamps', () => {
    expect(dragDimensionValue(2, 40, 10)).toBe(6)
    expect(dragDimensionValue(2, 13, 10, 0.5)).toBe(3.5)
    expect(dragDimensionValue(2, -500, 10)).toBe(0.01)
    expect(dragDimensionValue(2, -50, 10, 0, -Infinity)).toBe(-3)
    expect(dragDimensionValue(2, 10, 0)).toBe(2)
  })
})

describe('edge selection kept across undo', () => {
  const body = { id: 'b', brep: { faces: [1, 2] } }
  it('restores the consumed chain when the body returns to its pre-feature state', () => {
    const memory = rememberEdgeFeature(body, [4, 2, 4, -1])
    expect(memory?.edges).toEqual([2, 4])
    // After apply the body changed: keep waiting.
    expect(recallEdgeFeature(memory, [{ id: 'b', brep: { faces: [9] } }])).toEqual({ edges: null, keep: true })
    // Undo: same B-rep again.
    expect(recallEdgeFeature(memory, [structuredClone(body)])).toEqual({ edges: [2, 4], keep: false })
    // Body deleted: forget.
    expect(recallEdgeFeature(memory, [])).toEqual({ edges: null, keep: false })
  })
  it('ignores mesh bodies and empty selections', () => {
    expect(rememberEdgeFeature({ id: 'm' }, [1])).toBeNull()
    expect(rememberEdgeFeature(body, [])).toBeNull()
  })
})
