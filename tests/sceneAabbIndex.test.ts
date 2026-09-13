import { afterEach, describe, expect, it } from 'vitest'
import { buildSceneAabbIndex as buildNativeIndex, disposeSceneAabbIndex, querySceneAabbIndex } from '../src/services/sceneAabbIndex'
import { rayAabbDistance } from '../src/services/math3d'

const indexes: ReturnType<typeof buildNativeIndex>[] = []
const buildSceneAabbIndex: typeof buildNativeIndex = (...args) => {
  const index = buildNativeIndex(...args)
  indexes.push(index)
  return index
}
afterEach(() => { for (const index of indexes.splice(0)) disposeSceneAabbIndex(index) })

it('matches independent brute-force slab queries across a multi-level scene', () => {
  let seed = 913
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296 }
  const items = Array.from({ length: 257 }, (_, id) => {
    const min: [number, number, number] = [random() * 20 - 10, random() * 20 - 10, random() * 20 - 10]
    return { id, bounds: { min, max: min.map(x => x + random() * 4) as [number, number, number] } }
  })
  const index = buildSceneAabbIndex(items, 3)
  let hits = 0
  for (let n = 0; n < 500; n++) {
    const ray = { origin: [random() * 30 - 15, random() * 30 - 15, 15] as [number, number, number], direction: [random() - 0.5, random() - 0.5, -1] as [number, number, number] }
    const maximum = random() * 40
    const expected = items.flatMap(item => {
      const distance = rayAabbDistance(ray, item.bounds, maximum)
      return distance === null ? [] : [{ id: item.id, distance }]
    }).sort((a, b) => a.distance - b.distance || a.id - b.id)
    expect(querySceneAabbIndex(index, ray, maximum)).toEqual(expected)
    hits += expected.length
  }
  expect(hits).toBeGreaterThan(100)
})

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

it('retains tiny nonzero ray directions and rejects disposed indexes', () => {
  const index = buildSceneAabbIndex([{ id: 7, bounds: bounds([1, -1, -1], [2, 1, 1]) }])
  const ray = { origin: [0, 0, 0] as [number, number, number], direction: [1e-14, 0, 0] as [number, number, number] }
  expect(querySceneAabbIndex(index, ray, 1e14)).toEqual([{ id: 7, distance: 1e14 }])
  expect(querySceneAabbIndex(index, ray, 9e13)).toEqual([])
  disposeSceneAabbIndex(index)
  expect(() => querySceneAabbIndex(index, ray)).toThrow('disposed')
  expect(() => disposeSceneAabbIndex(index)).not.toThrow()
})

it('handles finite extreme bounds without overflowing the ray parameter', () => {
  const index = buildSceneAabbIndex([{ id: 1, bounds: bounds([1e308, -1, -1], [1.5e308, 1, 1]) }])
  expect(querySceneAabbIndex(index, { origin: [-1e308, 0, 0], direction: [1e308, 0, 0] }, 3))
    .toEqual([{ id: 1, distance: 2 }])
  expect(querySceneAabbIndex(index, { origin: [1e308, 0, 0], direction: [0, 0, 0] })).toEqual([])
})
