/** Public curve types and Rust/WASM adapters. All spline mathematics lives in crates/nurbs-core. */
import { callNurbsRust } from './geometry/nurbs'
export { NurbsCurveError } from './geometry/nurbs'

export interface NurbsCurve {
  degree: number
  knots: number[]
  controlPoints: number[][]
  weights: number[]
  /**
   * A periodic curve explicitly includes its p wrapped control points, weights,
   * and exterior knots. The last p controls equal the first p; knot intervals
   * repeat with the period knots[N]-knots[p]. A mere closed endpoint is not a
   * periodic representation. Bounded edits clamp one period and clear this flag.
   */
  periodic?: boolean
}

export type NurbsDerivativeStatus = 'available' | 'insufficient_continuity'
export type NurbsDerivativeSide = 'two_sided' | 'right' | 'left'

export interface NurbsBasisEvaluation {
  basis: number[]
  d1: number[]
  d2: number[]
  domain: [number, number]
  /** Null means that the parameter is inside a smooth polynomial span. */
  continuity: number | null
  derivative_status: NurbsDerivativeStatus
  derivative_side: NurbsDerivativeSide
}

export interface NurbsCurveEvaluation {
  point: number[]
  d1: number[] | null
  d2: number[] | null
  domain: [number, number]
  continuity: number | null
  derivative_status: NurbsDerivativeStatus
  derivative_side: NurbsDerivativeSide
}


export function validateNurbsCurve(curve: NurbsCurve): void { callNurbsRust('curve_validate', { curve }) }
export function nurbsBasisDerivatives(degree: number, knots: number[], controlCount: number, u: number, periodic = false): NurbsBasisEvaluation {
  return callNurbsRust('basis', { degree, knots, controlCount, u, periodic })
}
export function evaluateNurbsCurve(curve: NurbsCurve, u: number): NurbsCurveEvaluation { return callNurbsRust('curve_evaluate', { curve, u }) }
export function evaluateNurbsCurvePoints(curve: NurbsCurve, parameters: readonly number[]): number[][] { return callNurbsRust('curve_evaluate_points', { curve, parameters }) }
export function sampleNurbsCurve(curve: NurbsCurve, segments: number, includeEnd = true): number[][] { return callNurbsRust('curve_sample', { curve, segments, includeEnd }) }
export function insertNurbsKnot(curve: NurbsCurve, u: number, count = 1): NurbsCurve { return callNurbsRust('curve_insert', { curve, u, count }) }
export interface NurbsPointTrimResult {
  curve:NurbsCurve
  parameter:number
  parameterInterval:[number,number]
  point:number[]
  distanceUpperMm:number
  keptDomain:[number,number]
  projection:import('./nurbsFoundation').NurbsProjectionCertificate
}
export type NurbsScreenProjection = [[number,number,number,number],[number,number,number,number]]
/** Projection-certificate distances use CSS pixels; the returned point and curve stay in model coordinates. */
export function trimNurbsCurveAtScreenPoint(curve:NurbsCurve,point:[number,number],matrix:NurbsScreenProjection,keep:'start'|'end',radius:number):Omit<NurbsPointTrimResult,'distanceUpperMm'>&{distanceUpperPx:number;projectionSpace:'css-pixels'} {
  return callNurbsRust('curve_trim_screen_point',{curve,point,matrix,keep,radius})
}
export function trimNurbsCurveAtPoint(curve:NurbsCurve,point:number[],keep:'start'|'end',maxDistance:number):NurbsPointTrimResult {
  return callNurbsRust('curve_trim_point',{curve,point,keep,maxDistance})
}
export function trimNurbsCurve(curve: NurbsCurve, a: number, b: number): NurbsCurve { return callNurbsRust('curve_trim', { curve, a, b }) }
export function splitNurbsCurve(curve: NurbsCurve, u: number): [NurbsCurve, NurbsCurve] { return callNurbsRust('curve_split', { curve, u }) }
export function reverseNurbsCurve(curve: NurbsCurve): NurbsCurve { return callNurbsRust('curve_reverse', { curve }) }
export function decomposeNurbsCurve(curve: NurbsCurve): { curve: NurbsCurve; domain: [number, number] }[] { return callNurbsRust('curve_decompose', { curve }) }
/** Conditional parameterwise original-span/retained-Bezier error. Ownership
 * and complete wall/cap coverage are separate; exhaustion has no partial bound. */
export interface NurbsDecompositionAudit {
 errorUpper:number|null;products:number;reason:string|null
 method:'original-span-bernstein-decomposition';continuousBound:false
}
export function inspectNurbsDecomposition(curve:NurbsCurve,span:number,retained:NurbsCurve,maxProducts=10000):NurbsDecompositionAudit {
 return callNurbsRust('curve_decomposition_audit',{curve,span,retained,maxProducts})
}
export interface NurbsDecompositionBatchAudit extends NurbsDecompositionAudit {pairsInspected:number}
/** Complete maximum under one shared product budget. A refused final span
 * invalidates the aggregate; previously proved bounds are not returned. */
export function inspectNurbsDecompositionBatch(pairs:{curve:NurbsCurve;span:number;retained:NurbsCurve}[],maxProducts=100000):NurbsDecompositionBatchAudit {
 return callNurbsRust('curve_decomposition_batch_audit',{pairs,maxProducts})
}
export function elevateNurbsCurve(curve: NurbsCurve, degree: number): NurbsCurve { return callNurbsRust('curve_elevate', { curve, degree }) }
export function nurbsCurveBounds(curve: NurbsCurve): { min: number[]; max: number[] } { return callNurbsRust('curve_bounds', { curve }) }

/** Global distance interval on the original curves; convergence does not imply a unique pair. */
export interface NurbsCurveDistance {
  method:'interval-de-boor-pair-subdivision'
  distanceIntervalMm:[number,number]
  parameters:[number,number]
  points:[number[],number[]]
  pointEnclosures:[Array<[number,number]>,Array<[number,number]>]
  converged:boolean
  reason:'tolerance'|'work-limit'|'precision-limit'
  cells:number
  maxCells:number
  toleranceMm:number
}
export function measureNurbsCurveDistance(a:NurbsCurve,b:NurbsCurve,toleranceMm=0.001,maxCells=10000):NurbsCurveDistance {
  return callNurbsRust('curve_distance',{a,b,toleranceMm,maxCells})
}

export function transformNurbsCurve(curve:NurbsCurve,matrix:number[][]):NurbsCurve {return callNurbsRust("curve_affine",{curve,matrix})}

/** Curvature summary at parameter `u` along a NURBS curve. */
export interface NurbsCurveCurvature {
  point: number[]
  tangent: number[] | null
  curvatureVector: number[] | null
  curvature: number
}

/**
 * Compute arc length of `curve` via composite Gauss-Legendre / chord integration over `samples` sub-intervals.
 */
export function evaluateNurbsCurveLength(curve: NurbsCurve, samples = 64): number {
  const n = Math.max(2, Math.floor(samples))
  const pts = sampleNurbsCurve(curve, n, true)
  if (pts.length < 2) return 0
  let total = 0
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1]!
    const b = pts[i]!
    const dims = Math.min(a.length, b.length)
    let distSq = 0
    for (let d = 0; d < dims; d++) {
      const diff = (b[d] ?? 0) - (a[d] ?? 0)
      distSq += diff * diff
    }
    total += Math.sqrt(distSq)
  }
  return total
}

/**
 * Evaluate analytical/differential curvature $\kappa(u) = \frac{\|C'(u) \times C''(u)\|}{\|C'(u)\|^3}$
 * and normal curvature vector at parameter `u` on a 2D or 3D NURBS curve.
 */
export function evaluateNurbsCurveCurvature(curve: NurbsCurve, u: number): NurbsCurveCurvature {
  const ev = evaluateNurbsCurve(curve, u)
  const d1 = ev.d1
  const d2 = ev.d2
  if (!d1 || !d2 || d1.length < 2 || d2.length < 2) {
    return {
      point: ev.point,
      tangent: null,
      curvatureVector: null,
      curvature: 0,
    }
  }
  const dx = d1[0] ?? 0
  const dy = d1[1] ?? 0
  const dz = d1[2] ?? 0
  const ddx = d2[0] ?? 0
  const ddy = d2[1] ?? 0
  const ddz = d2[2] ?? 0

  const speedSq = dx * dx + dy * dy + dz * dz
  const speed = Math.sqrt(speedSq)
  if (!Number.isFinite(speed) || speed <= 1e-14) {
    return {
      point: ev.point,
      tangent: null,
      curvatureVector: null,
      curvature: 0,
    }
  }

  const tangent = d1.length >= 3 ? [dx / speed, dy / speed, dz / speed] : [dx / speed, dy / speed]
  // Component of C'' orthogonal to C' divided by |C'|^2 gives the curvature vector K = T'(s)
  const dotD1D2 = dx * ddx + dy * ddy + dz * ddz
  const invSpeedSq = 1 / speedSq
  const invSpeed4 = invSpeedSq * invSpeedSq
  const kx = (ddx * speedSq - dx * dotD1D2) * invSpeed4
  const ky = (ddy * speedSq - dy * dotD1D2) * invSpeed4
  const kz = (ddz * speedSq - dz * dotD1D2) * invSpeed4
  const curvature = Math.hypot(kx, ky, kz)
  const curvatureVector = d1.length >= 3 ? [kx, ky, kz] : [kx, ky]

  return {
    point: ev.point,
    tangent,
    curvatureVector,
    curvature: Number.isFinite(curvature) ? curvature : 0,
  }
}

/**
 * Find the closest point on a 2D or 3D NURBS curve to `target` using coarse uniform parameter
 * sampling followed by damped Newton / secant refinement on $f(u) = (C(u) - P) \cdot C'(u)$.
 */
export function closestPointOnNurbsCurve(
  curve: NurbsCurve,
  target: [number, number, number],
  samples = 64,
): { u: number; point: [number, number, number]; distance: number } {
  const n = Math.max(8, Math.floor(samples))
  const firstEv = evaluateNurbsCurve(curve, curve.knots[curve.degree] ?? 0)
  const [uMin, uMax] = firstEv.domain
  const span = Math.max(0, uMax - uMin)

  const toPoint3 = (p: readonly number[]): [number, number, number] => [
    p[0] ?? 0,
    p[1] ?? 0,
    p[2] ?? 0,
  ]
  const distSq3 = (a: [number, number, number], b: [number, number, number]): number => {
    const dx = a[0] - b[0]
    const dy = a[1] - b[1]
    const dz = a[2] - b[2]
    return dx * dx + dy * dy + dz * dz
  }

  let bestU = uMin
  let bestPt = toPoint3(firstEv.point)
  let bestDistSq = distSq3(bestPt, target)

  if (span <= 1e-15) {
    return { u: bestU, point: bestPt, distance: Math.sqrt(bestDistSq) }
  }

  const params: number[] = []
  for (let i = 0; i <= n; i++) {
    params.push(uMin + (i / n) * span)
  }
  const coarsePts = evaluateNurbsCurvePoints(curve, params)
  for (let i = 0; i < coarsePts.length; i++) {
    const pt = toPoint3(coarsePts[i]!)
    const d2 = distSq3(pt, target)
    if (d2 < bestDistSq) {
      bestDistSq = d2
      bestU = params[i]!
      bestPt = pt
    }
  }

  // Local Newton / golden-section hybrid refinement around bestU
  let u = bestU
  for (let iter = 0; iter < 12; iter++) {
    const ev = evaluateNurbsCurve(curve, u)
    const pt = toPoint3(ev.point)
    const d2 = distSq3(pt, target)
    if (d2 < bestDistSq) {
      bestDistSq = d2
      bestU = u
      bestPt = pt
    }
    const d1 = ev.d1
    if (!d1 || d1.length < 2) break
    const rx = pt[0] - target[0]
    const ry = pt[1] - target[1]
    const rz = pt[2] - target[2]
    const c1x = d1[0] ?? 0
    const c1y = d1[1] ?? 0
    const c1z = d1[2] ?? 0
    const grad = rx * c1x + ry * c1y + rz * c1z
    if (Math.abs(grad) <= 1e-12) break

    let hess = c1x * c1x + c1y * c1y + c1z * c1z
    if (ev.d2 && ev.d2.length >= 2) {
      const c2x = ev.d2[0] ?? 0
      const c2y = ev.d2[1] ?? 0
      const c2z = ev.d2[2] ?? 0
      hess += rx * c2x + ry * c2y + rz * c2z
    }
    if (!Number.isFinite(hess) || hess <= 1e-12) {
      hess = c1x * c1x + c1y * c1y + c1z * c1z
    }
    if (hess <= 1e-14) break

    const step = grad / hess
    const nextU = Math.min(uMax, Math.max(uMin, u - step))
    if (Math.abs(nextU - u) <= 1e-11 * Math.max(1, span)) {
      u = nextU
      break
    }
    const nextEv = evaluateNurbsCurve(curve, nextU)
    const nextPt = toPoint3(nextEv.point)
    const nextD2 = distSq3(nextPt, target)
    if (nextD2 <= bestDistSq) {
      bestDistSq = nextD2
      bestU = nextU
      bestPt = nextPt
      u = nextU
    } else {
      // Backtrack half step
      const halfU = Math.min(uMax, Math.max(uMin, u - 0.5 * step))
      const halfPt = toPoint3(evaluateNurbsCurve(curve, halfU).point)
      const halfD2 = distSq3(halfPt, target)
      if (halfD2 < bestDistSq) {
        bestDistSq = halfD2
        bestU = halfU
        bestPt = halfPt
        u = halfU
      } else {
        break
      }
    }
  }

  return {
    u: bestU,
    point: bestPt,
    distance: Math.sqrt(bestDistSq),
  }
}

/**
 * Compute a tight 3D axis-aligned bounding box `{ min, max }` for a 2D or 3D NURBS curve
 * by evaluating `samples` uniform parameter samples along the curve domain.
 */
export function computeNurbsCurveBoundingBox(
  curve: NurbsCurve,
  samples = 64,
): { min: [number, number, number]; max: [number, number, number] } {
  const n = Math.max(2, Math.floor(samples))
  const pts = sampleNurbsCurve(curve, n, true)
  if (pts.length === 0) {
    return {
      min: [0, 0, 0],
      max: [0, 0, 0],
    }
  }
  let minX = Number.POSITIVE_INFINITY
  let minY = Number.POSITIVE_INFINITY
  let minZ = Number.POSITIVE_INFINITY
  let maxX = Number.NEGATIVE_INFINITY
  let maxY = Number.NEGATIVE_INFINITY
  let maxZ = Number.NEGATIVE_INFINITY
  for (const pt of pts) {
    const x = pt[0] ?? 0
    const y = pt[1] ?? 0
    const z = pt[2] ?? 0
    if (x < minX) minX = x
    if (y < minY) minY = y
    if (z < minZ) minZ = z
    if (x > maxX) maxX = x
    if (y > maxY) maxY = y
    if (z > maxZ) maxZ = z
  }
  return {
    min: [minX, minY, minZ],
    max: [maxX, maxY, maxZ],
  }
}

/**
 * Adaptively tessellate a 2D or 3D NURBS curve into a 3D polyline whose midpoint chordal
 * deviation on every segment is at most `chordTolerance` (up to `maxDepth` recursive splits).
 */
export function subdivideNurbsCurvePolylineAdaptive(
  curve: NurbsCurve,
  chordTolerance: number,
  maxDepth = 10,
): Array<[number, number, number]> {
  const tol = Math.max(1e-9, chordTolerance)
  const tolSq = tol * tol
  const depthLimit = Math.max(1, Math.min(18, Math.floor(maxDepth)))
  const firstEv = evaluateNurbsCurve(curve, curve.knots[curve.degree] ?? 0)
  const [uMin, uMax] = firstEv.domain
  const toPoint3 = (p: readonly number[]): [number, number, number] => [
    p[0] ?? 0,
    p[1] ?? 0,
    p[2] ?? 0,
  ]
  const pStart = toPoint3(evaluateNurbsCurve(curve, uMin).point)
  const pEnd = toPoint3(evaluateNurbsCurve(curve, uMax).point)
  if (Math.abs(uMax - uMin) <= 1e-15) {
    return [pStart]
  }

  const out: Array<[number, number, number]> = [pStart]

  const pointToSegmentDistSq = (
    p: [number, number, number],
    a: [number, number, number],
    b: [number, number, number],
  ): number => {
    const abx = b[0] - a[0]
    const aby = b[1] - a[1]
    const abz = b[2] - a[2]
    const apx = p[0] - a[0]
    const apy = p[1] - a[1]
    const apz = p[2] - a[2]
    const abLenSq = abx * abx + aby * aby + abz * abz
    if (abLenSq <= 1e-24) {
      return apx * apx + apy * apy + apz * apz
    }
    const t = Math.max(0, Math.min(1, (apx * abx + apy * aby + apz * abz) / abLenSq))
    const dx = p[0] - (a[0] + t * abx)
    const dy = p[1] - (a[1] + t * aby)
    const dz = p[2] - (a[2] + t * abz)
    return dx * dx + dy * dy + dz * dz
  }

  const recurse = (
    u0: number,
    p0: [number, number, number],
    u1: number,
    p1: [number, number, number],
    depth: number,
  ): void => {
    const uMid = 0.5 * (u0 + u1)
    const pMid = toPoint3(evaluateNurbsCurve(curve, uMid).point)
    // Always split at least once at depth 0 so S-curves with inflection points at uMid are detected
    const devSq = pointToSegmentDistSq(pMid, p0, p1)
    if (depth < depthLimit && (depth === 0 || devSq > tolSq)) {
      recurse(u0, p0, uMid, pMid, depth + 1)
      recurse(uMid, pMid, u1, p1, depth + 1)
    } else {
      out.push(p1)
    }
  }

  recurse(uMin, pStart, uMax, pEnd, 0)
  return out
}
