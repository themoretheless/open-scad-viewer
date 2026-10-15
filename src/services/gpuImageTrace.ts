/**
 * WebGPU Compute Image Contour Tracer (Sub-Pixel Marching Squares).
 *
 * Extracts sub-pixel 2D iso-contours directly from RGBA8 pixel buffers on GPU:
 * 1. Evaluates Rec.709 darkness `(1 - luma) * alpha` or alpha field across `(W-1) x (H-1)` cells.
 * 2. Linearly interpolates sub-pixel boundary crossings in physical millimeter coordinates.
 * 3. Stitches emitted directed 2D segments into closed polygon rings for `imageTraceSketches`.
 */

import { runGpuCompute } from './webgpuCompute'
import { forEachChunkedAsync, type CooperativeAsyncOptions } from './cooperativeAsync'

export interface GpuImageContourOptions extends CooperativeAsyncOptions {
  widthPx: number
  heightPx: number
  widthMm: number
  heightMm?: number
  threshold?: number
  mode?: 'dark' | 'alpha'
  maxSegments?: number
}

export interface ImageContourSegment2D {
  p0: [number, number]
  p1: [number, number]
}

export interface GpuImageContourResult {
  rawSegments: Float32Array
  segments: ImageContourSegment2D[]
  segmentCount: number
  contours: [number, number][][]
}

import IMAGE_CONTOUR_WGSL from '../../crates/compute-core/shaders/image_contour.wgsl?raw'
export { IMAGE_CONTOUR_WGSL }


const IS_LITTLE_ENDIAN = new Uint8Array(new Uint32Array([0x01020304]).buffer)[0] === 0x04
const imageContourUniformBuffer = new ArrayBuffer(32)
const imageContourUniformView = new DataView(imageContourUniformBuffer)

function quantKey(p: [number, number]): number {
  const qx = Math.round(p[0] * 1e4) + 50_000_000
  const qy = Math.round(p[1] * 1e4) + 50_000_000
  return qx * 100_000_000 + qy
}

/**
 * Stitches directed 2D Marching Squares segments into closed or open polyline contours.
 */
export function stitchSegmentsToContours(
  segments: readonly ImageContourSegment2D[]
): [number, number][][] {
  const n = segments.length
  if (n === 0) return []

  const startKeys = new Float64Array(n)
  const endKeys = new Float64Array(n)
  const startMap = new Map<number, { indices: number[]; cursor: number }>()

  for (let i = 0; i < n; i++) {
    const seg = segments[i]!
    const sk = quantKey(seg.p0)
    startKeys[i] = sk
    endKeys[i] = quantKey(seg.p1)
    let bucket = startMap.get(sk)
    if (!bucket) {
      bucket = { indices: [i], cursor: 0 }
      startMap.set(sk, bucket)
    } else {
      bucket.indices.push(i)
    }
  }

  const visited = new Uint8Array(n)
  const contours: [number, number][][] = []

  for (let i = 0; i < n; i++) {
    if (visited[i]) continue
    visited[i] = 1

    const seg0 = segments[i]!
    const ring: [number, number][] = [
      [seg0.p0[0], seg0.p0[1]],
      [seg0.p1[0], seg0.p1[1]],
    ]

    let endKey = endKeys[i]!
    const firstKey = startKeys[i]!

    while (true) {
      if (endKey === firstKey && ring.length >= 3) {
        ring.pop()
        break
      }

      const bucket = startMap.get(endKey)
      if (!bucket) break

      const candidates = bucket.indices
      let nextIdx = -1
      let k = bucket.cursor
      while (k < candidates.length) {
        const cand = candidates[k]!
        if (!visited[cand]) {
          nextIdx = cand
          bucket.cursor = k + 1
          break
        }
        k++
      }
      if (nextIdx === -1) {
        bucket.cursor = candidates.length
        break
      }

      visited[nextIdx] = 1
      const currentEnd = segments[nextIdx]!.p1
      endKey = endKeys[nextIdx]!
      ring.push([currentEnd[0], currentEnd[1]])
    }

    if (ring.length >= 3) {
      contours.push(ring)
    }
  }

  return contours
}

function packRgbaPixels(pixels: Uint8Array | Uint8ClampedArray | Uint32Array): Uint32Array {
  if (pixels instanceof Uint32Array) return pixels
  const numPixels = (pixels.length / 4) | 0
  if (IS_LITTLE_ENDIAN && (pixels.byteOffset & 3) === 0) {
    return new Uint32Array(pixels.buffer, pixels.byteOffset, numPixels)
  }
  const out = new Uint32Array(numPixels)
  for (let i = 0; i < numPixels; i++) {
    const b = i * 4
    out[i] =
      (pixels[b + 0]! & 0xFF) |
      ((pixels[b + 1]! & 0xFF) << 8) |
      ((pixels[b + 2]! & 0xFF) << 16) |
      ((pixels[b + 3]! & 0xFF) << 24)
  }
  return out
}

/**
 * Exact CPU reference implementation of sub-pixel Marching Squares image contour tracing.
 */
export function referenceImageContourTrace(
  rgbaPixels: Uint8Array | Uint8ClampedArray | Uint32Array,
  opts: GpuImageContourOptions
): GpuImageContourResult {
  const gridW = Math.max(2, Math.floor(opts.widthPx))
  const gridH = Math.max(2, Math.floor(opts.heightPx))
  const widthMm = opts.widthMm
  const heightMm = opts.heightMm ?? (widthMm * gridH) / gridW
  const iso = opts.threshold ?? 0.5
  const isAlpha = opts.mode === 'alpha'
  const packed = packRgbaPixels(rgbaPixels)

  const sx = widthMm / Math.max(1, gridW - 1)
  const sy = heightMm / Math.max(1, gridH - 1)
  const inv255 = 1 / 255

  function sampleScalar(ix: number, iy: number): number {
    const cx = Math.min(ix, gridW - 1)
    const cy = Math.min(iy, gridH - 1)
    const rgba = packed[cy * gridW + cx] ?? 0
    const a = ((rgba >>> 24) & 0xFF) * inv255
    if (isAlpha) return a
    const r = (rgba & 0xFF) * inv255
    const g = ((rgba >>> 8) & 0xFF) * inv255
    const b = ((rgba >>> 16) & 0xFF) * inv255
    const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
    return (1 - luma) * a
  }

  function interpFrac(v0: number, v1: number): number {
    const denom = v1 - v0
    if (Math.abs(denom) < 1e-7) return 0.5
    return Math.max(0, Math.min(1, (iso - v0) / denom))
  }

  function toMm(gx: number, gy: number): [number, number] {
    return [gx * sx, (gridH - 1 - gy) * sy]
  }

  const rawList: number[] = []
  const segments: ImageContourSegment2D[] = []

  function emitSeg(p0: [number, number], p1: [number, number]) {
    const dx = p1[0] - p0[0]
    const dy = p1[1] - p0[1]
    if (dx * dx + dy * dy <= 1e-12) return
    rawList.push(p0[0], p0[1], p1[0], p1[1])
    segments.push({ p0, p1 })
  }

  for (let cy = 0; cy < gridH - 1; cy++) {
    for (let cx = 0; cx < gridW - 1; cx++) {
      const v0 = sampleScalar(cx, cy)
      const v1 = sampleScalar(cx + 1, cy)
      const v2 = sampleScalar(cx + 1, cy + 1)
      const v3 = sampleScalar(cx, cy + 1)

      let mask = 0
      if (v0 >= iso) mask |= 1
      if (v1 >= iso) mask |= 2
      if (v2 >= iso) mask |= 4
      if (v3 >= iso) mask |= 8
      if (mask === 0 || mask === 15) continue

      const eTop = toMm(cx + interpFrac(v0, v1), cy)
      const eRight = toMm(cx + 1, cy + interpFrac(v1, v2))
      const eBottom = toMm(cx + interpFrac(v3, v2), cy + 1)
      const eLeft = toMm(cx, cy + interpFrac(v0, v3))

      switch (mask) {
        case 1:
          emitSeg(eLeft, eTop)
          break
        case 2:
          emitSeg(eTop, eRight)
          break
        case 3:
          emitSeg(eLeft, eRight)
          break
        case 4:
          emitSeg(eRight, eBottom)
          break
        case 5: {
          const center = 0.25 * (v0 + v1 + v2 + v3)
          if (center >= iso) {
            emitSeg(eLeft, eBottom)
            emitSeg(eRight, eTop)
          } else {
            emitSeg(eLeft, eTop)
            emitSeg(eRight, eBottom)
          }
          break
        }
        case 6:
          emitSeg(eTop, eBottom)
          break
        case 7:
          emitSeg(eLeft, eBottom)
          break
        case 8:
          emitSeg(eBottom, eLeft)
          break
        case 9:
          emitSeg(eBottom, eTop)
          break
        case 10: {
          const center = 0.25 * (v0 + v1 + v2 + v3)
          if (center >= iso) {
            emitSeg(eTop, eLeft)
            emitSeg(eBottom, eRight)
          } else {
            emitSeg(eTop, eRight)
            emitSeg(eBottom, eLeft)
          }
          break
        }
        case 11:
          emitSeg(eBottom, eRight)
          break
        case 12:
          emitSeg(eRight, eLeft)
          break
        case 13:
          emitSeg(eRight, eTop)
          break
        case 14:
          emitSeg(eTop, eLeft)
          break
      }
    }
  }

  return {
    rawSegments: new Float32Array(rawList),
    segments,
    segmentCount: segments.length,
    contours: stitchSegmentsToContours(segments),
  }
}

/**
 * Cooperative non-blocking async CPU reference for sub-pixel Marching Squares image contour tracing.
 */
export async function referenceImageContourTraceAsync(
  rgbaPixels: Uint8Array | Uint8ClampedArray | Uint32Array,
  opts: GpuImageContourOptions
): Promise<GpuImageContourResult> {
  const gridW = Math.max(2, Math.floor(opts.widthPx))
  const gridH = Math.max(2, Math.floor(opts.heightPx))
  const widthMm = opts.widthMm
  const heightMm = opts.heightMm ?? (widthMm * gridH) / gridW
  const iso = opts.threshold ?? 0.5
  const isAlpha = opts.mode === 'alpha'
  const packed = packRgbaPixels(rgbaPixels)

  const sx = widthMm / Math.max(1, gridW - 1)
  const sy = heightMm / Math.max(1, gridH - 1)
  const inv255 = 1 / 255

  function sampleScalar(ix: number, iy: number): number {
    const cx = Math.min(ix, gridW - 1)
    const cy = Math.min(iy, gridH - 1)
    const rgba = packed[cy * gridW + cx] ?? 0
    const a = ((rgba >>> 24) & 0xFF) * inv255
    if (isAlpha) return a
    const r = (rgba & 0xFF) * inv255
    const g = ((rgba >>> 8) & 0xFF) * inv255
    const b = ((rgba >>> 16) & 0xFF) * inv255
    const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
    return (1 - luma) * a
  }

  function interpFrac(v0: number, v1: number): number {
    const denom = v1 - v0
    if (Math.abs(denom) < 1e-7) return 0.5
    return Math.max(0, Math.min(1, (iso - v0) / denom))
  }

  function toMm(gx: number, gy: number): [number, number] {
    return [gx * sx, (gridH - 1 - gy) * sy]
  }

  const rawList: number[] = []
  const segments: ImageContourSegment2D[] = []

  function emitSeg(p0: [number, number], p1: [number, number]) {
    const dx = p1[0] - p0[0]
    const dy = p1[1] - p0[1]
    if (dx * dx + dy * dy <= 1e-12) return
    rawList.push(p0[0], p0[1], p1[0], p1[1])
    segments.push({ p0, p1 })
  }

  await forEachChunkedAsync(
    gridH - 1,
    cy => {
      for (let cx = 0; cx < gridW - 1; cx++) {
        const v0 = sampleScalar(cx, cy)
        const v1 = sampleScalar(cx + 1, cy)
        const v2 = sampleScalar(cx + 1, cy + 1)
        const v3 = sampleScalar(cx, cy + 1)

        let mask = 0
        if (v0 >= iso) mask |= 1
        if (v1 >= iso) mask |= 2
        if (v2 >= iso) mask |= 4
        if (v3 >= iso) mask |= 8
        if (mask === 0 || mask === 15) continue

        const eTop = toMm(cx + interpFrac(v0, v1), cy)
        const eRight = toMm(cx + 1, cy + interpFrac(v1, v2))
        const eBottom = toMm(cx + interpFrac(v3, v2), cy + 1)
        const eLeft = toMm(cx, cy + interpFrac(v0, v3))

        switch (mask) {
          case 1: emitSeg(eLeft, eTop); break
          case 2: emitSeg(eTop, eRight); break
          case 3: emitSeg(eLeft, eRight); break
          case 4: emitSeg(eRight, eBottom); break
          case 5: {
            const center = 0.25 * (v0 + v1 + v2 + v3)
            if (center >= iso) { emitSeg(eLeft, eBottom); emitSeg(eRight, eTop) }
            else { emitSeg(eLeft, eTop); emitSeg(eRight, eBottom) }
            break
          }
          case 6: emitSeg(eTop, eBottom); break
          case 7: emitSeg(eLeft, eBottom); break
          case 8: emitSeg(eBottom, eLeft); break
          case 9: emitSeg(eBottom, eTop); break
          case 10: {
            const center = 0.25 * (v0 + v1 + v2 + v3)
            if (center >= iso) { emitSeg(eTop, eLeft); emitSeg(eBottom, eRight) }
            else { emitSeg(eTop, eRight); emitSeg(eBottom, eLeft) }
            break
          }
          case 11: emitSeg(eBottom, eRight); break
          case 12: emitSeg(eRight, eLeft); break
          case 13: emitSeg(eRight, eTop); break
          case 14: emitSeg(eTop, eLeft); break
        }
      }
    },
    { ...opts, checkStride: Math.max(1, Math.floor(2048 / Math.max(1, gridW - 1))) }
  )

  return {
    rawSegments: new Float32Array(rawList),
    segments,
    segmentCount: segments.length,
    contours: stitchSegmentsToContours(segments),
  }
}

/**
 * Extracts sub-pixel 2D Marching Squares contours on WebGPU Compute.
 * Falls back to deterministic CPU reference when WebGPU is unavailable.
 */
export async function runGpuImageContourTrace(
  rgbaPixels: Uint8Array | Uint8ClampedArray | Uint32Array,
  opts: GpuImageContourOptions
): Promise<GpuImageContourResult> {
  const totalCells = Math.max(1, (Math.floor(opts.widthPx) - 1) * (Math.floor(opts.heightPx) - 1))
  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return totalCells >= 4096
      ? referenceImageContourTraceAsync(rgbaPixels, opts)
      : referenceImageContourTrace(rgbaPixels, opts)
  }

  try {
    const gridW = Math.max(2, Math.floor(opts.widthPx))
    const gridH = Math.max(2, Math.floor(opts.heightPx))
    const widthMm = opts.widthMm
    const heightMm = opts.heightMm ?? (widthMm * gridH) / gridW
    const threshold = opts.threshold ?? 0.5
    const mode = opts.mode === 'alpha' ? 1 : 0
    const maxSegments = opts.maxSegments ?? Math.max(256, totalCells * 2)

    imageContourUniformView.setUint32(0, gridW, true)
    imageContourUniformView.setUint32(4, gridH, true)
    imageContourUniformView.setUint32(8, mode, true)
    imageContourUniformView.setUint32(12, maxSegments, true)
    imageContourUniformView.setFloat32(16, threshold, true)
    imageContourUniformView.setFloat32(20, widthMm, true)
    imageContourUniformView.setFloat32(24, heightMm, true)
    imageContourUniformView.setFloat32(28, 0, true)

    const packedPixels = packRgbaPixels(rgbaPixels)
    const outBytes = maxSegments * 4 * 4

    const results = await runGpuCompute({
      wgsl: IMAGE_CONTOUR_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: imageContourUniformBuffer, uniform: true },
            { binding: 1, data: packedPixels },
            { binding: 2, data: new Uint32Array([0]), output: true, outputBytes: 4 },
            { binding: 3, data: new Float32Array(0), output: true, outputBytes: outBytes },
          ],
          outputBytes: outBytes,
          workgroups: [Math.ceil(totalCells / 256), 1, 1],
        },
      ],
    })

    const count = Math.min(results[0]?.[0] ?? 0, maxSegments)
    const rawOut = results[1]
    if (rawOut) {
      const sliced = rawOut.slice(0, count * 4)
      const segments: ImageContourSegment2D[] = new Array(count)
      for (let i = 0; i < count; i++) {
        const off = i * 4
        segments[i] = {
          p0: [sliced[off + 0]!, sliced[off + 1]!],
          p1: [sliced[off + 2]!, sliced[off + 3]!],
        }
      }
      return {
        rawSegments: sliced,
        segments,
        segmentCount: count,
        contours: stitchSegmentsToContours(segments),
      }
    }
  } catch {
    // Fallback to CPU reference
  }

  return totalCells >= 4096
    ? referenceImageContourTraceAsync(rgbaPixels, opts)
    : referenceImageContourTrace(rgbaPixels, opts)
}
