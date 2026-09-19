import { expect, it, vi } from 'vitest'
import { BrepInspectionCache } from '../src/services/brepInspectionCache'
import { createBrepBox, inspectNurbsBrep } from '../src/services/geometry/brep'

const box = (size: number) => createBrepBox([0, 0, 0], [size, size, size])

it('hits cloned content but inspects changes and never caches a failed inspection', () => {
  const inspect = vi.fn(inspectNurbsBrep)
  const cache = new BrepInspectionCache(undefined, inspect)
  const model = box(1)
  cache.inspect(model)
  cache.inspect(structuredClone(model))
  expect(inspect).toHaveBeenCalledTimes(1)
  const retained = cache.retainedCharacters
  model.edges[0]!.vertices[0] = 99999
  for (let i = 0; i < 2; i++) expect(() => cache.inspect(model)).toThrow()
  expect(inspect).toHaveBeenCalledTimes(3)
  expect(cache.size).toBe(1)
  expect(cache.retainedCharacters).toBe(retained)
})

it('bounds retained characters and re-inspects evicted keys in FIFO order', () => {
  const models = [box(1), box(2), box(3)]
  const sizes = models.map(model => JSON.stringify(model).length)
  const budget = sizes[0]! + sizes[1]!
  const inspect = vi.fn(inspectNurbsBrep)
  const cache = new BrepInspectionCache({ maxEntries: 128, maxCharacters: budget }, inspect)
  cache.inspect(models[0]!)
  cache.inspect(models[1]!)
  expect(cache.retainedCharacters).toBe(budget)
  cache.inspect(models[0]!) // A hit preserves the pre-existing FIFO policy.
  cache.inspect(models[2]!)
  expect(cache.retainedCharacters).toBeLessThanOrEqual(budget)
  expect(cache.retainedCharacters).toBe(sizes[1]! + sizes[2]!)
  cache.inspect(models[1]!)
  expect(inspect).toHaveBeenCalledTimes(3)
  cache.inspect(models[0]!)
  expect(inspect).toHaveBeenCalledTimes(4)
})

it('retains the entry-count bound independently of character capacity', () => {
  const inspect = vi.fn(inspectNurbsBrep)
  const cache = new BrepInspectionCache({ maxEntries: 1, maxCharacters: 1_000_000 }, inspect)
  const first = box(1), second = box(2)
  cache.inspect(first)
  cache.inspect(second)
  expect(cache.size).toBe(1)
  expect(cache.retainedCharacters).toBe(JSON.stringify(second).length)
  cache.inspect(first)
  expect(inspect).toHaveBeenCalledTimes(3)
})

it('validates oversized inputs every time without flushing existing entries', () => {
  const small = box(1), large = box(12345)
  const budget = JSON.stringify(small).length
  expect(JSON.stringify(large).length).toBeGreaterThan(budget)
  const inspect = vi.fn(inspectNurbsBrep)
  const cache = new BrepInspectionCache({ maxEntries: 2, maxCharacters: budget }, inspect)
  cache.inspect(small)
  cache.inspect(large)
  cache.inspect(large)
  cache.inspect(small)
  expect(inspect).toHaveBeenCalledTimes(3)
  expect(cache.size).toBe(1)
  expect(cache.retainedCharacters).toBe(budget)
})

it.each([0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1])('rejects invalid capacity %s', value => {
  expect(() => new BrepInspectionCache({ maxEntries: value, maxCharacters: 100 })).toThrow(RangeError)
  expect(() => new BrepInspectionCache({ maxEntries: 1, maxCharacters: value })).toThrow(RangeError)
})
