import {
  type NurbsCurve,
  validateNurbsCurve,
  nurbsBasisDerivatives,
  insertNurbsKnot,
  elevateNurbsCurve,
  trimNurbsCurve,
  reverseNurbsCurve,
} from './nurbsCurve'

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

function reject(message: string): never { throw new Error(message) }
function axisCurves(surface: NurbsSurface, axis: NurbsSurfaceAxis): NurbsCurve[] {
  if (axis === 'u') {
    return surface.controlPoints[0].map((_, v) => ({
      degree: surface.degreeU,
      knots: [...surface.knotsU],
      controlPoints: surface.controlPoints.map(row => [...row[v]]),
      weights: surface.weights.map(row => row[v]),
      periodic: surface.periodicU ?? false,
    }))
  }
  return surface.controlPoints.map((row, u) => ({
    degree: surface.degreeV,
    knots: [...surface.knotsV],
    controlPoints: row.map(point => [...point]),
    weights: [...surface.weights[u]],
    periodic: surface.periodicV ?? false,
  }))
}

/** Tensor-product rational B-spline validation, without a geometry-library dependency. */
export function validateNurbsSurface(surface: NurbsSurface): void {
  if (!surface || !Array.isArray(surface.controlPoints) || surface.controlPoints.length < 2 || surface.controlPoints.length > 32) {
    reject('NURBS surface requires 2..32 U rows of control points.')
  }
  const countV = surface.controlPoints[0]?.length
  if (!Number.isInteger(countV) || countV < 2 || countV > 32 || surface.controlPoints.some(row => !Array.isArray(row) || row.length !== countV)) {
    reject('NURBS surface control net must be rectangular with 2..32 V points per U row.')
  }
  if (surface.controlPoints.some(row => row.some(point => !Array.isArray(point) || point.length !== 3))) {
    reject('NURBS surface control points must each have exactly three coordinates.')
  }
  if (!Array.isArray(surface.weights) || surface.weights.length !== surface.controlPoints.length || surface.weights.some(row => !Array.isArray(row) || row.length !== countV)) {
    reject('NURBS surface weights must match the rectangular U/V control net.')
  }
  const weights = surface.weights.flat()
  if (Math.max(...weights) / Math.min(...weights) > 1e12) reject('NURBS surface weight conditioning must not exceed 1e12 across the complete net.')
  // Each parameter line obeys the same degree, natural-domain and periodic
  // storage contract as a curve. Positive weights make the tensor denominator
  // a positive convex combination on the natural domain.
  for (const axis of ['u', 'v'] as const) {
    for (const curve of axisCurves(surface, axis)) validateNurbsCurve(curve)
  }
}

function dot(a: number[], b: number[]): number { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
function cross(a: number[], b: number[]): Point3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
function scale(a: number[], factor: number): Point3 { return [a[0] * factor, a[1] * factor, a[2] * factor] }

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

/** Analytic rational quotient jets; finite differences are not used. */
export function evaluateNurbsSurface(surface: NurbsSurface, u: number, v: number): NurbsSurfaceEvaluation {
  validateNurbsSurface(surface)
  return evaluateValidatedSurface(surface, u, v)
}

/** Validate and capture an immutable snapshot once for bounded tessellation. */
export function createNurbsSurfaceEvaluator(surface: NurbsSurface): (u: number, v: number) => NurbsSurfaceEvaluation {
  validateNurbsSurface(surface)
  const snapshot: NurbsSurface = {
    ...surface, knotsU: [...surface.knotsU], knotsV: [...surface.knotsV],
    controlPoints: surface.controlPoints.map(row => row.map(point => [...point])),
    weights: surface.weights.map(row => [...row]),
  }
  return (u, v) => evaluateValidatedSurface(snapshot, u, v)
}

function evaluateValidatedSurface(surface: NurbsSurface, u: number, v: number): NurbsSurfaceEvaluation {
  const basisU = nurbsBasisDerivatives(surface.degreeU, surface.knotsU, surface.controlPoints.length, u, surface.periodicU)
  const basisV = nurbsBasisDerivatives(surface.degreeV, surface.knotsV, surface.controlPoints[0].length, v, surface.periodicV)
  const uJets = [basisU.basis, basisU.d1, basisU.d2]
  const vJets = [basisV.basis, basisV.d1, basisV.d2]
  const derivatives = [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]] as const
  // Homogeneous weight normalization preserves the rational surface and keeps
  // the denominator near unit scale when caller weights share a large scale.
  const weightScale = Math.max(...surface.weights.flat())
  // Derivatives are translation invariant. Keeping homogeneous numerators
  // near the model avoids cancellation against a large world-space origin.
  const origin = surface.controlPoints[0][0]
  const homogeneous = derivatives.map(([orderU, orderV]) => {
    const result = [0, 0, 0, 0]
    for (let i = 0; i < surface.controlPoints.length; i++) {
      for (let j = 0; j < surface.controlPoints[i].length; j++) {
        const coefficient = uJets[orderU][i] * vJets[orderV][j] * (surface.weights[i][j] / weightScale)
        if (coefficient === 0) continue
        result[3] += coefficient
        for (let dimension = 0; dimension < 3; dimension++) result[dimension] += coefficient * (surface.controlPoints[i][j][dimension] - origin[dimension])
      }
    }
    return result
  })
  const [h, hu, hv, huu, huv, hvv] = homogeneous
  if (!Number.isFinite(h[3]) || h[3] <= 0) reject('NURBS surface has an invalid rational denominator.')
  const relativePoint: Point3 = [h[0] / h[3], h[1] / h[3], h[2] / h[3]]
  const point = relativePoint.map((coordinate, i) => coordinate + origin[i]) as Point3
  const first = (jet: number[]): Point3 => relativePoint.map((coordinate, i) => (jet[i] - jet[3] * coordinate) / h[3]) as Point3
  const du = first(hu), dv = first(hv)
  const duu = relativePoint.map((coordinate, i) => (huu[i] - 2 * hu[3] * du[i] - huu[3] * coordinate) / h[3]) as Point3
  const duv = relativePoint.map((coordinate, i) => (huv[i] - hu[3] * dv[i] - hv[3] * du[i] - huv[3] * coordinate) / h[3]) as Point3
  const dvv = relativePoint.map((coordinate, i) => (hvv[i] - 2 * hv[3] * dv[i] - hvv[3] * coordinate) / h[3]) as Point3
  if (![...point, ...du, ...dv, ...duu, ...duv, ...dvv].every(Number.isFinite)) reject('NURBS surface evaluation exceeded finite numeric bounds.')
  const smoothU = basisU.continuity === null ? Infinity : basisU.continuity
  const smoothV = basisV.continuity === null ? Infinity : basisV.continuity
  const firstAvailable = smoothU >= 1 && smoothV >= 1
  const secondAvailable = smoothU >= 2 && smoothV >= 2
  let normal: Point3 | null = null, gaussianCurvature: number | null = null, meanCurvature: number | null = null
  if (firstAvailable) {
    const direction = cross(du, dv)
    const magnitude = Math.hypot(...direction), speedU = Math.hypot(...du), speedV = Math.hypot(...dv)
    if (magnitude > 1e-12 * speedU * speedV && speedU > 0 && speedV > 0) {
      normal = scale(direction, 1 / magnitude)
      if (secondAvailable) {
        const E = dot(du, du), F = dot(du, dv), G = dot(dv, dv)
        const L = dot(normal, duu), M = dot(normal, duv), N = dot(normal, dvv)
        // |Su x Sv|^2 avoids cancellation in E*G-F^2 near oblique charts.
        const determinant = magnitude * magnitude
        const gaussian = (L * N - M * M) / determinant
        const mean = (E * N - 2 * F * M + G * L) / (2 * determinant)
        if (Number.isFinite(gaussian)) gaussianCurvature = gaussian
        if (Number.isFinite(mean)) meanCurvature = mean
      }
    }
  }
  return {
    point, du: smoothU >= 1 ? du : null, dv: smoothV >= 1 ? dv : null,
    duu: smoothU >= 2 ? duu : null, duv: firstAvailable ? duv : null, dvv: smoothV >= 2 ? dvv : null,
    normal, gaussianCurvature, meanCurvature,
    derivative_status: !firstAvailable || !secondAvailable ? 'insufficient_continuity' : normal === null ? 'singular' : 'available',
    derivative_side_u: basisU.derivative_side,
    derivative_side_v: basisV.derivative_side,
    domainU: [...basisU.domain], domainV: [...basisV.domain],
  }
}

function editAxis(surface: NurbsSurface, axis: NurbsSurfaceAxis, edit: (curve: NurbsCurve) => NurbsCurve): NurbsSurface {
  validateNurbsSurface(surface)
  if (axis !== 'u' && axis !== 'v') reject('NURBS surface axis must be u or v.')
  const curves = axisCurves(surface, axis).map(edit)
  const reference = curves[0]
  if (curves.some(curve => curve.degree !== reference.degree || curve.controlPoints.length !== reference.controlPoints.length || curve.knots.length !== reference.knots.length || curve.knots.some((knot, i) => knot !== reference.knots[i]))) {
    reject('NURBS surface axis edit produced inconsistent parameter lines.')
  }
  const result: NurbsSurface = axis === 'u' ? {
    degreeU: reference.degree, degreeV: surface.degreeV,
    knotsU: [...reference.knots], knotsV: [...surface.knotsV],
    controlPoints: reference.controlPoints.map((_, u) => curves.map(curve => [...curve.controlPoints[u]])),
    weights: reference.weights.map((_, u) => curves.map(curve => curve.weights[u])),
    periodicU: reference.periodic ?? false, periodicV: surface.periodicV ?? false,
  } : {
    degreeU: surface.degreeU, degreeV: reference.degree,
    knotsU: [...surface.knotsU], knotsV: [...reference.knots],
    controlPoints: curves.map(curve => curve.controlPoints.map(point => [...point])),
    weights: curves.map(curve => [...curve.weights]),
    periodicU: surface.periodicU ?? false, periodicV: reference.periodic ?? false,
  }
  validateNurbsSurface(result)
  return result
}

export function insertNurbsSurfaceKnot(surface: NurbsSurface, axis: NurbsSurfaceAxis, value: number, count = 1): NurbsSurface {
  return editAxis(surface, axis, curve => insertNurbsKnot(curve, value, count))
}

export function elevateNurbsSurface(surface: NurbsSurface, axis: NurbsSurfaceAxis, targetDegree: number): NurbsSurface {
  return editAxis(surface, axis, curve => elevateNurbsCurve(curve, targetDegree))
}

export function reverseNurbsSurface(surface: NurbsSurface, axis: NurbsSurfaceAxis): NurbsSurface {
  return editAxis(surface, axis, reverseNurbsCurve)
}

export function trimNurbsSurface(surface: NurbsSurface, bounds: [number, number, number, number]): NurbsSurface
export function trimNurbsSurface(surface: NurbsSurface, uMin: number, uMax: number, vMin: number, vMax: number): NurbsSurface
export function trimNurbsSurface(surface: NurbsSurface, first: number | [number, number, number, number], uMax?: number, vMin?: number, vMax?: number): NurbsSurface {
  const bounds = Array.isArray(first) ? first : [first, uMax!, vMin!, vMax!]
  if (bounds.length !== 4 || !bounds.every(Number.isFinite) || bounds[0] >= bounds[1] || bounds[2] >= bounds[3]) reject('NURBS surface trim requires increasing finite U and V intervals.')
  const uTrim = editAxis(surface, 'u', curve => trimNurbsCurve(curve, bounds[0], bounds[1]))
  return editAxis(uTrim, 'v', curve => trimNurbsCurve(curve, bounds[2], bounds[3]))
}

/** Fix the named parameter and return the exact rational curve in the other. */
export function isoNurbsCurve(surface: NurbsSurface, direction: NurbsSurfaceAxis, parameter: number): NurbsCurve {
  validateNurbsSurface(surface)
  if (direction !== 'u' && direction !== 'v') reject('NURBS iso-curve direction must be u or v.')
  const fixedU = direction === 'u'
  const countFixed = fixedU ? surface.controlPoints.length : surface.controlPoints[0].length
  const countVarying = fixedU ? surface.controlPoints[0].length : surface.controlPoints.length
  const basis = nurbsBasisDerivatives(fixedU ? surface.degreeU : surface.degreeV, fixedU ? surface.knotsU : surface.knotsV, countFixed, parameter, fixedU ? surface.periodicU : surface.periodicV).basis
  const controlPoints: number[][] = [], weights: number[] = []
  for (let i = 0; i < countVarying; i++) {
    const weighted = [0, 0, 0]
    const origin = surface.controlPoints[fixedU ? 0 : i][fixedU ? i : 0]
    let weight = 0
    for (let j = 0; j < countFixed; j++) {
      const u = fixedU ? j : i, v = fixedU ? i : j
      const coefficient = basis[j] * surface.weights[u][v]
      weight += coefficient
      for (let dimension = 0; dimension < 3; dimension++) weighted[dimension] += coefficient * (surface.controlPoints[u][v][dimension] - origin[dimension])
    }
    if (!Number.isFinite(weight) || weight <= 0) reject('NURBS iso-curve has an invalid rational denominator.')
    controlPoints.push(weighted.map((value, dimension) => origin[dimension] + value / weight))
    weights.push(weight)
  }
  const curve: NurbsCurve = {
    degree: fixedU ? surface.degreeV : surface.degreeU,
    knots: [...(fixedU ? surface.knotsV : surface.knotsU)],
    controlPoints, weights,
    periodic: (fixedU ? surface.periodicV : surface.periodicU) ?? false,
  }
  validateNurbsCurve(curve)
  return curve
}

/** Positive rational basis functions keep the image inside this control hull. */
export function nurbsSurfaceBounds(surface: NurbsSurface): { min: Point3; max: Point3 } {
  validateNurbsSurface(surface)
  const points = surface.controlPoints.flat()
  return {
    min: [0, 1, 2].map(axis => Math.min(...points.map(point => point[axis]))) as Point3,
    max: [0, 1, 2].map(axis => Math.max(...points.map(point => point[axis]))) as Point3,
  }
}
