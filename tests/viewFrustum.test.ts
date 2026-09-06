import { describe, expect, it } from 'vitest'
import { ViewFrustum } from '../src/services/viewFrustum'
import { invert, lookAt, multiply, orthographic, perspective, transformPoint } from '../src/services/math3d'

describe('WebGPU view frustum', () => {
  it('uses zero-to-one depth and keeps spheres touching every clip boundary', () => {
    const frustum = new ViewFrustum()
    frustum.update(orthographic(-2, 2, -3, 3, 1, 10))
    for (const center of [[2.5, 0, -5], [-2.5, 0, -5], [0, 3.5, -5], [0, -3.5, -5], [0, 0, -0.5], [0, 0, -10.5]]) {
      expect(frustum.intersects(center, 0.5)).toBe(true)
      expect(frustum.intersects(center, 0.4)).toBe(false)
    }
    expect(frustum.intersects([0, 0, 1], 0)).toBe(false)
    expect(frustum.intersects([0, 0, -5], 20)).toBe(true)
  })

  it('rejects outside perspective objects and preserves objects crossing a plane', () => {
    const frustum = new ViewFrustum()
    frustum.update(perspective(Math.PI / 2, 1, 1, 100))
    expect(frustum.intersects([12, 0, -10], 0.1)).toBe(false)
    expect(frustum.intersects([12, 0, -10], 2)).toBe(true)
    expect(frustum.intersects([0, 0, 10], 1)).toBe(false)
  })

  it('never rejects sampled visible points with rotated or distant cameras', () => {
    for (const origin of [0, 1e5]) for (const projection of [perspective(Math.PI / 3, 1.5, 0.1, 1000), orthographic(-20, 20, -10, 10, 0.1, 1000)]) {
      const matrix = multiply(projection, lookAt([origin + 15, origin - 20, origin + 30], [origin, origin, origin], [0, 0, 1]))
      const inverse = invert(matrix)
      const frustum = new ViewFrustum()
      frustum.update(matrix)
      for (let i = 0; i < 200; i++) {
        const world = transformPoint(inverse, [Math.sin(i) * 0.99, Math.cos(i * 3) * 0.99, (i % 100) / 101])
        expect(frustum.intersects(world, 0)).toBe(true)
      }
    }
  })

  it('fails open for a degenerate projection', () => {
    const frustum = new ViewFrustum()
    frustum.update(new Float32Array(16))
    expect(frustum.intersects([1e20, -1e20, 1e20], 1)).toBe(true)
  })
})
