/**
 * WebGPU Compute NURBS & B-Spline Surface Tessellator.
 *
 * Evaluates tensor-product NURBS surfaces entirely on GPU Compute units:
 * 1. Parallel Cox-de Boor basis function and first-derivative evaluation.
 * 2. Homogeneous projection and exact analytical surface normal extraction.
 * 3. Water-tight vertex and triangle index generation directly in VRAM.
 */

import { runGpuCompute } from './webgpuCompute'
import { forEachChunkedAsync, type CooperativeAsyncOptions } from './cooperativeAsync'

export interface GpuNurbsSurface {
  degreeU: number
  degreeV: number
  knotsU: number[] | Float32Array
  knotsV: number[] | Float32Array
  /** Array of 3D control points [x, y, z] in row-major order: index = x + y * numCpU */
  controlPoints: Array<[number, number, number]> | Float32Array
  /** Optional weights for rational NURBS (default 1.0 for polynomial B-spline) */
  weights?: number[] | Float32Array
  numCpU: number
  numCpV: number
  domainU?: [number, number]
  domainV?: [number, number]
}

export interface GpuNurbsTessellationResult {
  /** Interleaved vertex buffer: stride 8 floats [x, y, z, nx, ny, nz, u, v] */
  vertices: Float32Array
  /** Triangle indices */
  indices: Uint32Array
  vertexCount: number
  triangleCount: number
}

import NURBS_TESSELLATE_WGSL from '../../crates/compute-core/shaders/nurbs_tessellate.wgsl?raw'
export { NURBS_TESSELLATE_WGSL }


const nurbsUniformBuffer = new ArrayBuffer(64)
const nurbsUniformView = new DataView(nurbsUniformBuffer)
const quadIndexCache = new Map<number, Uint32Array>()

function buildNurbsUniform(
  surface: GpuNurbsSurface,
  samplesU: number,
  samplesV: number
): ArrayBuffer {
  const ku = surface.knotsU
  const kv = surface.knotsV
  const uMin = surface.domainU?.[0] ?? ku[surface.degreeU] ?? 0
  const uMax = surface.domainU?.[1] ?? ku[surface.numCpU] ?? 1
  const vMin = surface.domainV?.[0] ?? kv[surface.degreeV] ?? 0
  const vMax = surface.domainV?.[1] ?? kv[surface.numCpV] ?? 1

  const isRational = surface.weights && surface.weights.length > 0 ? 1 : 0
  const totalVertices = samplesU * samplesV
  const totalQuads = (samplesU - 1) * (samplesV - 1)

  nurbsUniformView.setUint32(0, surface.degreeU, true)
  nurbsUniformView.setUint32(4, surface.degreeV, true)
  nurbsUniformView.setUint32(8, surface.numCpU, true)
  nurbsUniformView.setUint32(12, surface.numCpV, true)

  nurbsUniformView.setUint32(16, surface.knotsU.length, true)
  nurbsUniformView.setUint32(20, surface.knotsV.length, true)
  nurbsUniformView.setUint32(24, samplesU, true)
  nurbsUniformView.setUint32(28, samplesV, true)

  nurbsUniformView.setFloat32(32, uMin, true)
  nurbsUniformView.setFloat32(36, uMax, true)
  nurbsUniformView.setFloat32(40, vMin, true)
  nurbsUniformView.setFloat32(44, vMax, true)

  nurbsUniformView.setUint32(48, isRational, true)
  nurbsUniformView.setUint32(52, totalVertices, true)
  nurbsUniformView.setUint32(56, totalQuads, true)
  nurbsUniformView.setUint32(60, 0, true) // pad

  return nurbsUniformBuffer
}

function packControlPoints(surface: GpuNurbsSurface): Float32Array {
  const count = surface.numCpU * surface.numCpV
  const packed = new Float32Array(count * 4)

  const cp = surface.controlPoints
  const isFlat = cp instanceof Float32Array
  const weights = surface.weights

  for (let i = 0; i < count; i++) {
    let x = 0, y = 0, z = 0
    if (isFlat) {
      x = cp[i * 3 + 0] ?? 0
      y = cp[i * 3 + 1] ?? 0
      z = cp[i * 3 + 2] ?? 0
    } else {
      const pt = (cp as Array<[number, number, number]>)[i]
      if (pt) {
        x = pt[0]
        y = pt[1]
        z = pt[2]
      }
    }
    const w = weights ? (weights[i] ?? 1.0) : 1.0
    packed[i * 4 + 0] = x
    packed[i * 4 + 1] = y
    packed[i * 4 + 2] = z
    packed[i * 4 + 3] = w
  }

  return packed
}

/**
 * Evaluates a NURBS surface into a watertight triangle mesh using WebGPU Compute.
 */
export async function runGpuNurbsTessellate(
  surface: GpuNurbsSurface,
  samplesU = 32,
  samplesV = 32,
  options: CooperativeAsyncOptions = {}
): Promise<GpuNurbsTessellationResult> {
  const totalVertices = samplesU * samplesV
  const totalQuads = (samplesU - 1) * (samplesV - 1)
  const triangleCount = totalQuads * 2

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return totalVertices >= 1024
      ? referenceNurbsTessellateAsync(surface, samplesU, samplesV, options)
      : referenceNurbsTessellate(surface, samplesU, samplesV)
  }

  try {
    const uniformData = buildNurbsUniform(surface, samplesU, samplesV)
    const knotsU = surface.knotsU instanceof Float32Array ? surface.knotsU : new Float32Array(surface.knotsU)
    const knotsV = surface.knotsV instanceof Float32Array ? surface.knotsV : new Float32Array(surface.knotsV)
    const cpPacked = packControlPoints(surface)

    const vertOutputBytes = totalVertices * 8 * 4
    const indexOutputBytes = totalQuads * 6 * 4

    const results = await runGpuCompute({
      wgsl: NURBS_TESSELLATE_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformData, uniform: true },
            { binding: 1, data: knotsU },
            { binding: 2, data: knotsV },
            { binding: 3, data: cpPacked },
            { binding: 4, data: new Float32Array(0), output: true, outputBytes: vertOutputBytes },
            { binding: 5, data: new Uint32Array(0), output: true, outputBytes: indexOutputBytes },
          ],
          outputBytes: vertOutputBytes,
          workgroups: [Math.ceil(samplesU / 16), Math.ceil(samplesV / 16), 1],
        },
      ],
    })

    const rawVerts = results[0]
    const rawIndices = results[1]
    if (rawVerts && rawVerts.length >= totalVertices * 8) {
      const indexCount = totalQuads * 6
      const indices =
        rawIndices && rawIndices.byteLength >= indexOutputBytes
          ? new Uint32Array(rawIndices.buffer, rawIndices.byteOffset, indexCount).slice()
          : generateQuadIndices(samplesU, samplesV)
      return {
        vertices: rawVerts.slice(0, totalVertices * 8),
        indices,
        vertexCount: totalVertices,
        triangleCount,
      }
    }
  } catch {
    // Fallback if WebGPU compute fails or device is unsupported
  }

  return totalVertices >= 1024
    ? referenceNurbsTessellateAsync(surface, samplesU, samplesV, options)
    : referenceNurbsTessellate(surface, samplesU, samplesV)
}

/**
 * Fast quad index generator for a regular (samplesU x samplesV) parametric grid.
 */
export function generateQuadIndices(samplesU: number, samplesV: number): Uint32Array {
  const key = (samplesU << 16) | (samplesV & 0xffff)
  if (samplesU <= 256 && samplesV <= 256) {
    const cached = quadIndexCache.get(key)
    if (cached) return cached.slice()
  }

  const totalQuads = (samplesU - 1) * (samplesV - 1)
  const indices = new Uint32Array(totalQuads * 6)
  let ptr = 0

  for (let iv = 0; iv < samplesV - 1; iv++) {
    for (let iu = 0; iu < samplesU - 1; iu++) {
      const v00 = iu + iv * samplesU
      const v10 = (iu + 1) + iv * samplesU
      const v01 = iu + (iv + 1) * samplesU
      const v11 = (iu + 1) + (iv + 1) * samplesU

      // Tri 1: CCW
      indices[ptr++] = v00
      indices[ptr++] = v10
      indices[ptr++] = v01

      // Tri 2: CCW
      indices[ptr++] = v10
      indices[ptr++] = v11
      indices[ptr++] = v01
    }
  }

  if (samplesU <= 256 && samplesV <= 256) {
    if (quadIndexCache.size >= 32) {
      const firstKey = quadIndexCache.keys().next().value
      if (firstKey !== undefined) quadIndexCache.delete(firstKey)
    }
    quadIndexCache.set(key, indices)
    return indices.slice()
  }

  return indices
}

function findSpan(n: number, deg: number, u: number, knots: number[] | Float32Array): number {
  if (u >= knots[n + 1]!) return n
  if (u <= knots[deg]!) return deg
  let low = deg
  let high = n + 1
  let mid = (low + high) >> 1
  while (u < knots[mid]! || u >= knots[mid + 1]!) {
    if (u < knots[mid]!) high = mid
    else low = mid
    mid = (low + high) >> 1
  }
  return mid
}

function createBasisEvaluator(maxDeg: number) {
  const stride = maxDeg + 1
  const ndu = new Float64Array(stride * stride)
  const left = new Float64Array(stride)
  const right = new Float64Array(stride)

  return function evaluateBasisFunsDers(
    span: number,
    u: number,
    deg: number,
    knots: number[] | Float32Array,
    basisOut: Float64Array,
    dersOut: Float64Array
  ): void {
    ndu[0] = 1.0
    for (let j = 1; j <= deg; j++) {
      left[j] = u - knots[span + 1 - j]!
      right[j] = knots[span + j]! - u
      let saved = 0.0
      const jRow = j * stride
      for (let r = 0; r < j; r++) {
        const denom = right[r + 1]! + left[j - r]!
        ndu[jRow + r] = denom
        const rRow = r * stride
        const temp = Math.abs(denom) > 1e-12 ? ndu[rRow + (j - 1)]! / denom : 0.0
        ndu[rRow + j] = saved + right[r + 1]! * temp
        saved = left[j - r]! * temp
      }
      ndu[jRow + j] = saved
    }

    for (let j = 0; j <= deg; j++) {
      basisOut[j] = ndu[j * stride + deg]!
    }

    const degRow = deg * stride
    for (let r = 0; r <= deg; r++) {
      let d = 0.0
      if (r === 0) {
        const den = ndu[degRow]!
        if (Math.abs(den) > 1e-12) d = (-deg * ndu[deg - 1]!) / den
      } else if (r === deg) {
        const den = ndu[degRow + (deg - 1)]!
        if (Math.abs(den) > 1e-12) d = (deg * ndu[(deg - 1) * stride + (deg - 1)]!) / den
      } else {
        const den1 = ndu[degRow + (r - 1)]!
        const den2 = ndu[degRow + r]!
        const t1 = Math.abs(den1) > 1e-12 ? (deg * ndu[(r - 1) * stride + (deg - 1)]!) / den1 : 0.0
        const t2 = Math.abs(den2) > 1e-12 ? (deg * ndu[r * stride + (deg - 1)]!) / den2 : 0.0
        d = t1 - t2
      }
      dersOut[r] = d
    }
  }
}

/**
 * CPU-side reference evaluator for NURBS surface tessellation.
 * Used for mathematical verification, unit testing, and fallback.
 */
export function referenceNurbsTessellate(
  surface: GpuNurbsSurface,
  samplesU = 32,
  samplesV = 32
): GpuNurbsTessellationResult {
  const p = surface.degreeU
  const q = surface.degreeV
  const ku = surface.knotsU
  const kv = surface.knotsV
  const numCpU = surface.numCpU
  const numCpV = surface.numCpV

  const uMin = surface.domainU?.[0] ?? ku[p] ?? 0
  const uMax = surface.domainU?.[1] ?? ku[numCpU] ?? 1
  const vMin = surface.domainV?.[0] ?? kv[q] ?? 0
  const vMax = surface.domainV?.[1] ?? kv[numCpV] ?? 1

  const isRational = Boolean(surface.weights && surface.weights.length > 0)
  const cp = surface.controlPoints
  const isFlatCp = cp instanceof Float32Array
  const weights = surface.weights

  const evaluateBasis = createBasisEvaluator(Math.max(p, q))
  const buBasis = new Float64Array(p + 1)
  const buDers = new Float64Array(p + 1)
  const bvBasis = new Float64Array(q + 1)
  const bvDers = new Float64Array(q + 1)

  const totalVertices = samplesU * samplesV
  const vertices = new Float32Array(totalVertices * 8)

  for (let iv = 0; iv < samplesV; iv++) {
    const tv = iv / (samplesV - 1)
    const v = vMin + tv * (vMax - vMin)
    const spanV = findSpan(numCpV - 1, q, v, kv)
    evaluateBasis(spanV, v, q, kv, bvBasis, bvDers)

    for (let iu = 0; iu < samplesU; iu++) {
      const tu = iu / (samplesU - 1)
      const u = uMin + tu * (uMax - uMin)
      const spanU = findSpan(numCpU - 1, p, u, ku)
      evaluateBasis(spanU, u, p, ku, buBasis, buDers)

      let ptX = 0, ptY = 0, ptZ = 0, ptW = 0
      let duX = 0, duY = 0, duZ = 0, duW = 0
      let dvX = 0, dvY = 0, dvZ = 0, dvW = 0

      for (let r = 0; r <= p; r++) {
        const cpx = spanU - p + r
        const nu = buBasis[r]!
        const dnu = buDers[r]!

        for (let s = 0; s <= q; s++) {
          const cpy = spanV - q + s
          const mv = bvBasis[s]!
          const dmv = bvDers[s]!

          const idx = cpx + cpy * numCpU
          let cx = 0, cy = 0, cz = 0
          if (isFlatCp) {
            const base = idx * 3
            cx = cp[base] ?? 0
            cy = cp[base + 1] ?? 0
            cz = cp[base + 2] ?? 0
          } else {
            const pt = (cp as Array<[number, number, number]>)[idx]
            if (pt) { cx = pt[0]; cy = pt[1]; cz = pt[2] }
          }
          const cw = weights ? (weights[idx] ?? 1.0) : 1.0

          if (isRational) {
            cx *= cw
            cy *= cw
            cz *= cw
          }

          const wpt = nu * mv
          const wdu = dnu * mv
          const wdv = nu * dmv

          ptX += cx * wpt; ptY += cy * wpt; ptZ += cz * wpt; ptW += cw * wpt
          duX += cx * wdu; duY += cy * wdu; duZ += cz * wdu; duW += cw * wdu
          dvX += cx * wdv; dvY += cy * wdv; dvZ += cz * wdv; dvW += cw * wdv
        }
      }

      let posX = ptX, posY = ptY, posZ = ptZ
      let tanUX = duX, tanUY = duY, tanUZ = duZ
      let tanVX = dvX, tanVY = dvY, tanVZ = dvZ

      if (isRational && Math.abs(ptW) > 1e-12) {
        const invW = 1.0 / ptW
        posX = ptX * invW
        posY = ptY * invW
        posZ = ptZ * invW
        tanUX = (duX - posX * duW) * invW
        tanUY = (duY - posY * duW) * invW
        tanUZ = (duZ - posZ * duW) * invW
        tanVX = (dvX - posX * dvW) * invW
        tanVY = (dvY - posY * dvW) * invW
        tanVZ = (dvZ - posZ * dvW) * invW
      }

      let nx = tanUY * tanVZ - tanUZ * tanVY
      let ny = tanUZ * tanVX - tanUX * tanVZ
      let nz = tanUX * tanVY - tanUY * tanVX
      const len = Math.hypot(nx, ny, nz)
      if (len > 1e-7) {
        const invLen = 1.0 / len
        nx *= invLen; ny *= invLen; nz *= invLen
      } else {
        nx = 0; ny = 0; nz = 1
      }

      const vIdx = iu + iv * samplesU
      const off = vIdx * 8
      vertices[off + 0] = posX
      vertices[off + 1] = posY
      vertices[off + 2] = posZ
      vertices[off + 3] = nx
      vertices[off + 4] = ny
      vertices[off + 5] = nz
      vertices[off + 6] = tu
      vertices[off + 7] = tv
    }
  }

  const indices = generateQuadIndices(samplesU, samplesV)

  return {
    vertices,
    indices,
    vertexCount: totalVertices,
    triangleCount: (samplesU - 1) * (samplesV - 1) * 2,
  }
}

/**
 * Cooperative non-blocking async CPU reference evaluator for NURBS surface tessellation.
 */
export async function referenceNurbsTessellateAsync(
  surface: GpuNurbsSurface,
  samplesU = 32,
  samplesV = 32,
  options: CooperativeAsyncOptions = {}
): Promise<GpuNurbsTessellationResult> {
  const p = surface.degreeU
  const q = surface.degreeV
  const ku = surface.knotsU
  const kv = surface.knotsV
  const numCpU = surface.numCpU
  const numCpV = surface.numCpV

  const uMin = surface.domainU?.[0] ?? ku[p] ?? 0
  const uMax = surface.domainU?.[1] ?? ku[numCpU] ?? 1
  const vMin = surface.domainV?.[0] ?? kv[q] ?? 0
  const vMax = surface.domainV?.[1] ?? kv[numCpV] ?? 1

  const isRational = Boolean(surface.weights && surface.weights.length > 0)
  const cp = surface.controlPoints
  const isFlatCp = cp instanceof Float32Array
  const weights = surface.weights

  const evaluateBasis = createBasisEvaluator(Math.max(p, q))
  const buBasis = new Float64Array(p + 1)
  const buDers = new Float64Array(p + 1)
  const bvBasis = new Float64Array(q + 1)
  const bvDers = new Float64Array(q + 1)

  const totalVertices = samplesU * samplesV
  const vertices = new Float32Array(totalVertices * 8)

  await forEachChunkedAsync(
    samplesV,
    iv => {
      const tv = iv / (samplesV - 1)
      const v = vMin + tv * (vMax - vMin)
      const spanV = findSpan(numCpV - 1, q, v, kv)
      evaluateBasis(spanV, v, q, kv, bvBasis, bvDers)

      for (let iu = 0; iu < samplesU; iu++) {
        const tu = iu / (samplesU - 1)
        const u = uMin + tu * (uMax - uMin)
        const spanU = findSpan(numCpU - 1, p, u, ku)
        evaluateBasis(spanU, u, p, ku, buBasis, buDers)

        let ptX = 0, ptY = 0, ptZ = 0, ptW = 0
        let duX = 0, duY = 0, duZ = 0, duW = 0
        let dvX = 0, dvY = 0, dvZ = 0, dvW = 0

        for (let r = 0; r <= p; r++) {
          const cpx = spanU - p + r
          const nu = buBasis[r]!
          const dnu = buDers[r]!

          for (let s = 0; s <= q; s++) {
            const cpy = spanV - q + s
            const mv = bvBasis[s]!
            const dmv = bvDers[s]!

            const idx = cpx + cpy * numCpU
            let cx = 0, cy = 0, cz = 0
            if (isFlatCp) {
              const base = idx * 3
              cx = cp[base] ?? 0
              cy = cp[base + 1] ?? 0
              cz = cp[base + 2] ?? 0
            } else {
              const pt = (cp as Array<[number, number, number]>)[idx]
              if (pt) { cx = pt[0]; cy = pt[1]; cz = pt[2] }
            }
            const cw = weights ? (weights[idx] ?? 1.0) : 1.0

            if (isRational) {
              cx *= cw
              cy *= cw
              cz *= cw
            }

            const wpt = nu * mv
            const wdu = dnu * mv
            const wdv = nu * dmv

            ptX += cx * wpt; ptY += cy * wpt; ptZ += cz * wpt; ptW += cw * wpt
            duX += cx * wdu; duY += cy * wdu; duZ += cz * wdu; duW += cw * wdu
            dvX += cx * wdv; dvY += cy * wdv; dvZ += cz * wdv; dvW += cw * wdv
          }
        }

        let posX = ptX, posY = ptY, posZ = ptZ
        let tanUX = duX, tanUY = duY, tanUZ = duZ
        let tanVX = dvX, tanVY = dvY, tanVZ = dvZ

        if (isRational && Math.abs(ptW) > 1e-12) {
          const invW = 1.0 / ptW
          posX = ptX * invW
          posY = ptY * invW
          posZ = ptZ * invW
          tanUX = (duX - posX * duW) * invW
          tanUY = (duY - posY * duW) * invW
          tanUZ = (duZ - posZ * duW) * invW
          tanVX = (dvX - posX * dvW) * invW
          tanVY = (dvY - posY * dvW) * invW
          tanVZ = (dvZ - posZ * dvW) * invW
        }

        let nx = tanUY * tanVZ - tanUZ * tanVY
        let ny = tanUZ * tanVX - tanUX * tanVZ
        let nz = tanUX * tanVY - tanUY * tanVX
        const len = Math.hypot(nx, ny, nz)
        if (len > 1e-7) {
          const invLen = 1.0 / len
          nx *= invLen; ny *= invLen; nz *= invLen
        } else {
          nx = 0; ny = 0; nz = 1
        }

        const vIdx = iu + iv * samplesU
        const off = vIdx * 8
        vertices[off + 0] = posX
        vertices[off + 1] = posY
        vertices[off + 2] = posZ
        vertices[off + 3] = nx
        vertices[off + 4] = ny
        vertices[off + 5] = nz
        vertices[off + 6] = tu
        vertices[off + 7] = tv
      }
    },
    { ...options, checkStride: Math.max(1, Math.floor(256 / Math.max(1, samplesU))) }
  )

  const indices = generateQuadIndices(samplesU, samplesV)

  return {
    vertices,
    indices,
    vertexCount: totalVertices,
    triangleCount: (samplesU - 1) * (samplesV - 1) * 2,
  }
}

/**
 * Creates a flat bilinear patch (degree 1x1).
 */
export function createBilinearNurbsPatch(
  p00: [number, number, number],
  p10: [number, number, number],
  p01: [number, number, number],
  p11: [number, number, number]
): GpuNurbsSurface {
  return {
    degreeU: 1,
    degreeV: 1,
    knotsU: [0, 0, 1, 1],
    knotsV: [0, 0, 1, 1],
    controlPoints: [p00, p10, p01, p11],
    numCpU: 2,
    numCpV: 2,
  }
}

/**
 * Creates an exact rational 90-degree cylindrical NURBS patch.
 */
export function createCylinderQuarterNurbsPatch(radius = 1.0, height = 2.0): GpuNurbsSurface {
  // Quadratic in U (quarter circle: w = [1, cos(45°), 1] = [1, sqrt(2)/2, 1])
  // Linear in V (height)
  const w1 = Math.SQRT1_2
  return {
    degreeU: 2,
    degreeV: 1,
    knotsU: [0, 0, 0, 1, 1, 1],
    knotsV: [0, 0, 1, 1],
    numCpU: 3,
    numCpV: 2,
    controlPoints: [
      // v = 0 (z = 0)
      [radius, 0, 0],
      [radius, radius, 0],
      [0, radius, 0],
      // v = 1 (z = height)
      [radius, 0, height],
      [radius, radius, height],
      [0, radius, height],
    ],
    weights: [
      1.0, w1, 1.0,
      1.0, w1, 1.0,
    ],
  }
}
