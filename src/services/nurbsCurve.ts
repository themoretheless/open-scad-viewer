/**
 * Native ModelGraph NURBS curve mathematics. No geometry package is used.
 *
 * Knots use the usual expanded representation: N control points have N+p+1
 * knots, and the active domain is [knots[p], knots[N]]. Evaluation and editing
 * use rational homogeneous coordinates in binary64; these are numerical
 * algorithms, not interval certificates or exact real arithmetic.
 */
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

export class NurbsCurveError extends Error {
  constructor(public readonly code: 'NURBS_INVALID_INPUT' | 'NURBS_RESOURCE_LIMIT' | 'NURBS_NUMERIC_ERROR', message: string) {
    super(message)
    this.name = 'NurbsCurveError'
  }
}

const MAX_CONTROLS = 256
const MAX_DEGREE = 25
const MAX_ABS_VALUE = 1e9

function requireInput(condition: unknown, message: string): asserts condition {
  if (!condition) throw new NurbsCurveError('NURBS_INVALID_INPUT', message)
}

function requireBudget(controlCount: number): void {
  if (controlCount > MAX_CONTROLS) {
    throw new NurbsCurveError('NURBS_RESOURCE_LIMIT', `The result exceeds ${MAX_CONTROLS} control points`)
  }
}

function isBounded(value: number): boolean {
  return Number.isFinite(value) && Math.abs(value) <= MAX_ABS_VALUE
}

function validateBasis(degree: number, knots: number[], count: number): [number, number] {
  requireInput(Number.isInteger(degree) && degree >= 1 && degree <= MAX_DEGREE, 'Degree must be an integer in [1, 25]')
  requireInput(Number.isInteger(count) && count >= degree + 1 && count <= MAX_CONTROLS, 'Control point count must be between degree+1 and 256')
  requireInput(Array.isArray(knots) && knots.length === count + degree + 1, 'Expanded knot count must equal control point count + degree + 1')
  let multiplicity = 0
  for (let i = 0; i < knots.length; i++) {
    requireInput(isBounded(knots[i]), 'Knots must be finite and bounded by 1e9')
    requireInput(i === 0 || knots[i] >= knots[i - 1], 'Knots must be nondecreasing')
    multiplicity = i > 0 && knots[i] === knots[i - 1] ? multiplicity + 1 : 1
    requireInput(multiplicity <= degree + 1, 'Knot multiplicity must not exceed degree+1')
  }
  const domain: [number, number] = [knots[degree], knots[count]]
  requireInput(domain[0] < domain[1], 'The active knot domain must have positive length')
  for (let i = 0; i < knots.length;) {
    let end = i + 1
    while (end < knots.length && knots[end] === knots[i]) end++
    requireInput(knots[i] <= domain[0] || knots[i] >= domain[1] || end - i <= degree,
      'Interior multiplicity must not exceed degree; disconnected curves need separate nodes')
    i = end
  }
  return domain
}

function nearlyEqual(a: number, b: number, coordinateScale: number): boolean {
  return Math.abs(a - b) <= 32 * Number.EPSILON * Math.max(Number.MIN_VALUE, coordinateScale, Math.abs(a), Math.abs(b))
}

export function validateNurbsCurve(curve: NurbsCurve): void {
  requireInput(curve !== null && typeof curve === 'object', 'Expected a NURBS curve')
  requireInput(Array.isArray(curve.controlPoints), 'Expected control points')
  const count = curve.controlPoints.length
  const domain = validateBasis(curve.degree, curve.knots, count)
  const dimension = curve.controlPoints[0]?.length
  requireInput(dimension === 2 || dimension === 3, 'Control points must have two or three coordinates')
  requireInput(curve.controlPoints.every(point => Array.isArray(point) && point.length === dimension && point.every(isBounded)),
    'Control points must have consistent dimensions and finite coordinates bounded by 1e9')
  requireInput(Array.isArray(curve.weights) && curve.weights.length === count, 'Weights must match the control point count')
  requireInput(curve.weights.every(weight => Number.isFinite(weight) && weight >= 1e-12 && weight <= 1e12),
    'Weights must be positive, finite, and in [1e-12, 1e12]')
  requireInput(Math.max(...curve.weights) / Math.min(...curve.weights) <= 1e12, 'Weight conditioning must not exceed 1e12')
  requireInput(curve.periodic === undefined || typeof curve.periodic === 'boolean', 'Periodic must be boolean')
  if (curve.periodic) {
    const periodControls = count - curve.degree
    requireInput(periodControls >= curve.degree + 1, 'A periodic curve needs at least degree+1 unwrapped control points')
    for (let i = 0; i < curve.degree; i++) {
      requireInput(curve.weights[i] === curve.weights[periodControls + i]
        && curve.controlPoints[i].every((coordinate, axis) => coordinate === curve.controlPoints[periodControls + i][axis]),
      'Periodic curves must explicitly repeat the first degree control points and weights at the end')
    }
    const period = domain[1] - domain[0]
    const knotScale = Math.max(...curve.knots.map(Math.abs))
    for (let i = 0; i + periodControls < curve.knots.length; i++) {
      requireInput(nearlyEqual(curve.knots[i + periodControls] - curve.knots[i], period, knotScale),
        'Periodic exterior knots must repeat with the active period')
    }
  }
}

/**
 * Cox–de Boor basis and the first two polynomial derivative recurrences.
 * Arrays contain all N basis functions, including zeros outside the local
 * support. Jets at an interior low-continuity knot are right-sided; consumers
 * must inspect continuity/status before presenting them as two-sided values.
 */
export function nurbsBasisDerivatives(degree: number, knots: number[], controlCount: number, u: number, periodic = false): NurbsBasisEvaluation {
  const domain = validateBasis(degree, knots, controlCount)
  requireInput(Number.isFinite(u) && u >= domain[0] && u <= domain[1], 'Parameter is outside the active knot domain')

  // At the natural end choose the last nonzero span on its left, including
  // nonclamped curves whose last several active knots may coincide.
  let span = degree
  if (u === domain[1]) {
    span = controlCount - 1
    while (span > 0 && knots[span] === u) span--
  } else {
    while (span + 1 < knots.length && knots[span + 1] <= u) span++
  }
  let basis = new Array<number>(knots.length - 1).fill(0)
  basis[span] = 1
  let first = new Array<number>(basis.length).fill(0)
  let second = new Array<number>(basis.length).fill(0)
  for (let order = 1; order <= degree; order++) {
    const next = new Array<number>(knots.length - order - 1).fill(0)
    const nextFirst = new Array<number>(next.length).fill(0)
    const nextSecond = new Array<number>(next.length).fill(0)
    for (let i = 0; i < next.length; i++) {
      const left = knots[i + order] - knots[i]
      const right = knots[i + order + 1] - knots[i + 1]
      if (left !== 0) {
        next[i] += (u - knots[i]) / left * basis[i]
        nextFirst[i] += order / left * basis[i]
        nextSecond[i] += order / left * first[i]
      }
      if (right !== 0) {
        next[i] += (knots[i + order + 1] - u) / right * basis[i + 1]
        nextFirst[i] -= order / right * basis[i + 1]
        nextSecond[i] -= order / right * first[i + 1]
      }
    }
    basis = next
    first = nextFirst
    second = nextSecond
  }
  const endpoint = u === domain[0] || u === domain[1]
  // A nonperiodic endpoint has one-sided derivatives. A periodic endpoint is
  // also a seam in a closed parameter space, so its knot continuity applies.
  const multiplicity = endpoint && !periodic ? 0 : knots.filter(knot => knot === u).length
  const continuity = multiplicity === 0 ? null : degree - multiplicity
  const derivative_status = continuity === null || continuity >= 2 ? 'available' : 'insufficient_continuity'
  const derivative_side = u === domain[0] ? 'right' : u === domain[1] ? 'left'
    : derivative_status === 'insufficient_continuity' ? 'right' : 'two_sided'
  if (![...basis, ...first, ...second].every(Number.isFinite)) {
    throw new NurbsCurveError('NURBS_NUMERIC_ERROR', 'Knot scale exhausted finite derivative precision')
  }
  return { basis, d1: first, d2: second, domain, continuity, derivative_status, derivative_side }
}

export function evaluateNurbsCurve(curve: NurbsCurve, u: number): NurbsCurveEvaluation {
  validateNurbsCurve(curve)
  const basis = nurbsBasisDerivatives(curve.degree, curve.knots, curve.controlPoints.length, u, curve.periodic)
  const dimension = curve.controlPoints[0].length
  const point = new Array<number>(dimension).fill(0)
  const d1 = new Array<number>(dimension).fill(0)
  const d2 = new Array<number>(dimension).fill(0)
  const scale = Math.max(...curve.weights)
  // Local coordinates prevent translated geometry from cancelling large
  // homogeneous position terms in the rational derivative quotient.
  const origin = curve.controlPoints[basis.basis.findIndex(value => value > 0)]
  let weight = 0, weightFirst = 0, weightSecond = 0
  for (let i = 0; i < curve.controlPoints.length; i++) {
    const w = curve.weights[i] / scale
    weight += basis.basis[i] * w
    weightFirst += basis.d1[i] * w
    weightSecond += basis.d2[i] * w
    for (let axis = 0; axis < dimension; axis++) {
      const coordinate = curve.controlPoints[i][axis] - origin[axis]
      point[axis] += basis.basis[i] * w * coordinate
      d1[axis] += basis.d1[i] * w * coordinate
      d2[axis] += basis.d2[i] * w * coordinate
    }
  }
  if (!(weight > 0) || !Number.isFinite(weight)) {
    throw new NurbsCurveError('NURBS_NUMERIC_ERROR', 'Rational denominator lost its positive finite value')
  }
  for (let axis = 0; axis < dimension; axis++) {
    point[axis] /= weight
    d1[axis] = (d1[axis] - weightFirst * point[axis]) / weight
    d2[axis] = (d2[axis] - 2 * weightFirst * d1[axis] - weightSecond * point[axis]) / weight
    point[axis] += origin[axis]
  }
  if (![...point, ...d1, ...d2].every(Number.isFinite)) {
    throw new NurbsCurveError('NURBS_NUMERIC_ERROR', 'Rational evaluation exhausted finite precision')
  }
  return {
    point,
    d1: basis.continuity === null || basis.continuity >= 1 ? d1 : null,
    d2: basis.continuity === null || basis.continuity >= 2 ? d2 : null,
    domain: basis.domain,
    continuity: basis.continuity,
    derivative_status: basis.derivative_status,
    derivative_side: basis.derivative_side,
  }
}

function clone(curve: NurbsCurve): NurbsCurve {
  return { degree: curve.degree, knots: [...curve.knots], controlPoints: curve.controlPoints.map(point => [...point]),
    weights: [...curve.weights], periodic: curve.periodic ?? false }
}

interface HomogeneousCurve {
  degree: number
  knots: number[]
  controls: number[][]
}

function homogeneous(curve: NurbsCurve): HomogeneousCurve {
  return { degree: curve.degree, knots: [...curve.knots], controls: curve.controlPoints.map((point, i) =>
    [...point.map(coordinate => coordinate * curve.weights[i]), curve.weights[i]]) }
}

function fromHomogeneous(curve: HomogeneousCurve): NurbsCurve {
  const weights = curve.controls.map(point => point[point.length - 1])
  const result: NurbsCurve = {
    degree: curve.degree, knots: [...curve.knots], weights, periodic: false,
    controlPoints: curve.controls.map((point, i) => point.slice(0, -1).map(coordinate => coordinate / weights[i])),
  }
  requireBudget(result.controlPoints.length)
  validateNurbsCurve(result)
  return result
}

function multiplicity(knots: number[], u: number): number {
  return knots.filter(knot => knot === u).length
}

/** Insert a single knot, allowing a temporary interior degree+1 split. */
function insertHomogeneous(curve: HomogeneousCurve, u: number): HomogeneousCurve {
  const p = curve.degree, n = curve.controls.length - 1
  const s = multiplicity(curve.knots, u)
  requireInput(s <= p, 'Requested knot already has degree+1 multiplicity')
  // Unlike endpoint evaluation, nonclamped endpoint insertion uses the span
  // on the right of the knot. There are still exterior controls/knots there.
  let k = p
  while (k + 1 < curve.knots.length && curve.knots[k + 1] <= u) k++
  const controls: number[][] = new Array(n + 2)
  for (let i = 0; i <= k - p; i++) controls[i] = [...curve.controls[i]]
  for (let i = k - s + 1; i <= n + 1; i++) controls[i] = [...curve.controls[i - 1]]
  for (let i = k - p + 1; i <= k - s; i++) {
    const denominator = curve.knots[i + p] - curve.knots[i]
    requireInput(denominator > 0, 'Knot insertion has an empty local span')
    const alpha = (u - curve.knots[i]) / denominator
    controls[i] = curve.controls[i].map((coordinate, axis) =>
      alpha * coordinate + (1 - alpha) * curve.controls[i - 1][axis])
  }
  const knots = [...curve.knots.slice(0, k + 1), u, ...curve.knots.slice(k + 1)]
  return { degree: p, controls, knots }
}

function clampRange(curve: NurbsCurve, a: number, b: number): NurbsCurve {
  let work = homogeneous(curve)
  while (multiplicity(work.knots, a) < work.degree + 1) work = insertHomogeneous(work, a)
  while (multiplicity(work.knots, b) < work.degree + 1) work = insertHomogeneous(work, b)
  const start = work.knots.indexOf(a)
  const end = work.knots.lastIndexOf(b)
  const knots = work.knots.slice(start, end + 1)
  const count = knots.length - work.degree - 1
  return fromHomogeneous({ degree: work.degree, knots, controls: work.controls.slice(start, start + count) })
}

export function insertNurbsKnot(curve: NurbsCurve, u: number, count = 1): NurbsCurve {
  validateNurbsCurve(curve)
  requireInput(Number.isInteger(count) && count >= 1 && count <= curve.degree + 1, 'Insertion count must be in [1, degree+1]')
  const domain = [curve.knots[curve.degree], curve.knots[curve.controlPoints.length]]
  requireInput(Number.isFinite(u) && u >= domain[0] && u <= domain[1], 'Inserted knot must be inside the active domain')
  const base = curve.periodic ? clampRange(curve, domain[0], domain[1]) : curve
  const maximum = u === domain[0] || u === domain[1] ? curve.degree + 1 : curve.degree
  requireInput(multiplicity(base.knots, u) + count <= maximum, 'Insertion would exceed the permitted knot multiplicity')
  requireBudget(base.controlPoints.length + count)
  let work = homogeneous(base)
  for (let i = 0; i < count; i++) work = insertHomogeneous(work, u)
  return fromHomogeneous(work)
}

export function trimNurbsCurve(curve: NurbsCurve, a: number, b: number): NurbsCurve {
  validateNurbsCurve(curve)
  const domain = [curve.knots[curve.degree], curve.knots[curve.controlPoints.length]]
  requireInput(Number.isFinite(a) && Number.isFinite(b) && domain[0] <= a && a < b && b <= domain[1],
    'Trim must be a nonempty ordered subdomain of the active knot domain')
  return clampRange(curve, a, b)
}

export function splitNurbsCurve(curve: NurbsCurve, u: number): [NurbsCurve, NurbsCurve] {
  validateNurbsCurve(curve)
  const domain = [curve.knots[curve.degree], curve.knots[curve.controlPoints.length]]
  requireInput(Number.isFinite(u) && domain[0] < u && u < domain[1], 'Split parameter must lie strictly inside the active domain')
  return [trimNurbsCurve(curve, domain[0], u), trimNurbsCurve(curve, u, domain[1])]
}

export function reverseNurbsCurve(curve: NurbsCurve): NurbsCurve {
  validateNurbsCurve(curve)
  const reflected = clone(curve)
  const sum = curve.knots[curve.degree] + curve.knots[curve.controlPoints.length]
  reflected.knots = [...curve.knots].reverse().map(knot => sum - knot)
  reflected.controlPoints.reverse()
  reflected.weights.reverse()
  validateNurbsCurve(reflected)
  return reflected
}

export function decomposeNurbsCurve(curve: NurbsCurve): { curve: NurbsCurve; domain: [number, number] }[] {
  validateNurbsCurve(curve)
  const low = curve.knots[curve.degree], high = curve.knots[curve.controlPoints.length]
  const breaks = [...new Set(curve.knots.filter(knot => knot >= low && knot <= high))]
  return breaks.slice(0, -1).map((start, index) => ({
    curve: trimNurbsCurve(curve, start, breaks[index + 1]), domain: [start, breaks[index + 1]],
  }))
}

/**
 * Homogeneous Bézier elevation followed by C0 recombination. Geometry and the
 * original parameter domain are retained. Interior knot multiplicity increases
 * to the new degree, so this operation deliberately does not claim to preserve
 * the original minimal knot vector or its encoded smoothness classification.
 */
export function elevateNurbsCurve(curve: NurbsCurve, degree: number): NurbsCurve {
  validateNurbsCurve(curve)
  requireInput(Number.isInteger(degree) && degree >= curve.degree && degree <= MAX_DEGREE,
    'Elevation degree must be between the current degree and 25')
  if (degree === curve.degree) return clone(curve)
  const low = curve.knots[curve.degree], high = curve.knots[curve.controlPoints.length]
  const spanCount = new Set(curve.knots.filter(knot => knot >= low && knot <= high)).size - 1
  // Admit the C0 output before allocating/refining individual Bézier pieces.
  requireBudget(spanCount * degree + 1)
  const segments = decomposeNurbsCurve(curve)
  const controls: number[][] = []
  const knots = new Array<number>(degree + 1).fill(segments[0].domain[0])
  for (let segmentIndex = 0; segmentIndex < segments.length; segmentIndex++) {
    const segment = segments[segmentIndex]
    let points = homogeneous(segment.curve).controls
    for (let order = curve.degree; order < degree; order++) {
      const next = [[...points[0]]]
      for (let i = 1; i <= order; i++) {
        const alpha = i / (order + 1)
        next.push(points[i].map((coordinate, axis) => alpha * points[i - 1][axis] + (1 - alpha) * coordinate))
      }
      next.push([...points[points.length - 1]])
      points = next
    }
    controls.push(...points.slice(segmentIndex === 0 ? 0 : 1))
    knots.push(...new Array<number>(segmentIndex === segments.length - 1 ? degree + 1 : degree).fill(segment.domain[1]))
  }
  return fromHomogeneous({ degree, controls, knots })
}

/**
 * Positive rational bases lie in their control polygon's convex hull. This box
 * is therefore a conservative mathematical enclosure (possibly loose). No
 * claim of outward-rounded floating-point certification is made.
 */
export function nurbsCurveBounds(curve: NurbsCurve): { min: number[]; max: number[] } {
  validateNurbsCurve(curve)
  const dimension = curve.controlPoints[0].length
  return {
    min: Array.from({ length: dimension }, (_, axis) => Math.min(...curve.controlPoints.map(point => point[axis]))),
    max: Array.from({ length: dimension }, (_, axis) => Math.max(...curve.controlPoints.map(point => point[axis]))),
  }
}
