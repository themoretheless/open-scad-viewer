import { describe, expect, it } from 'vitest'
import { buildSceneAabbIndex, querySceneAabbIndex } from '../src/services/sceneAabbIndex'

const bounds = (
  min: [number, number, number],
  max: [number, number, number],
) => ({ min, max })

describe('scene AABB index', () => {
  it('rejects whole branches and returns intersected object bounds front-to-back', () => {
    const index = buildSceneAabbIndex([
      { id: 12, bounds: bounds([-1, -1, -8], [1, 1, -7]) },
      { id: 4, bounds: bounds([-1, -1, -3], [1, 1, -2]) },
      { id: 99, bounds: bounds([20, 20, -5], [21, 21, -4]) },
      { id: 7, bounds: bounds([-1, -1, -5], [1, 1, -4]) },
    ], 1)

    const hits = querySceneAabbIndex(index, {
      origin: [0, 0, 0],
      direction: [0, 0, -1],
    })

    expect(index.itemCount).toBe(4)
    expect(index.nodeCount).toBe(7)
    expect(hits.map(hit => hit.id)).toEqual([4, 7, 12])
    expect(hits.map(hit => hit.distance)).toEqual([2, 4, 7])
  })

  it('is deterministic for ties, supports rays starting inside, and honours range', () => {
    const index = buildSceneAabbIndex([
      { id: 8, bounds: bounds([-2, -2, -2], [2, 2, 2]) },
      { id: 3, bounds: bounds([-1, -1, -5], [1, 1, -4]) },
      { id: 1, bounds: bounds([-1, -1, -5], [1, 1, -4]) },
    ])
    const ray = { origin: [0, 0, 0] as [number, number, number], direction: [0, 0, -1] as [number, number, number] }

    expect(querySceneAabbIndex(index, ray, 4).map(hit => [hit.id, hit.distance])).toEqual([
      [8, 0], [1, 4], [3, 4],
    ])
    expect(querySceneAabbIndex(index, ray, 3.9).map(hit => hit.id)).toEqual([8])
  })

  it('copies valid inputs and ignores malformed entries', () => {
    const mutable = bounds([0, 0, 0], [1, 1, 1])
    const index = buildSceneAabbIndex([
      { id: 2, bounds: mutable },
      { id: 3, bounds: bounds([2, 0, 0], [1, 1, 1]) },
      { id: Number.NaN, bounds: bounds([0, 0, 0], [1, 1, 1]) },
    ])
    mutable.min[0] = 100

    expect(index.itemCount).toBe(1)
    expect(querySceneAabbIndex(index, {
      origin: [-1, 0.5, 0.5],
      direction: [1, 0, 0],
    }).map(hit => hit.id)).toEqual([2])
  })
})
