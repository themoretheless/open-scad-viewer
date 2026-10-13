/**
 * WebGPU Compute Image Contour Tracer (Sub-Pixel Marching Squares).
 *
 * Extracts sub-pixel 2D iso-contours directly from RGBA8 pixel buffers on GPU:
 * 1. Evaluates Rec.709 darkness `(1 - luma) * alpha` or alpha field across `(W-1) x (H-1)` cells.
 * 2. Linearly interpolates sub-pixel boundary crossings in physical millimeter coordinates.
 * 3. Stitches emitted directed 2D segments into closed polygon rings for `imageTraceSketches`.
 */

import { runGpuCompute } from './webgpuCompute'

export interface GpuImageContourOptions {
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

export const IMAGE_CONTOUR_WGSL = /* wgsl */`
struct ContourParams {
  grid_w: u32,
  grid_h: u32,
  mode: u32,
  max_segments: u32,
  threshold: f32,
  width_mm: f32,
  height_mm: f32,
  _pad0: f32,
};

@group(0) @binding(0) var<uniform> params: ContourParams;
@group(0) @binding(1) var<storage, read> rgba_pixels: array<u32>;
@group(0) @binding(2) var<storage, read_write> segment_counter: array<atomic<u32>, 1>;
@group(0) @binding(3) var<storage, read_write> out_segments: array<f32>;

fn sample_scalar(ix: u32, iy: u32) -> f32 {
  let cx = min(ix, params.grid_w - 1u);
  let cy = min(iy, params.grid_h - 1u);
  let rgba = rgba_pixels[cy * params.grid_w + cx];

  let r = f32(rgba & 0xFFu) / 255.0;
  let g = f32((rgba >> 8u) & 0xFFu) / 255.0;
  let b = f32((rgba >> 16u) & 0xFFu) / 255.0;
  let a = f32((rgba >> 24u) & 0xFFu) / 255.0;

  if (params.mode == 1u) {
    return a;
  }
  let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return (1.0 - luma) * a;
}

fn interp_frac(v0: f32, v1: f32, iso: f32) -> f32 {
  let denom = v1 - v0;
  if (abs(denom) < 1e-7) {
    return 0.5;
  }
  return clamp((iso - v0) / denom, 0.0, 1.0);
}

fn to_mm(gx: f32, gy: f32) -> vec2<f32> {
  let sx = params.width_mm / max(1.0, f32(params.grid_w - 1u));
  let sy = params.height_mm / max(1.0, f32(params.grid_h - 1u));
  return vec2<f32>(gx * sx, (f32(params.grid_h - 1u) - gy) * sy);
}

fn emit_seg(p0: vec2<f32>, p1: vec2<f32>) {
  let d = p1 - p0;
  if (dot(d, d) <= 1e-12) {
    return;
  }
  let slot = atomicAdd(&segment_counter[0], 1u);
  if (slot < params.max_segments) {
    let off = slot * 4u;
    out_segments[off + 0u] = p0.x;
    out_segments[off + 1u] = p0.y;
    out_segments[off + 2u] = p1.x;
    out_segments[off + 3u] = p1.y;
  }
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (params.grid_w < 2u || params.grid_h < 2u) {
    return;
  }
  let cells_w = params.grid_w - 1u;
  let cells_h = params.grid_h - 1u;
  let total_cells = cells_w * cells_h;
  let cell_idx = id.x;
  if (cell_idx >= total_cells) {
    return;
  }

  let cx = cell_idx % cells_w;
  let cy = cell_idx / cells_w;

  let v0 = sample_scalar(cx, cy);
  let v1 = sample_scalar(cx + 1u, cy);
  let v2 = sample_scalar(cx + 1u, cy + 1u);
  let v3 = sample_scalar(cx, cy + 1u);

  let iso = params.threshold;
  var mask = 0u;
  if (v0 >= iso) { mask |= 1u; }
  if (v1 >= iso) { mask |= 2u; }
  if (v2 >= iso) { mask |= 4u; }
  if (v3 >= iso) { mask |= 8u; }

  if (mask == 0u || mask == 15u) {
    return;
  }

  let fx = f32(cx);
  let fy = f32(cy);

  let e_top = to_mm(fx + interp_frac(v0, v1, iso), fy);
  let e_right = to_mm(fx + 1.0, fy + interp_frac(v1, v2, iso));
  let e_bottom = to_mm(fx + interp_frac(v3, v2, iso), fy + 1.0);
  let e_left = to_mm(fx, fy + interp_frac(v0, v3, iso));

  switch (mask) {
    case 1u:  { emit_seg(e_left, e_top); }
    case 2u:  { emit_seg(e_top, e_right); }
    case 3u:  { emit_seg(e_left, e_right); }
    case 4u:  { emit_seg(e_right, e_bottom); }
    case 5u:  {
      let center = 0.25 * (v0 + v1 + v2 + v3);
      if (center >= iso) {
        emit_seg(e_left, e_bottom);
        emit_seg(e_right, e_top);
      } else {
        emit_seg(e_left, e_top);
        emit_seg(e_right, e_bottom);
      }
    }
    case 6u:  { emit_seg(e_top, e_bottom); }
    case 7u:  { emit_seg(e_left, e_bottom); }
    case 8u:  { emit_seg(e_bottom, e_left); }
    case 9u:  { emit_seg(e_bottom, e_top); }
    case 10u: {
      let center = 0.25 * (v0 + v1 + v2 + v3);
      if (center >= iso) {
        emit_seg(e_top, e_left);
        emit_seg(e_bottom, e_right);
      } else {
        emit_seg(e_top, e_right);
        emit_seg(e_bottom, e_left);
      }
    }
    case 11u: { emit_seg(e_bottom, e_right); }
    case 12u: { emit_seg(e_right, e_left); }
    case 13u: { emit_seg(e_right, e_top); }
    case 14u: { emit_seg(e_top, e_left); }
    default: {}
  }
}
`

function quantKey(p: [number, number]): string {
  return `${Math.round(p[0] * 1e4)}:${Math.round(p[1] * 1e4)}`
}

/**
 * Stitches directed 2D Marching Squares segments into closed or open polyline contours.
 */
export function stitchSegmentsToContours(
  segments: readonly ImageContourSegment2D[]
): [number, number][][] {
  if (segments.length === 0) return []

  const startMap = new Map<string, number[]>()
  for (let i = 0; i < segments.length; i++) {
    const key = quantKey(segments[i]!.p0)
    let bucket = startMap.get(key)
    if (!bucket) {
      bucket = []
      startMap.set(key, bucket)
    }
    bucket.push(i)
  }

  const visited = new Uint8Array(segments.length)
  const contours: [number, number][][] = []

  for (let i = 0; i < segments.length; i++) {
    if (visited[i]) continue
    visited[i] = 1

    const ring: [number, number][] = [
      [segments[i]!.p0[0], segments[i]!.p0[1]],
      [segments[i]!.p1[0], segments[i]!.p1[1]],
    ]

    let currentEnd = segments[i]!.p1
    const firstKey = quantKey(segments[i]!.p0)

    while (true) {
      const endKey = quantKey(currentEnd)
      if (endKey === firstKey && ring.length >= 3) {
        ring.pop()
        break
      }

      const candidates = startMap.get(endKey)
      if (!candidates) break

      let nextIdx = -1
      for (let k = 0; k < candidates.length; k++) {
        const cand = candidates[k]!
        if (!visited[cand]) {
          nextIdx = cand
          break
        }
      }

      if (nextIdx === -1) break
      visited[nextIdx] = 1
      currentEnd = segments[nextIdx]!.p1
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
  const numPixels = Math.floor(pixels.length / 4)
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

  function sampleScalar(ix: number, iy: number): number {
    const cx = Math.min(ix, gridW - 1)
    const cy = Math.min(iy, gridH - 1)
    const rgba = packed[cy * gridW + cx] ?? 0
    const r = (rgba & 0xFF) / 255
    const g = ((rgba >>> 8) & 0xFF) / 255
    const b = ((rgba >>> 16) & 0xFF) / 255
    const a = ((rgba >>> 24) & 0xFF) / 255
    if (isAlpha) return a
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
 * Extracts sub-pixel 2D Marching Squares contours on WebGPU Compute.
 * Falls back to deterministic CPU reference when WebGPU is unavailable.
 */
export async function runGpuImageContourTrace(
  rgbaPixels: Uint8Array | Uint8ClampedArray | Uint32Array,
  opts: GpuImageContourOptions
): Promise<GpuImageContourResult> {
  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return referenceImageContourTrace(rgbaPixels, opts)
  }

  try {
    const gridW = Math.max(2, Math.floor(opts.widthPx))
    const gridH = Math.max(2, Math.floor(opts.heightPx))
    const widthMm = opts.widthMm
    const heightMm = opts.heightMm ?? (widthMm * gridH) / gridW
    const threshold = opts.threshold ?? 0.5
    const mode = opts.mode === 'alpha' ? 1 : 0
    const totalCells = (gridW - 1) * (gridH - 1)
    const maxSegments = opts.maxSegments ?? Math.max(256, totalCells * 2)

    const uniformBuf = new ArrayBuffer(32)
    const v = new DataView(uniformBuf)
    v.setUint32(0, gridW, true)
    v.setUint32(4, gridH, true)
    v.setUint32(8, mode, true)
    v.setUint32(12, maxSegments, true)
    v.setFloat32(16, threshold, true)
    v.setFloat32(20, widthMm, true)
    v.setFloat32(24, heightMm, true)
    v.setFloat32(28, 0, true)

    const packedPixels = packRgbaPixels(rgbaPixels)
    const outBytes = maxSegments * 4 * 4

    const results = await runGpuCompute({
      wgsl: IMAGE_CONTOUR_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformBuf, uniform: true },
            { binding: 1, data: packedPixels },
            { binding: 2, data: new Uint32Array([0]), output: true, outputBytes: 4 },
            { binding: 3, data: new Float32Array(maxSegments * 4), output: true, outputBytes: outBytes },
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
      const segments: ImageContourSegment2D[] = []
      for (let i = 0; i < count; i++) {
        const off = i * 4
        segments.push({
          p0: [sliced[off + 0]!, sliced[off + 1]!],
          p1: [sliced[off + 2]!, sliced[off + 3]!],
        })
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

  return referenceImageContourTrace(rgbaPixels, opts)
}
