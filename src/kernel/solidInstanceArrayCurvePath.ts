import { DirectInputError } from '../i18n/kernelErrors'
import { evaluateNurbsCurvePoints, type NurbsCurve } from './nurbsCurve'

type Vec3 = [number, number, number]

export const DEFAULT_CURVE_PATH_TOLERANCE_MM = 0.01
const MAX_PATH_POINTS = 10_000

/**
 * Samples a NURBS curve as a path-array polyline with an explicit tolerance:
 * every parameter interval is split until the exact curve point at its
 * midpoint lies within `toleranceMm` of the chord midpoint. Arc-length spacing
 * on the result is accurate to that chord sag. `closed` is true when the curve
 * ends where it starts.
 */
export function sampleCurveArrayPath(
  curve: NurbsCurve,
  toleranceMm = DEFAULT_CURVE_PATH_TOLERANCE_MM,
  refuse: (message: string) => never = message => {
    throw new DirectInputError('DIRECT_PATTERN_ARRAY', message, 'patternCount', [])
  },
): { points: Vec3[]; closed: boolean } {
  if (!(toleranceMm >= 1e-6 && toleranceMm <= 10)) refuse('Curve path tolerance must be between 0.000001 and 10 mm.')
  const p = curve.degree,
    a = curve.knots[p],
    b = curve.knots[curve.knots.length - 1 - p]
  if (!(a !== undefined && b !== undefined && Number.isFinite(a) && Number.isFinite(b) && b > a))
    refuse('Select a NURBS curve with a non-empty domain as the array path.')
  const at = (parameters: number[]): Vec3[] =>
    evaluateNurbsCurvePoints(curve, parameters).map(q => [q[0] ?? 0, q[1] ?? 0, q[2] ?? 0])
  let params = Array.from({ length: 17 }, (_, i) => a! + ((b! - a!) * i) / 16),
    points = at(params)
  for (;;) {
    const mids = params.slice(1).map((u, i) => (u + params[i]!) / 2),
      exact = at(mids)
    const split = exact.map((m, i) => {
      const c = points[i]!,
        d = points[i + 1]!
      return Math.hypot(m[0] - (c[0] + d[0]) / 2, m[1] - (c[1] + d[1]) / 2, m[2] - (c[2] + d[2]) / 2) > toleranceMm
    })
    if (!split.some(Boolean)) break
    const nextParams = [params[0]!],
      nextPoints = [points[0]!]
    for (let i = 0; i < mids.length; i++) {
      if (split[i]) {
        nextParams.push(mids[i]!)
        nextPoints.push(exact[i]!)
      }
      nextParams.push(params[i + 1]!)
      nextPoints.push(points[i + 1]!)
    }
    if (nextPoints.length > MAX_PATH_POINTS)
      refuse('The curve path needs more than 10000 samples at this tolerance. Increase the tolerance.')
    params = nextParams
    points = nextPoints
  }
  const first = points[0]!,
    last = points[points.length - 1]!
  const closed = Math.hypot(first[0] - last[0], first[1] - last[1], first[2] - last[2]) <= toleranceMm
  return { points, closed }
}
