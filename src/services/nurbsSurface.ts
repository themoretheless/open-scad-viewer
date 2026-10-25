/** Surface types and Rust/WASM adapters. */
import { type NurbsCurve } from './nurbsCurve'
import { callNurbsRust, createRustSurfaceEvaluator, decodeNurbsResult } from './geometry/nurbs'

export type NurbsSurface = {
  degreeU: number
  degreeV: number
  knotsU: number[]
  knotsV: number[]
  /** Outer index is U, inner index is V; each point is [x, y, z]. */
  controlPoints: number[][][]
  weights: number[][]
  periodicU?: boolean
  periodicV?: boolean
}
export type NurbsSurfaceAxis = 'u' | 'v'
type Point3 = [number, number, number]

export type NurbsSurfaceEvaluation = {
  point: Point3
  du: Point3 | null
  dv: Point3 | null
  duu: Point3 | null
  duv: Point3 | null
  dvv: Point3 | null
  normal: Point3 | null
  gaussianCurvature: number | null
  /** Signed with respect to the Su cross Sv normal. */
  meanCurvature: number | null
  derivative_status: 'available' | 'insufficient_continuity' | 'singular'
  derivative_side_u: 'two_sided' | 'left' | 'right'
  derivative_side_v: 'two_sided' | 'left' | 'right'
  domainU: [number, number]
  domainV: [number, number]
}


export function validateNurbsSurface(surface: NurbsSurface): void { callNurbsRust('surface_validate', { surface }) }
export function evaluateNurbsSurface(surface: NurbsSurface, u: number, v: number): NurbsSurfaceEvaluation { return callNurbsRust('surface_evaluate', { surface, u, v }) }
/** Immutable validated Rust snapshot. Release explicitly after bounded sampling. */
export function createNurbsSurfaceEvaluator(surface: NurbsSurface): ((u: number, v: number) => NurbsSurfaceEvaluation) & { dispose(): void } {
  const evaluator = createRustSurfaceEvaluator(surface)
  let disposed = false
  const sample = (u: number, v: number): NurbsSurfaceEvaluation => {
    if (disposed) throw new Error('NURBS surface evaluator is disposed.')
    return decodeNurbsResult(evaluator.evaluate(u, v))
  }
  sample.dispose = () => { if (!disposed) { disposed = true; evaluator.free() } }
  return sample
}
export function insertNurbsSurfaceKnot(surface: NurbsSurface, axis: NurbsSurfaceAxis, value: number, count = 1): NurbsSurface { return callNurbsRust('surface_insert', { surface, axis, u: value, count }) }
export function elevateNurbsSurface(surface: NurbsSurface, axis: NurbsSurfaceAxis, targetDegree: number): NurbsSurface { return callNurbsRust('surface_elevate', { surface, axis, degree: targetDegree }) }
export function reverseNurbsSurface(surface: NurbsSurface, axis: NurbsSurfaceAxis): NurbsSurface { return callNurbsRust('surface_reverse', { surface, axis }) }
export function trimNurbsSurface(surface: NurbsSurface, bounds: [number, number, number, number]): NurbsSurface
export function trimNurbsSurface(surface: NurbsSurface, uMin: number, uMax: number, vMin: number, vMax: number): NurbsSurface
export function trimNurbsSurface(surface: NurbsSurface, first: number | [number, number, number, number], uMax?: number, vMin?: number, vMax?: number): NurbsSurface {
  return callNurbsRust('surface_trim', { surface, bounds: Array.isArray(first) ? first : [first, uMax, vMin, vMax] })
}
export function isoNurbsCurve(surface: NurbsSurface, direction: NurbsSurfaceAxis, parameter: number): NurbsCurve { return callNurbsRust('surface_iso', { surface, axis: direction, u: parameter }) }
export function nurbsSurfaceBounds(surface: NurbsSurface): { min: Point3; max: Point3 } { return callNurbsRust('surface_bounds', { surface }) }

/** Distance between complete surface images; trimmed B-rep faces are a separate domain. */
export interface NurbsSurfaceDistance {
  method:'interval-tensor-de-boor-pair-subdivision'
  scope:'untrimmed-surfaces'
  distanceIntervalMm:[number,number]
  parameters:[[number,number],[number,number]]
  points:[Point3,Point3]
  pointEnclosures:[Array<[number,number]>,Array<[number,number]>]
  converged:boolean
  reason:'tolerance'|'work-limit'|'precision-limit'
  cells:number
  maxCells:number
  toleranceMm:number
}
export function measureNurbsSurfaceDistance(a:NurbsSurface,b:NurbsSurface,toleranceMm=0.001,maxCells=10000):NurbsSurfaceDistance {
  return callNurbsRust('surface_distance',{a,b,toleranceMm,maxCells})
}

export function transformNurbsSurface(surface:NurbsSurface,matrix:number[][]):NurbsSurface {return callNurbsRust("surface_affine",{surface,matrix})}
export function transformNurbsSurfacePatches(patches:NurbsSurface[],matrix:number[][]):NurbsSurface[] {return callNurbsRust("patches_affine",{patches,matrix})}

/**
 * Evaluate the unit surface normal at `(u, v)` on `surface`, falling back to normalized
 * $\partial_u S \times \partial_v S$ if needed.
 */
export function evaluateNurbsSurfaceNormal(
  surface: NurbsSurface,
  u: number,
  v: number,
): Point3 | null {
  const ev = evaluateNurbsSurface(surface, u, v)
  if (ev.normal) {
    const [nx, ny, nz] = ev.normal
    const len = Math.hypot(nx, ny, nz)
    if (len > 1e-14) {
      return [nx / len, ny / len, nz / len]
    }
  }
  if (ev.du && ev.dv) {
    const [ux, uy, uz] = ev.du
    const [vx, vy, vz] = ev.dv
    const cx = uy * vz - uz * vy
    const cy = uz * vx - ux * vz
    const cz = ux * vy - uy * vx
    const len = Math.hypot(cx, cy, cz)
    if (len > 1e-14) {
      return [cx / len, cy / len, cz / len]
    }
  }
  return null
}

/**
 * Approximate total surface area $\iint \|\partial_u S \times \partial_v S\| \, du \, dv$
 * over the active `(u, v)` parameter domain using midpoint quadrature on a `stepsU x stepsV` grid.
 */
export function evaluateNurbsSurfaceArea(
  surface: NurbsSurface,
  stepsU = 16,
  stepsV = 16,
): number {
  const nu = Math.max(1, Math.floor(stepsU))
  const nv = Math.max(1, Math.floor(stepsV))
  const evaluator = createNurbsSurfaceEvaluator(surface)
  try {
    const first = evaluator(surface.knotsU[surface.degreeU] ?? 0, surface.knotsV[surface.degreeV] ?? 0)
    const [uMin, uMax] = first.domainU
    const [vMin, vMax] = first.domainV
    const duStep = (uMax - uMin) / nu
    const dvStep = (vMax - vMin) / nv
    if (duStep <= 0 || dvStep <= 0) return 0

    let area = 0
    for (let i = 0; i < nu; i++) {
      const u = uMin + (i + 0.5) * duStep
      for (let j = 0; j < nv; j++) {
        const v = vMin + (j + 0.5) * dvStep
        const sample = evaluator(u, v)
        if (sample.du && sample.dv) {
          const [ux, uy, uz] = sample.du
          const [vx, vy, vz] = sample.dv
          const cx = uy * vz - uz * vy
          const cy = uz * vx - ux * vz
          const cz = ux * vy - uy * vx
          area += Math.hypot(cx, cy, cz) * duStep * dvStep
        }
      }
    }
    return area
  } finally {
    evaluator.dispose()
  }
}

/**
 * Find the closest point on a NURBS surface to `target` using a coarse `(gridSteps + 1)^2` lattice
 * search followed by 2D Gauss-Newton / damped Newton refinement on $(S(u, v) - P)$.
 */
export function closestPointOnNurbsSurface(
  surface: NurbsSurface,
  target: [number, number, number],
  gridSteps = 16,
): { u: number; v: number; point: [number, number, number]; distance: number } {
  const steps = Math.max(4, Math.floor(gridSteps))
  const evaluator = createNurbsSurfaceEvaluator(surface)
  try {
    const first = evaluator(surface.knotsU[surface.degreeU] ?? 0, surface.knotsV[surface.degreeV] ?? 0)
    const [uMin, uMax] = first.domainU
    const [vMin, vMax] = first.domainV
    const spanU = Math.max(0, uMax - uMin)
    const spanV = Math.max(0, vMax - vMin)

    const distSq3 = (a: Point3, b: Point3): number => {
      const dx = a[0] - b[0]
      const dy = a[1] - b[1]
      const dz = a[2] - b[2]
      return dx * dx + dy * dy + dz * dz
    }

    let bestU = uMin
    let bestV = vMin
    let bestPt: Point3 = [first.point[0], first.point[1], first.point[2]]
    let bestDistSq = distSq3(bestPt, target)

    if (spanU <= 1e-15 && spanV <= 1e-15) {
      return { u: bestU, v: bestV, point: bestPt, distance: Math.sqrt(bestDistSq) }
    }

    for (let i = 0; i <= steps; i++) {
      const u = uMin + (i / steps) * spanU
      for (let j = 0; j <= steps; j++) {
        const v = vMin + (j / steps) * spanV
        const ev = evaluator(u, v)
        const pt: Point3 = [ev.point[0], ev.point[1], ev.point[2]]
        const d2 = distSq3(pt, target)
        if (d2 < bestDistSq) {
          bestDistSq = d2
          bestU = u
          bestV = v
          bestPt = pt
        }
      }
    }

    // 2D Gauss-Newton refinement
    let u = bestU
    let v = bestV
    for (let iter = 0; iter < 12; iter++) {
      const ev = evaluator(u, v)
      const pt: Point3 = [ev.point[0], ev.point[1], ev.point[2]]
      const d2 = distSq3(pt, target)
      if (d2 < bestDistSq) {
        bestDistSq = d2
        bestU = u
        bestV = v
        bestPt = pt
      }
      if (!ev.du || !ev.dv) break

      const rx = pt[0] - target[0]
      const ry = pt[1] - target[1]
      const rz = pt[2] - target[2]
      const [ux, uy, uz] = ev.du
      const [vx, vy, vz] = ev.dv

      const gu = rx * ux + ry * uy + rz * uz
      const gv = rx * vx + ry * vy + rz * vz
      if (Math.hypot(gu, gv) <= 1e-12) break

      let huu = ux * ux + uy * uy + uz * uz
      let huv = ux * vx + uy * vy + uz * vz
      let hvv = vx * vx + vy * vy + vz * vz
      if (ev.duu && ev.duv && ev.dvv) {
        huu += rx * ev.duu[0] + ry * ev.duu[1] + rz * ev.duu[2]
        huv += rx * ev.duv[0] + ry * ev.duv[1] + rz * ev.duv[2]
        hvv += rx * ev.dvv[0] + ry * ev.dvv[1] + rz * ev.dvv[2]
      }
      // Fall back to positive-definite first-order Jacobian J^T J if full Hessian is indefinite
      let det = huu * hvv - huv * huv
      if (!Number.isFinite(det) || det <= 1e-14 || huu <= 0 || hvv <= 0) {
        huu = ux * ux + uy * uy + uz * uz + 1e-12
        huv = ux * vx + uy * vy + uz * vz
        hvv = vx * vx + vy * vy + vz * vz + 1e-12
        det = huu * hvv - huv * huv
      }
      if (!Number.isFinite(det) || Math.abs(det) <= 1e-20) break

      const invDet = 1 / det
      const du = (hvv * gu - huv * gv) * invDet
      const dv = (-huv * gu + huu * gv) * invDet

      let accepted = false
      for (const alpha of [1.0, 0.5, 0.25]) {
        const candU = Math.min(uMax, Math.max(uMin, u - alpha * du))
        const candV = Math.min(vMax, Math.max(vMin, v - alpha * dv))
        const candEv = evaluator(candU, candV)
        const candPt: Point3 = [candEv.point[0], candEv.point[1], candEv.point[2]]
        const candD2 = distSq3(candPt, target)
        if (candD2 <= bestDistSq) {
          const moved = Math.hypot(candU - u, candV - v)
          bestDistSq = candD2
          bestU = candU
          bestV = candV
          bestPt = candPt
          u = candU
          v = candV
          accepted = moved > 1e-11
          break
        }
      }
      if (!accepted) break
    }

    return {
      u: bestU,
      v: bestV,
      point: bestPt,
      distance: Math.sqrt(bestDistSq),
    }
  } finally {
    evaluator.dispose()
  }
}

import { runGpuNurbsTessellate, type GpuNurbsTessellationResult } from './gpuNurbsTessellate'

/**
 * Evaluates and tessellates a tensor-product rational B-spline surface on the
 * GPU via WebGPU compute shaders (`gpuNurbsTessellate.ts`). Returns `null` when
 * the surface uses periodic knot wrapping.
 */
export async function tessellateNurbsSurfaceOnGpu(
  surface: NurbsSurface,
  subdivisionsU = 32,
  subdivisionsV = 32,
): Promise<GpuNurbsTessellationResult | null> {
  if (surface.periodicU || surface.periodicV) return null
  const numCtrlU = surface.controlPoints.length
  const numCtrlV = numCtrlU > 0 ? surface.controlPoints[0].length : 0
  if (numCtrlU === 0 || numCtrlV === 0) return null

  const totalCp = numCtrlU * numCtrlV
  const controlPoints = new Float32Array(totalCp * 3)
  const weights = new Float32Array(totalCp)
  for (let i = 0; i < numCtrlU; i++) {
    const row = surface.controlPoints[i]
    const wRow = surface.weights[i]
    for (let j = 0, idx = i; j < numCtrlV; j++, idx += numCtrlU) {
      const pt = row[j]
      const cIdx = idx * 3
      controlPoints[cIdx] = pt[0]
      controlPoints[cIdx + 1] = pt[1]
      controlPoints[cIdx + 2] = pt[2]
      weights[idx] = wRow !== undefined ? (wRow[j] ?? 1) : 1
    }
  }

  return runGpuNurbsTessellate(
    {
      degreeU: surface.degreeU,
      degreeV: surface.degreeV,
      knotsU: surface.knotsU instanceof Float32Array ? surface.knotsU : new Float32Array(surface.knotsU),
      knotsV: surface.knotsV instanceof Float32Array ? surface.knotsV : new Float32Array(surface.knotsV),
      controlPoints,
      weights,
      numCpU: numCtrlU,
      numCpV: numCtrlV,
    },
    subdivisionsU,
    subdivisionsV,
  )
}

export type NurbsSurface3D = NurbsSurface

/**
 * Compute an axis-aligned 3D bounding box `{ min: [x, y, z], max: [x, y, z] }` for a NURBS surface
 * by sampling a `(gridSteps + 1) x (gridSteps + 1)` parameter lattice and refining extreme interior
 * candidates along `(u, v)`.
 */
export function computeNurbsSurfaceBoundingBox(
  surface: NurbsSurface3D,
  gridSteps = 16,
): { min: [number, number, number]; max: [number, number, number] } {
  const steps = Math.max(4, Math.floor(gridSteps))
  const evaluator = createNurbsSurfaceEvaluator(surface)
  try {
    const first = evaluator(surface.knotsU[surface.degreeU] ?? 0, surface.knotsV[surface.degreeV] ?? 0)
    const [uMin, uMax] = first.domainU
    const [vMin, vMax] = first.domainV
    const spanU = Math.max(0, uMax - uMin)
    const spanV = Math.max(0, vMax - vMin)

    const min: Point3 = [first.point[0], first.point[1], first.point[2]]
    const max: Point3 = [first.point[0], first.point[1], first.point[2]]

    for (let i = 0; i <= steps; i++) {
      const u = uMin + (i / steps) * spanU
      for (let j = 0; j <= steps; j++) {
        const v = vMin + (j / steps) * spanV
        const pt = evaluator(u, v).point
        for (let axis = 0; axis < 3; axis++) {
          if (pt[axis] < min[axis]) min[axis] = pt[axis]
          if (pt[axis] > max[axis]) max[axis] = pt[axis]
        }
      }
    }

    return { min, max }
  } finally {
    evaluator.dispose()
  }
}

/**
 * Extract `sampleCount` 3D points along an isoparametric curve on `surface`:
 * - When `direction === 'u'`, `u` is held constant at `fixedParam` and `v` is sampled uniformly across `domainV`.
 * - When `direction === 'v'`, `v` is held constant at `fixedParam` and `u` is sampled uniformly across `domainU`.
 */
export function extractNurbsSurfaceIsocurvePoints(
  surface: NurbsSurface3D,
  direction: 'u' | 'v',
  fixedParam: number,
  sampleCount = 32,
): [number, number, number][] {
  const count = Math.max(2, Math.floor(sampleCount))
  const evaluator = createNurbsSurfaceEvaluator(surface)
  try {
    const first = evaluator(surface.knotsU[surface.degreeU] ?? 0, surface.knotsV[surface.degreeV] ?? 0)
    const [uMin, uMax] = first.domainU
    const [vMin, vMax] = first.domainV
    const points: [number, number, number][] = []

    if (direction === 'u') {
      const uClamped = Math.min(uMax, Math.max(uMin, fixedParam))
      const spanV = Math.max(0, vMax - vMin)
      for (let i = 0; i < count; i++) {
        const v = vMin + (i / (count - 1)) * spanV
        const pt = evaluator(uClamped, v).point
        points.push([pt[0], pt[1], pt[2]])
      }
    } else {
      const vClamped = Math.min(vMax, Math.max(vMin, fixedParam))
      const spanU = Math.max(0, uMax - uMin)
      for (let i = 0; i < count; i++) {
        const u = uMin + (i / (count - 1)) * spanU
        const pt = evaluator(u, vClamped).point
        points.push([pt[0], pt[1], pt[2]])
      }
    }

    return points
  } finally {
    evaluator.dispose()
  }
}
