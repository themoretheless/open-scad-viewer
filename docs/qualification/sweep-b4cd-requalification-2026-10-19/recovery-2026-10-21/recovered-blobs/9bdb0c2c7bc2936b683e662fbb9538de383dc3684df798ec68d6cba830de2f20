/**
 * WebGPU Compute G-Code Toolpath Segment & Layer Filter Accelerator.
 *
 * Filters and projects packed G-Code move rows (7 floats per move:
 * [x, y, z, e, feedrateMmS, layerIndex, extrudedFlag]) into packed 3D/2D segments
 * with normalized feedrate [0..1] for hardware-accelerated preview rendering.
 */

import { runGpuCompute } from './webgpuCompute'
import { GCODE_MOVE_ROW_WIDTH } from './gcodePreviewTransport'
import { forEachChunkedAsync, type CooperativeAsyncOptions } from './cooperativeAsync'

export interface GcodeFilterOptions extends CooperativeAsyncOptions {
  minLayer?: number
  maxLayer?: number
  includeTravel?: boolean
  includeExtrusion?: boolean
  minFeedrateMmS?: number
  maxFeedrateMmS?: number
  maxSegments?: number
}

export interface GcodeToolpathSegment {
  p0: [number, number, number]
  p1: [number, number, number]
  feedrateNorm: number
  extruded: boolean
}

export interface GcodeSegmentFilterResult {
  /** Packed 8 floats per segment: [x0, y0, z0, feedrateNorm, x1, y1, z1, extrudedFlag] */
  rawSegments: Float32Array
  segments: GcodeToolpathSegment[]
  segmentCount: number
}

import GCODE_SEGMENTS_WGSL from '../../crates/compute-core/shaders/gcode_segments.wgsl?raw'
export { GCODE_SEGMENTS_WGSL }


function unpackGcodeSegments(raw: Float32Array, count: number): GcodeToolpathSegment[] {
  const segments: GcodeToolpathSegment[] = []
  for (let i = 0; i < count; i++) {
    const off = i * 8
    segments.push({
      p0: [raw[off + 0]!, raw[off + 1]!, raw[off + 2]!],
      feedrateNorm: raw[off + 3]!,
      p1: [raw[off + 4]!, raw[off + 5]!, raw[off + 6]!],
      extruded: raw[off + 7]! >= 0.5,
    })
  }
  return segments
}

/**
 * Exact CPU reference implementation of G-Code move segment filtering.
 */
export function referenceFilterGcodeSegments(
  moveRows: Float64Array | Float32Array,
  opts: GcodeFilterOptions = {}
): GcodeSegmentFilterResult {
  const numMoves = Math.floor(moveRows.length / GCODE_MOVE_ROW_WIDTH)
  const minLayer = opts.minLayer ?? 0
  const maxLayer = opts.maxLayer ?? 2048
  const includeTravel = opts.includeTravel ?? true
  const includeExtrusion = opts.includeExtrusion ?? true
  const minFeed = opts.minFeedrateMmS ?? 10
  const maxFeed = opts.maxFeedrateMmS ?? 120
  const maxSegments = opts.maxSegments ?? Math.max(1, numMoves)
  const span = Math.max(1e-6, maxFeed - minFeed)

  const rawList: number[] = []
  const segments: GcodeToolpathSegment[] = []

  for (let m = 1; m < numMoves; m++) {
    const cur = m * GCODE_MOVE_ROW_WIDTH
    const prev = (m - 1) * GCODE_MOVE_ROW_WIDTH
    const layer = Math.round(moveRows[cur + 5]!)
    if (layer < minLayer || layer > maxLayer) continue

    const isExtrusion = moveRows[cur + 6]! >= 0.5
    if (isExtrusion && !includeExtrusion) continue
    if (!isExtrusion && !includeTravel) continue

    const x0 = moveRows[prev + 0]!, y0 = moveRows[prev + 1]!, z0 = moveRows[prev + 2]!
    const x1 = moveRows[cur + 0]!, y1 = moveRows[cur + 1]!, z1 = moveRows[cur + 2]!
    const dx = x1 - x0, dy = y1 - y0, dz = z1 - z0
    if (dx * dx + dy * dy + dz * dz <= 1e-12) continue

    const feed = moveRows[cur + 4]!
    const feedNorm = Math.max(0, Math.min(1, (feed - minFeed) / span))
    const extFlag = isExtrusion ? 1 : 0

    if (segments.length < maxSegments) {
      rawList.push(x0, y0, z0, feedNorm, x1, y1, z1, extFlag)
      segments.push({
        p0: [x0, y0, z0],
        p1: [x1, y1, z1],
        feedrateNorm: feedNorm,
        extruded: isExtrusion,
      })
    }
  }

  return {
    rawSegments: new Float32Array(rawList),
    segments,
    segmentCount: segments.length,
  }
}

/**
 * Cooperative non-blocking async CPU reference for G-Code move segment filtering.
 */
export async function referenceFilterGcodeSegmentsAsync(
  moveRows: Float64Array | Float32Array,
  opts: GcodeFilterOptions = {}
): Promise<GcodeSegmentFilterResult> {
  const numMoves = Math.floor(moveRows.length / GCODE_MOVE_ROW_WIDTH)
  if (numMoves <= 1) return referenceFilterGcodeSegments(moveRows, opts)

  const minLayer = opts.minLayer ?? 0
  const maxLayer = opts.maxLayer ?? 2048
  const includeTravel = opts.includeTravel ?? true
  const includeExtrusion = opts.includeExtrusion ?? true
  const minFeed = opts.minFeedrateMmS ?? 10
  const maxFeed = opts.maxFeedrateMmS ?? 120
  const maxSegments = opts.maxSegments ?? Math.max(1, numMoves)
  const span = Math.max(1e-6, maxFeed - minFeed)

  const rawList: number[] = []
  const segments: GcodeToolpathSegment[] = []

  await forEachChunkedAsync(
    numMoves - 1,
    idx => {
      const m = idx + 1
      const cur = m * GCODE_MOVE_ROW_WIDTH
      const prev = (m - 1) * GCODE_MOVE_ROW_WIDTH
      const layer = Math.round(moveRows[cur + 5]!)
      if (layer < minLayer || layer > maxLayer) return

      const isExtrusion = moveRows[cur + 6]! >= 0.5
      if (isExtrusion && !includeExtrusion) return
      if (!isExtrusion && !includeTravel) return

      const x0 = moveRows[prev + 0]!, y0 = moveRows[prev + 1]!, z0 = moveRows[prev + 2]!
      const x1 = moveRows[cur + 0]!, y1 = moveRows[cur + 1]!, z1 = moveRows[cur + 2]!
      const dx = x1 - x0, dy = y1 - y0, dz = z1 - z0
      if (dx * dx + dy * dy + dz * dz <= 1e-12) return

      const feed = moveRows[cur + 4]!
      const feedNorm = Math.max(0, Math.min(1, (feed - minFeed) / span))
      const extFlag = isExtrusion ? 1 : 0

      if (segments.length < maxSegments) {
        rawList.push(x0, y0, z0, feedNorm, x1, y1, z1, extFlag)
        segments.push({
          p0: [x0, y0, z0],
          p1: [x1, y1, z1],
          feedrateNorm: feedNorm,
          extruded: isExtrusion,
        })
      }
    },
    opts
  )

  return {
    rawSegments: new Float32Array(rawList),
    segments,
    segmentCount: segments.length,
  }
}

/**
 * Filters G-Code move rows in parallel on WebGPU Compute.
 * Falls back to deterministic CPU reference when WebGPU is unavailable.
 */
export async function runGpuFilterGcodeSegments(
  moveRows: Float64Array | Float32Array,
  opts: GcodeFilterOptions = {}
): Promise<GcodeSegmentFilterResult> {
  const numMoves = Math.floor(moveRows.length / GCODE_MOVE_ROW_WIDTH)
  if (numMoves <= 1 || typeof navigator === 'undefined' || !navigator.gpu) {
    return numMoves >= 1024
      ? referenceFilterGcodeSegmentsAsync(moveRows, opts)
      : referenceFilterGcodeSegments(moveRows, opts)
  }

  try {
    const minLayer = opts.minLayer ?? 0
    const maxLayer = opts.maxLayer ?? 2048
    const includeTravel = opts.includeTravel ?? true
    const includeExtrusion = opts.includeExtrusion ?? true
    const minFeed = opts.minFeedrateMmS ?? 10
    const maxFeed = opts.maxFeedrateMmS ?? 120
    const maxSegments = opts.maxSegments ?? Math.max(1, numMoves)

    const uniformBuf = new ArrayBuffer(32)
    const v = new DataView(uniformBuf)
    v.setUint32(0, numMoves, true)
    v.setUint32(4, minLayer, true)
    v.setUint32(8, maxLayer, true)
    v.setUint32(12, includeTravel ? 1 : 0, true)
    v.setUint32(16, includeExtrusion ? 1 : 0, true)
    v.setUint32(20, maxSegments, true)
    v.setFloat32(24, minFeed, true)
    v.setFloat32(28, maxFeed, true)

    const rowsF32 = moveRows instanceof Float32Array ? moveRows : new Float32Array(moveRows)
    const outBytes = maxSegments * 8 * 4

    const results = await runGpuCompute({
      wgsl: GCODE_SEGMENTS_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformBuf, uniform: true },
            { binding: 1, data: rowsF32 },
            { binding: 2, data: new Uint32Array([0]), output: true, outputBytes: 4 },
            { binding: 3, data: new Float32Array(maxSegments * 8), output: true, outputBytes: outBytes },
          ],
          outputBytes: outBytes,
          workgroups: [Math.ceil(numMoves / 256), 1, 1],
        },
      ],
    })

    const count = Math.min(results[0]?.[0] ?? 0, maxSegments)
    const rawOut = results[1]
    if (rawOut) {
      const sliced = rawOut.slice(0, count * 8)
      return {
        rawSegments: sliced,
        segments: unpackGcodeSegments(sliced, count),
        segmentCount: count,
      }
    }
  } catch {
    // Fallback to CPU reference
  }

  return numMoves >= 1024
    ? referenceFilterGcodeSegmentsAsync(moveRows, opts)
    : referenceFilterGcodeSegments(moveRows, opts)
}
