import { describe, expect, it } from 'vitest'
import { GpuResidencyCache } from '../src/render/gpuResidencyCache'

const resource = (log: string[], key: string) => ({ destroy: () => log.push(key) })

describe('GpuResidencyCache budget', () => {
  it('evicts unpinned entries at once when the budget shrinks', () => {
    const log: string[] = []
    const cache = new GpuResidencyCache(300)
    cache.pin(key => key === 'b')
    for (const key of ['a', 'b', 'c']) cache.lru.set(key, resource(log, key), 100)
    cache.setBudget(100)
    expect(cache.budgetBytes).toBe(100)
    expect(log).toEqual(['a', 'c'])
    expect(cache.lru.keys()).toEqual(['b'])
  })

  it('keeps more entries after the budget grows', () => {
    const log: string[] = []
    const cache = new GpuResidencyCache(100)
    cache.setBudget(250)
    for (const key of ['a', 'b']) cache.lru.set(key, resource(log, key), 100)
    expect(log).toEqual([])
    expect(cache.bytes).toBe(200)
  })
})
