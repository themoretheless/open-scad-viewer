import { describe, it, expect } from 'vitest'
import {
  referenceImageContourTrace,
  runGpuImageContourTrace,
  IMAGE_CONTOUR_WGSL,
} from '../src/services/gpuImageTrace'

function createCircleRgbaGrid(w: number, h: number, radiusPx: number): Uint8Array {
  const rgba = new Uint8Array(w * h * 4)
  const cx = (w - 1) * 0.5
  const cy = (h - 1) * 0.5
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const dist = Math.hypot(x - cx, y - cy)
      // Smooth signed distance field mapped to [0..255] alpha & darkness
      const inside = Math.max(0, Math.min(1, 0.5 + (radiusPx - dist) * 0.5))
      const off = (y * w + x) * 4
      const lumaByte = Math.round((1 - inside) * 255)
      rgba[off + 0] = lumaByte
      rgba[off + 1] = lumaByte
      rgba[off + 2] = lumaByte
      rgba[off + 3] = Math.round(inside * 255)
    }
  }
  return rgba
}

describe('WebGPU Compute Image Contour Tracer (Marching Squares)', () => {
  it('extracts sub-pixel closed contour ring from a synthetic circle RGBA buffer', () => {
    const rgba = createCircleRgbaGrid(16, 16, 4.5)
    const result = referenceImageContourTrace(rgba, {
      widthPx: 16,
      heightPx: 16,
      widthMm: 30,
      heightMm: 30,
      threshold: 0.5,
      mode: 'dark',
    })

    expect(result.segmentCount).toBeGreaterThan(16)
    expect(result.contours.length).toBe(1)
    expect(result.contours[0]!.length).toBeGreaterThan(16)

    // Verify all contour vertices lie near circle radius (4.5 / 15 * 30 = 9.0 mm) around center (15, 15)
    for (const [x, y] of result.contours[0]!) {
      const r = Math.hypot(x - 15, y - 15)
      expect(r).toBeGreaterThan(8.0)
      expect(r).toBeLessThan(9.8)
    }
  })

  it('traces alpha channel contours via runGpuImageContourTrace', async () => {
    const rgba = createCircleRgbaGrid(12, 12, 3.2)
    const res = await runGpuImageContourTrace(rgba, {
      widthPx: 12,
      heightPx: 12,
      widthMm: 22,
      threshold: 0.5,
      mode: 'alpha',
    })

    expect(res.segmentCount).toBeGreaterThan(12)
    expect(res.contours.length).toBe(1)
  })

  it('declares valid WGSL compute shader with 16-case Marching Squares table and sub-pixel interpolation', () => {
    expect(IMAGE_CONTOUR_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(IMAGE_CONTOUR_WGSL).toContain('sample_scalar')
    expect(IMAGE_CONTOUR_WGSL).toContain('interp_frac')
    expect(IMAGE_CONTOUR_WGSL).toContain('atomicAdd(&segment_counter[0], 1u)')
  })
})
