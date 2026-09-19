import { expect, it, vi } from 'vitest'
import { SurfaceGroupContentCache } from '../src/services/surfaceGroupContentCache'

it('reuses IDs and evicts oldest entries at the count limit', () => {
  const cache = new SurfaceGroupContentCache({ maxEntries: 2, maxBytes: 100 })
  const compute = vi.fn(() => new Uint32Array([1]))
  const first = cache.getOrCompute('a', compute)
  cache.getOrCompute('b', compute)
  expect(cache.getOrCompute('a', compute)).toBe(first)
  cache.getOrCompute('c', compute)
  expect(cache.getOrCompute('a', compute)).not.toBe(first)
  expect(compute).toHaveBeenCalledTimes(4)
  expect(cache.size).toBe(2)
  expect(cache.retainedBytes).toBe(12)
})

it('enforces byte limits before count limits and counts the full backing buffer', () => {
  const cache = new SurfaceGroupContentCache({ maxEntries: 100, maxBytes: 20 })
  const ids = new Uint32Array(4)
  cache.getOrCompute('a', () => ids.subarray(0, 1))
  expect(cache.retainedBytes).toBe(18)
  cache.getOrCompute('b', () => new Uint32Array([2]))
  expect(cache.size).toBe(1)
  expect(cache.retainedBytes).toBe(6)
})

it('oversized values and failures do not evict the working set', () => {
  const cache = new SurfaceGroupContentCache({ maxEntries: 1, maxBytes: 8 })
  const ids = cache.getOrCompute('a', () => new Uint32Array([1]))
  expect(cache.getOrCompute('large', () => new Uint32Array(100)).length).toBe(100)
  expect(() => cache.getOrCompute('bad', () => { throw Error('invalid mesh') })).toThrow('invalid mesh')
  expect(cache.getOrCompute('a', () => { throw Error('cache miss') })).toBe(ids)
  expect(cache.retainedBytes).toBe(6)
})

it('bounds retention across many unique publications without changing results', () => {
  const cache = new SurfaceGroupContentCache({ maxEntries: 4, maxBytes: 100 })
  for (let i = 0; i < 1000; i++) {
    expect(cache.getOrCompute(`asset:${i}`, () => new Uint32Array([i]))[0]).toBe(i)
    expect(cache.size).toBeLessThanOrEqual(4)
    expect(cache.retainedBytes).toBeLessThanOrEqual(100)
  }
})

it('rejects invalid limits', () => {
  for (const value of [0, -1, Infinity, NaN, 1.5]) {
    expect(() => new SurfaceGroupContentCache({ maxEntries: value, maxBytes: 10 })).toThrow(RangeError)
    expect(() => new SurfaceGroupContentCache({ maxEntries: 10, maxBytes: value })).toThrow(RangeError)
  }
})
