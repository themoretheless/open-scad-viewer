import { describe, expect, it, vi } from 'vitest'
import { computeSurfaceSize, createDepthTarget } from '../src/render/surfaceResources'

describe('computeSurfaceSize', () => {
  it('returns null for an empty or non-finite canvas', () => {
    expect(computeSurfaceSize(0, 100, 1, 8192)).toBeNull()
    expect(computeSurfaceSize(100, Number.NaN, 1, 8192)).toBeNull()
    expect(computeSurfaceSize(-5, 10, 1, 8192)).toBeNull()
  })

  it('clamps DPR to [1, MAX_DPR]', () => {
    expect(computeSurfaceSize(100, 50, 0.5, 8192)).toEqual({ width: 100, height: 50 })
    expect(computeSurfaceSize(100, 50, 3, 8192)).toEqual({ width: 200, height: 100 })
    expect(computeSurfaceSize(100, 50, Number.NaN, 8192)).toEqual({ width: 100, height: 50 })
  })

  it('keeps the aspect within the texture dimension limit', () => {
    expect(computeSurfaceSize(4000, 1000, 2, 4096)).toEqual({ width: 4096, height: 1024 })
  })
})

describe('createDepthTarget', () => {
  it('allocates a depth24plus attachment of the surface size', () => {
    vi.stubGlobal('GPUTextureUsage', { RENDER_ATTACHMENT: 16 })
    const view = {}
    const texture = { createView: vi.fn(() => view) }
    const device = { createTexture: vi.fn(() => texture) } as unknown as GPUDevice
    const target = createDepthTarget(device, { width: 30, height: 20 })
    expect(device.createTexture).toHaveBeenCalledWith({ size: [30, 20], format: 'depth24plus', usage: 16 })
    expect(target).toEqual({ texture, view })
    vi.unstubAllGlobals()
  })
})
