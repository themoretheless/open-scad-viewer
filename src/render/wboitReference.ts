/**
 * CPU reference for the WBOIT pass in `gpuWboit.ts`. It mirrors the WGSL
 * weight, the two blend states (additive accumulation, multiplicative
 * revealage) and the composite shader so the math can be qualified without a
 * GPU. It does not model fp16/unorm8 rounding beyond the documented limits.
 */

export type Rgb = readonly [number, number, number]

export interface WboitFragment {
  /** Straight (non-premultiplied) fragment color. */
  readonly color: Rgb
  readonly alpha: number
  /** Window-space depth in [0, 1], as `v.p.z` in the fragment shader. */
  readonly depth: number
}

export interface WboitPixel {
  /** Accumulation target: sum of premultiplied color and alpha, each times the weight. */
  readonly accum: readonly [number, number, number, number]
  /** Revealage target: product of (1 - alpha). */
  readonly reveal: number
}

/**
 * Upper clamp of the WGSL weight; also bounds one fragment's accumulation
 * contribution (the shader multiplies the weight by alpha), so rgba16float
 * holds floor(65504 / 500) = 131 fully opaque layers before it overflows.
 */
export const WBOIT_WEIGHT_MAX = 5e2
export const WBOIT_WEIGHT_MIN = 1e-2
/** Largest finite rgba16float value. */
export const HALF_FLOAT_MAX = 65504

/**
 * Same expression as `wboitWeight` in the WGSL produced by `toWboitShader`.
 * The weight depends on depth only: alpha already scales the accumulated
 * value, so an alpha term here would only push the weight into the clamp.
 * Nearer fragments weigh up to 8000x more than fragments at the far plane.
 */
export function wboitWeight(depth: number, _alpha?: number): number {
  const b = 1 - depth * 0.95
  return Math.min(WBOIT_WEIGHT_MAX, Math.max(WBOIT_WEIGHT_MIN, WBOIT_WEIGHT_MAX * b ** 3))
}

/** Clear values of the accumulation pass. */
export const emptyWboitPixel = (): WboitPixel => ({ accum: [0, 0, 0, 0], reveal: 1 })

/** One fragment through both blend states: one+one on target 0, zero/one-minus-src on target 1. */
export function blendWboitFragment(pixel: WboitPixel, fragment: WboitFragment): WboitPixel {
  const { color, alpha, depth } = fragment
  const w = wboitWeight(depth, alpha)
  const [r, g, b, a] = pixel.accum
  return {
    accum: [r + color[0] * alpha * w, g + color[1] * alpha * w, b + color[2] * alpha * w, a + alpha * w],
    reveal: pixel.reveal * (1 - alpha),
  }
}

export function accumulateWboit(fragments: readonly WboitFragment[]): WboitPixel {
  return fragments.reduce(blendWboitFragment, emptyWboitPixel())
}

/**
 * `fs_composite` followed by the src-alpha / one-minus-src-alpha blend over the
 * opaque color. A fully revealing pixel is discarded and keeps the background.
 */
export function compositeWboit(pixel: WboitPixel, background: Rgb): Rgb {
  if (pixel.reveal >= 0.99999) return background
  const total = Math.max(pixel.accum[3], 1e-5)
  const alpha = 1 - pixel.reveal
  return [0, 1, 2].map(
    channel => (pixel.accum[channel]! / total) * alpha + background[channel]! * (1 - alpha),
  ) as unknown as Rgb
}

/** Exact back-to-front "over" compositing; fragments are sorted far to near by depth. */
export function sortedOverReference(fragments: readonly WboitFragment[], background: Rgb): Rgb {
  const ordered = [...fragments].sort((left, right) => right.depth - left.depth)
  let out: [number, number, number] = [...background]
  for (const { color, alpha } of ordered) {
    out = [0, 1, 2].map(channel => color[channel]! * alpha + out[channel]! * (1 - alpha)) as [number, number, number]
  }
  return out
}
