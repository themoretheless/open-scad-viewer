/** Deterministic CPU NURBS tessellation reference and shared grid helpers (WebGPU fallback). */
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

const quadIndexCache = new Map<number, Uint32Array>()

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
