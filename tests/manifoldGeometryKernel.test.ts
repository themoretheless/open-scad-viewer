import type { ManifoldToplevel } from 'manifold-3d/manifold'
import { describe, expect, it, vi } from 'vitest'
import { ManifoldGeometryKernel } from '../src/services/manifoldGeometryKernel'

describe('ManifoldGeometryKernel lifecycle', () => {
  it('warms once and disposes each serialized lease exactly once', async () => {
    const module = {} as ManifoldToplevel
    const load = vi.fn(async () => module)
    const instrument = vi.fn((value: ManifoldToplevel) => value)
    const cleanup = vi.fn()
    const kernel = new ManifoldGeometryKernel({ load, instrument, cleanup })

    const [first, second] = await Promise.all([kernel.openSession(), kernel.openSession()])
    expect(first.module).toBe(module)
    expect(second.module).toBe(module)
    expect(load).toHaveBeenCalledTimes(1)
    expect(instrument).toHaveBeenCalledTimes(1)
    first.dispose()
    first.dispose()
    second.dispose()
    expect(cleanup).toHaveBeenCalledTimes(2)
  })

  it('does not cache a rejected warm-up attempt', async () => {
    const module = {} as ManifoldToplevel
    const load = vi.fn()
      .mockRejectedValueOnce(new Error('temporary load failure'))
      .mockResolvedValueOnce(module)
    const kernel = new ManifoldGeometryKernel({ load, instrument: value => value, cleanup: vi.fn() })

    await expect(kernel.warm()).rejects.toThrow('temporary load failure')
    await expect(kernel.warm()).resolves.toBe(module)
    expect(load).toHaveBeenCalledTimes(2)
  })
})
