/**
 * Canvas surface sizing and the main depth attachment, split out of the
 * renderer so the DPR/limit clamping and depth reallocation rules can be
 * tested with a fake device.
 */
import { MAX_DPR } from './types'

export interface SurfaceSize {
  width: number
  height: number
}

/**
 * Device-pixel size for a canvas of the given CSS size, or null when the
 * canvas has no drawable area. DPR is clamped to [1, MAX_DPR] and the
 * result never exceeds `maxTextureDimension2D` while keeping the aspect.
 */
export function computeSurfaceSize(
  cssWidth: number,
  cssHeight: number,
  devicePixelRatio: number,
  maxTextureDimension2D: number,
): SurfaceSize | null {
  const w = Number.isFinite(cssWidth) ? Math.max(0, cssWidth) : 0
  const h = Number.isFinite(cssHeight) ? Math.max(0, cssHeight) : 0
  if (w <= 0 || h <= 0) return null
  const rawDpr = Number.isFinite(devicePixelRatio) ? devicePixelRatio : 1
  const dpr = Math.min(MAX_DPR, Math.max(1, rawDpr))
  const maxDimension = Math.max(1, maxTextureDimension2D)
  const scale = Math.min(dpr, maxDimension / w, maxDimension / h)
  return {
    width: Math.max(1, Math.min(maxDimension, Math.round(w * scale))),
    height: Math.max(1, Math.min(maxDimension, Math.round(h * scale))),
  }
}

export interface DepthTarget {
  texture: GPUTexture
  view: GPUTextureView
}

/** Allocates the main pass depth attachment for the given surface size. */
export function createDepthTarget(device: GPUDevice, size: SurfaceSize): DepthTarget {
  const texture = device.createTexture({
    size: [size.width, size.height],
    format: 'depth24plus',
    usage: GPUTextureUsage.RENDER_ATTACHMENT,
  })
  return { texture, view: texture.createView() }
}
