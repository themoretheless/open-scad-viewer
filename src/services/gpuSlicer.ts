/**
 * WebGPU Compute Multi-Plane Mesh Slicer & Cross-Section Contour Extractor.
 *
 * Slices 3D triangle meshes across 1 or N parallel planes in a single GPU Compute pass:
 * 1. O(1) per-triangle layer interval culling [l_start .. l_end].
 * 2. Exact edge-plane linear interpolation.
 * 3. Canonical counter-clockwise contour winding around the slicing plane normal.
 * 4. Exact cross-sectional area evaluation via Stokes' / Green's theorem.
 */

import { runGpuCompute } from './webgpuCompute'
import { dot3, cross3, normalizeVec3 } from './math3d'
import { forEachChunkedAsync, type CooperativeAsyncOptions } from './cooperativeAsync'

export interface SlicerOptions extends CooperativeAsyncOptions {
  /** Slicing plane normal vector (default: [0, 0, 1] for Z-slicing) */
  planeNormal?: [number, number, number]
  /** Distance d of the first slicing plane (n . p = d) */
  firstLayerDistance: number
  /** Step between consecutive parallel layers (default: 0.2) */
  layerStep?: number
  /** Number of parallel layers to slice simultaneously (default: 1) */
  numLayers?: number
  /** Float stride per vertex in vertices buffer (default: 3) */
  strideFloats?: number
  /** Float offset of XYZ position in vertex stride (default: 0) */
  posOffset?: number
  /** Maximum segments to allocate in output buffer */
  maxSegments?: number
}

export interface SliceSegment {
  p0: [number, number, number]
  p1: [number, number, number]
  layerIndex: number
  triangleIndex: number
}

export interface SlicerResult {
  /** Raw packed segments: 8 floats per segment [x0, y0, z0, layerIdx, x1, y1, z1, triIdx] */
  rawSegments: Float32Array
  /** Parsed structured segments */
  segments: SliceSegment[]
  /** Number of emitted contour segments */
  segmentCount: number
}

import MESH_SLICER_WGSL from '../../crates/compute-core/shaders/mesh_slicer.wgsl?raw'
export { MESH_SLICER_WGSL }

const slicerUniformBuffer = new ArrayBuffer(48)
const slicerUniformView = new DataView(slicerUniformBuffer)

function buildSlicerUniform(
  totalTriangles: number,
  numLayers: number,
  strideFloats: number,
  posOffset: number,
  planeNormal: [number, number, number],
  firstLayerDistance: number,
  layerStep: number,
  maxSegments: number
): ArrayBuffer {
  const n = normalizeVec3(planeNormal)

  slicerUniformView.setUint32(0, totalTriangles, true)
  slicerUniformView.setUint32(4, numLayers, true)
  slicerUniformView.setUint32(8, strideFloats, true)
  slicerUniformView.setUint32(12, posOffset, true)

  slicerUniformView.setFloat32(16, n[0], true)
  slicerUniformView.setFloat32(20, n[1], true)
  slicerUniformView.setFloat32(24, n[2], true)
  slicerUniformView.setFloat32(28, firstLayerDistance, true)

  slicerUniformView.setFloat32(32, layerStep, true)
  slicerUniformView.setUint32(36, maxSegments, true)
  slicerUniformView.setUint32(40, 0, true)
  slicerUniformView.setUint32(44, 0, true)

  return slicerUniformBuffer
}

function unpackSegments(raw: Float32Array, count: number): SliceSegment[] {
  const segments: SliceSegment[] = new Array(count)
  for (let i = 0; i < count; i++) {
    const off = i * 8
    segments[i] = {
      p0: [raw[off + 0]!, raw[off + 1]!, raw[off + 2]!],
      layerIndex: Math.round(raw[off + 3]!),
      p1: [raw[off + 4]!, raw[off + 5]!, raw[off + 6]!],
      triangleIndex: Math.round(raw[off + 7]!),
    }
  }
  return segments
}

/**
 * Slices a mesh across 1 or N parallel planes on WebGPU Compute.
 */
export async function runGpuMeshSlicer(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SlicerOptions
): Promise<SlicerResult> {
  const totalTriangles = Math.floor(indices.length / 3)
  const numLayers = Math.max(1, opts.numLayers ?? 1)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0
  const planeNormal = opts.planeNormal ?? [0, 0, 1]
  const layerStep = opts.layerStep ?? 0.2
  const maxSegments = opts.maxSegments ?? Math.max(1024, totalTriangles * Math.min(numLayers, 16))

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return totalTriangles >= 512
      ? referenceMeshSlicerAsync(vertices, indices, opts)
      : referenceMeshSlicer(vertices, indices, opts)
  }

  try {
    const uniformData = buildSlicerUniform(
      totalTriangles,
      numLayers,
      strideFloats,
      posOffset,
      planeNormal,
      opts.firstLayerDistance,
      layerStep,
      maxSegments
    )

    const counterInit = new Uint32Array([0])
    const segOutBytes = maxSegments * 8 * 4

    const results = await runGpuCompute({
      wgsl: MESH_SLICER_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformData, uniform: true },
            { binding: 1, data: vertices },
            { binding: 2, data: indices },
            { binding: 3, data: counterInit, output: true, outputBytes: 4 },
            { binding: 4, data: new Float32Array(0), output: true, outputBytes: segOutBytes },
          ],
          outputBytes: segOutBytes,
          workgroups: [Math.ceil(totalTriangles / 256), 1, 1],
        },
      ],
    })

    const count = Math.min(results[0]?.[0] ?? 0, maxSegments)
    const rawOut = results[1]
    if (rawOut) {
      const sliced = rawOut.slice(0, count * 8)
      return {
        rawSegments: sliced,
        segments: unpackSegments(sliced, count),
        segmentCount: count,
      }
    }
  } catch {
    // Fallback to CPU reference when WebGPU adapter is unavailable
  }

  return totalTriangles >= 512
    ? referenceMeshSlicerAsync(vertices, indices, opts)
    : referenceMeshSlicer(vertices, indices, opts)
}

function interpEdge(
  x0: number, y0: number, z0: number,
  s0: number,
  x1: number, y1: number, z1: number,
  s1: number
): [number, number, number] {
  const denom = s1 - s0
  if (Math.abs(denom) < 1e-9) {
    return [(x0 + x1) * 0.5, (y0 + y1) * 0.5, (z0 + z1) * 0.5]
  }
  const t = Math.max(0, Math.min(1, -s0 / denom))
  return [
    x0 + t * (x1 - x0),
    y0 + t * (y1 - y0),
    z0 + t * (z1 - z0),
  ]
}

function sliceSingleTriangle(
  t: number,
  vertices: Float32Array,
  indices: Uint32Array,
  strideFloats: number,
  posOffset: number,
  planeN: [number, number, number],
  firstD: number,
  layerStep: number,
  numLayers: number,
  rawList: number[],
  segments: SliceSegment[]
): void {
  const i0 = indices[t * 3 + 0]!
  const i1 = indices[t * 3 + 1]!
  const i2 = indices[t * 3 + 2]!

  const b0 = i0 * strideFloats + posOffset
  const b1 = i1 * strideFloats + posOffset
  const b2 = i2 * strideFloats + posOffset

  const p0x = vertices[b0]!, p0y = vertices[b0 + 1]!, p0z = vertices[b0 + 2]!
  const p1x = vertices[b1]!, p1y = vertices[b1 + 1]!, p1z = vertices[b1 + 2]!
  const p2x = vertices[b2]!, p2y = vertices[b2 + 1]!, p2z = vertices[b2 + 2]!

  const nx = planeN[0], ny = planeN[1], nz = planeN[2]
  const d0 = nx * p0x + ny * p0y + nz * p0z
  const d1 = nx * p1x + ny * p1y + nz * p1z
  const d2 = nx * p2x + ny * p2y + nz * p2z

  const dMin = Math.min(d0, d1, d2)
  const dMax = Math.max(d0, d1, d2)

  let lStart = 0
  let lEnd = numLayers - 1

  if (layerStep > 1e-7 && numLayers > 1) {
    lStart = Math.max(0, Math.ceil((dMin - firstD) / layerStep))
    lEnd = Math.min(numLayers - 1, Math.floor((dMax - firstD) / layerStep))
  } else {
    if (firstD < dMin || firstD > dMax) return
    lStart = 0
    lEnd = 0
  }

  if (lStart > lEnd) return

  const e1x = p1x - p0x, e1y = p1y - p0y, e1z = p1z - p0z
  const e2x = p2x - p0x, e2y = p2y - p0y, e2z = p2z - p0z
  const triNx = e1y * e2z - e1z * e2y
  const triNy = e1z * e2x - e1x * e2z
  const triNz = e1x * e2y - e1y * e2x
  const ccwX = ny * triNz - nz * triNy
  const ccwY = nz * triNx - nx * triNz
  const ccwZ = nx * triNy - ny * triNx

  for (let layer = lStart; layer <= lEnd; layer++) {
    const planeD = firstD + layer * layerStep
    const s0 = d0 - planeD
    const s1 = d1 - planeD
    const s2 = d2 - planeD

    const n0 = s0 < 0
    const n1 = s1 < 0
    const n2 = s2 < 0
    const negCount = (n0 ? 1 : 0) + (n1 ? 1 : 0) + (n2 ? 1 : 0)
    if (negCount === 0 || negCount === 3) continue

    let soloX: number, soloY: number, soloZ: number, soloS: number
    let pair0X: number, pair0Y: number, pair0Z: number, pair0S: number
    let pair1X: number, pair1Y: number, pair1Z: number, pair1S: number

    const soloIsNeg = negCount === 1
    if (n0 === soloIsNeg) {
      soloX = p0x; soloY = p0y; soloZ = p0z; soloS = s0
      pair0X = p1x; pair0Y = p1y; pair0Z = p1z; pair0S = s1
      pair1X = p2x; pair1Y = p2y; pair1Z = p2z; pair1S = s2
    } else if (n1 === soloIsNeg) {
      soloX = p1x; soloY = p1y; soloZ = p1z; soloS = s1
      pair0X = p0x; pair0Y = p0y; pair0Z = p0z; pair0S = s0
      pair1X = p2x; pair1Y = p2y; pair1Z = p2z; pair1S = s2
    } else {
      soloX = p2x; soloY = p2y; soloZ = p2z; soloS = s2
      pair0X = p0x; pair0Y = p0y; pair0Z = p0z; pair0S = s0
      pair1X = p1x; pair1Y = p1y; pair1Z = p1z; pair1S = s1
    }

    let q1 = interpEdge(soloX, soloY, soloZ, soloS, pair0X, pair0Y, pair0Z, pair0S)
    let q2 = interpEdge(soloX, soloY, soloZ, soloS, pair1X, pair1Y, pair1Z, pair1S)

    const dx = q2[0] - q1[0], dy = q2[1] - q1[1], dz = q2[2] - q1[2]
    if (ccwX * dx + ccwY * dy + ccwZ * dz < 0) {
      const tmp = q1
      q1 = q2
      q2 = tmp
    }

    if (dx * dx + dy * dy + dz * dz < 1e-14) continue

    rawList.push(q1[0], q1[1], q1[2], layer, q2[0], q2[1], q2[2], t)
    segments.push({
      p0: q1,
      p1: q2,
      layerIndex: layer,
      triangleIndex: t,
    })
  }
}

/**
 * Exact CPU reference implementation of multi-plane mesh slicing.
 */
export function referenceMeshSlicer(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SlicerOptions
): SlicerResult {
  const totalTriangles = Math.floor(indices.length / 3)
  const numLayers = Math.max(1, opts.numLayers ?? 1)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0
  const planeN = normalizeVec3(opts.planeNormal ?? [0, 0, 1])
  const firstD = opts.firstLayerDistance
  const layerStep = opts.layerStep ?? 0.2

  const rawList: number[] = []
  const segments: SliceSegment[] = []

  for (let t = 0; t < totalTriangles; t++) {
    sliceSingleTriangle(
      t,
      vertices,
      indices,
      strideFloats,
      posOffset,
      planeN,
      firstD,
      layerStep,
      numLayers,
      rawList,
      segments
    )
  }

  return {
    rawSegments: new Float32Array(rawList),
    segments,
    segmentCount: segments.length,
  }
}

/**
 * Cooperative non-blocking async CPU reference for multi-plane mesh slicing.
 */
export async function referenceMeshSlicerAsync(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SlicerOptions
): Promise<SlicerResult> {
  const totalTriangles = Math.floor(indices.length / 3)
  const numLayers = Math.max(1, opts.numLayers ?? 1)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0
  const planeN = normalizeVec3(opts.planeNormal ?? [0, 0, 1])
  const firstD = opts.firstLayerDistance
  const layerStep = opts.layerStep ?? 0.2

  const rawList: number[] = []
  const segments: SliceSegment[] = []

  await forEachChunkedAsync(
    totalTriangles,
    t => {
      sliceSingleTriangle(
        t,
        vertices,
        indices,
        strideFloats,
        posOffset,
        planeN,
        firstD,
        layerStep,
        numLayers,
        rawList,
        segments
      )
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
 * Computes the exact signed 2D cross-sectional area of a slice layer
 * via Green's / Stokes' theorem: Area = 0.5 * sum((q1 x q2) . planeNormal).
 */
export function computeSliceArea(
  segments: readonly SliceSegment[],
  planeNormal: [number, number, number] = [0, 0, 1],
  layerIndex?: number
): number {
  const n = normalizeVec3(planeNormal)
  let doubleArea = 0.0

  for (const seg of segments) {
    if (layerIndex !== undefined && seg.layerIndex !== layerIndex) continue
    const [x1, y1, z1] = seg.p0
    const [x2, y2, z2] = seg.p1

    // Cross product q1 x q2
    const cx = y1 * z2 - z1 * y2
    const cy = z1 * x2 - x1 * z2
    const cz = x1 * y2 - y1 * x2

    doubleArea += cx * n[0] + cy * n[1] + cz * n[2]
  }

  return 0.5 * doubleArea
}

/**
 * Groups emitted segments by their layer index.
 */
export function groupSegmentsByLayer(result: SlicerResult): Map<number, SliceSegment[]> {
  const map = new Map<number, SliceSegment[]>()
  for (const seg of result.segments) {
    let list = map.get(seg.layerIndex)
    if (!list) {
      list = []
      map.set(seg.layerIndex, list)
    }
    list.push(seg)
  }
  return map
}
