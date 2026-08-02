import { describe, expect, it } from 'vitest'
import { backendQuality, isTransparentAlpha, usesTransparentPass } from '../src/services/backendQuality'

describe('rendering backend quality tiers', () => {
  it('reports the exact headless fallback without claiming a viewport', () => {
    expect(backendQuality(false, 'xray')).toEqual({
      backend: 'headless-geometry',
      geometryBuilds: 'full',
      interactiveViewport: false,
      transparency: 'unavailable',
    })
  })

  it('labels x-ray as object-sorted alpha rather than order-independent', () => {
    expect(backendQuality(true, 'shaded').transparency).toBe('opaque-depth')
    expect(backendQuality(true, 'xray').transparency).toBe('object-sorted-alpha')
  })

  it('routes material alpha and x-ray bodies through the transparent pass', () => {
    expect(usesTransparentPass(1, 'shaded')).toBe(false)
    expect(usesTransparentPass(0.5, 'shaded')).toBe(true)
    expect(usesTransparentPass(1, 'xray')).toBe(true)
    expect(isTransparentAlpha(0.99)).toBe(true)
    expect(isTransparentAlpha(0.999)).toBe(true)
    expect(isTransparentAlpha(1)).toBe(false)
  })
})
