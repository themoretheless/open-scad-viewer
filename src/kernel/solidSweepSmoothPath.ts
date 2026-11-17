import { DirectInputError } from '../i18n/kernelErrors'
import { evaluateNurbsCurvePoints, type NurbsCurve } from './nurbsCurve'
import { sampleCurveArrayPath } from './solidInstanceArrayCurvePath'
import { createPeriodicBrepSectionLoft, createRationalBrepSectionLoft, type NurbsBrep } from './geometry/brep'

type Vec3 = [number, number, number]

/** Default chordal tolerance for smooth sweep paths (C9), in mm. */
export const DEFAULT_SWEEP_PATH_TOLERANCE_MM = 0.01
const MAX_SWEEP_PATH_POINTS = 2_000

const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
const add = (a: Vec3, b: Vec3, s = 1): Vec3 => [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
const len = (a: Vec3) => Math.hypot(a[0], a[1], a[2])
const unit = (a: Vec3): Vec3 => {
  const l = len(a) || 1
  return [a[0] / l, a[1] / l, a[2] / l]
}
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
]
const lift = (q: readonly number[]): Vec3 => [q[0] ?? 0, q[1] ?? 0, q[2] ?? 0]

function refuse(message: string, ids: string[]): never {
  throw new DirectInputError('DIRECT_SWEEP_PATH', message, undefined, ids)
}

/** Sweep station frame: origin on the path, unit tangent `t`, and the profile axes `r`, `s`. */
export interface SweepPathFrame {
  origin: Vec3
  t: Vec3
  r: Vec3
  s: Vec3
}

/**
 * Rotation-minimizing frames along a polyline by the double reflection method
 * (Wang, Jüttler, Zheng, Liu 2008). Station tangents are the normalized sum of the
 * adjacent segment directions; the reference axis `r0` must be normal to the first
 * tangent. The frame never flips: each step is a pair of reflections, so `r` turns
 * only as much as the tangent does.
 */
export function sweepPathFrames(
  points: readonly Vec3[],
  r0: Vec3,
  closed = false,
  startTangent?: Vec3,
  endTangent?: Vec3,
): SweepPathFrame[] {
  const n = points.length
  const seg = (i: number) => unit(sub(points[i + 1]!, points[i]!))
  const tangents = points.map((_, i): Vec3 => {
    if (i === 0) return startTangent ?? (closed ? unit(add(seg(n - 2), seg(0))) : seg(0))
    if (i === n - 1) return endTangent ?? (closed ? unit(add(seg(n - 2), seg(0))) : seg(n - 2))
    return unit(add(seg(i - 1), seg(i)))
  })
  const t0 = tangents[0]!
  let r = unit(add(r0, t0, -dot(r0, t0)))
  const frames: SweepPathFrame[] = [{ origin: points[0]!, t: t0, r, s: cross(t0, r) }]
  for (let i = 0; i + 1 < n; i++) {
    const v1 = sub(points[i + 1]!, points[i]!),
      c1 = dot(v1, v1)
    const ti = tangents[i]!,
      tn = tangents[i + 1]!
    const rL = add(r, v1, (-2 * dot(v1, r)) / c1),
      tL = add(ti, v1, (-2 * dot(v1, ti)) / c1)
    const v2 = sub(tn, tL),
      c2 = dot(v2, v2)
    r = unit(c2 > 1e-30 ? add(rL, v2, (-2 * dot(v2, rL)) / c2) : rL)
    frames.push({ origin: points[i + 1]!, t: tn, r, s: cross(tn, r) })
  }
  return frames
}

function segmentDistance(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3): number {
  const d1 = sub(q1, p1),
    d2 = sub(q2, p2),
    r = sub(p1, p2)
  const a = dot(d1, d1),
    e = dot(d2, d2),
    f = dot(d2, r),
    c = dot(d1, r),
    b = dot(d1, d2)
  const den = a * e - b * b
  let s = den > 1e-18 ? Math.min(1, Math.max(0, (b * f - c * e) / den)) : 0
  let t = (b * s + f) / e
  if (t < 0) {
    t = 0
    s = Math.min(1, Math.max(0, -c / a))
  } else if (t > 1) {
    t = 1
    s = Math.min(1, Math.max(0, (b - c) / a))
  }
  return len(sub(add(p1, d1, s), add(p2, d2, t)))
}

/** Smallest circumradius over consecutive sample triples: the discrete curvature radius. */
export function minimumPathCurvatureRadius(points: readonly Vec3[], closed = false): number {
  const n = points.length
  let min = Infinity
  const triple = (a: Vec3, b: Vec3, c: Vec3) => {
    const ab = sub(b, a),
      bc = sub(c, b),
      ca = sub(a, c)
    const area2 = len(cross(ab, bc))
    if (area2 > 1e-18) min = Math.min(min, (len(ab) * len(bc) * len(ca)) / (2 * area2))
  }
  for (let i = 1; i + 1 < n; i++) triple(points[i - 1]!, points[i]!, points[i + 1]!)
  if (closed && n > 3) triple(points[n - 2]!, points[0]!, points[1]!)
  return min
}

/**
 * Samples an arc, circle or spline path for a solid sweep (C9) and checks that a profile
 * of radius `profileRadius` (largest distance from the path start to the profile) can travel
 * along it: the curvature radius must exceed the profile radius, and path stretches farther
 * apart along the curve than half a turn of the profile must stay more than two profile radii
 * apart, otherwise the swept tube would cut through itself. A closed path drops its repeated
 * end point. The exact end tangents are returned so the end stations stay normal to the curve.
 */
export function sampleSweepPath(
  curve: NurbsCurve,
  profileRadius: number,
  ids: string[],
  toleranceMm = DEFAULT_SWEEP_PATH_TOLERANCE_MM,
): { points: Vec3[]; closed: boolean; startTangent: Vec3; endTangent: Vec3 } {
  const sampled = sampleCurveArrayPath(curve, toleranceMm, message => refuse(message, ids))
  const closed = sampled.closed || !!curve.periodic
  const points = sampled.points.filter((q, i, all) => i === 0 || len(sub(q, all[i - 1]!)) > 1e-9)
  if (closed && points.length > 1 && len(sub(points[0]!, points.at(-1)!)) <= toleranceMm) points.pop()
  if (points.length < (closed ? 3 : 2)) refuse('The sweep path needs two distinct points.', ids)
  if (points.length > MAX_SWEEP_PATH_POINTS)
    refuse('The sweep path needs more than 2000 stations at this tolerance. Increase the tolerance.', ids)
  const loop = closed ? [...points, points[0]!] : points
  const minRadius = minimumPathCurvatureRadius(points, closed)
  if (!(minRadius > profileRadius * (1 + 1e-6)))
    refuse(
      `The sweep path bends tighter (radius ${minRadius.toFixed(4)} mm) than the profile radius ${profileRadius.toFixed(4)} mm.`,
      ids,
    )
  const arc = [0]
  for (let i = 1; i < loop.length; i++) arc.push(arc[i - 1]! + len(sub(loop[i]!, loop[i - 1]!)))
  const total = arc.at(-1)!,
    reach = Math.PI * profileRadius
  for (let i = 0; i + 1 < loop.length; i++)
    for (let j = i + 2; j + 1 < loop.length; j++) {
      let gap = arc[j]! - arc[i + 1]!
      if (closed) gap = Math.min(gap, total - arc[j + 1]! + arc[i]!)
      if (gap <= reach) continue
      const d = segmentDistance(loop[i]!, loop[i + 1]!, loop[j]!, loop[j + 1]!)
      if (d < 2 * profileRadius)
        refuse(
          d < toleranceMm
            ? 'The sweep path crosses itself.'
            : `The swept profile would cut through itself: path stretches come ${d.toFixed(4)} mm apart, closer than twice the profile radius.`,
          ids,
        )
    }
  const p = curve.degree,
    a = curve.knots[p]!,
    b = curve.knots[curve.knots.length - 1 - p]!
  // Second-order one-sided differences give the end tangents to ~h² accuracy.
  const h = (b - a) * 1e-5
  const [s0, s1, s2, e2, e1, e0] = evaluateNurbsCurvePoints(curve, [a, a + h, a + 2 * h, b - 2 * h, b - h, b]).map(lift)
  const oneSided = (q0: Vec3, q1: Vec3, q2: Vec3) => unit(add(add(q0, q1, -4 / 3), q2, 1 / 3).map(v => -v) as Vec3)
  return { points, closed, startTangent: oneSided(s0!, s1!, s2!), endTangent: oneSided(e0!, e1!, e2!).map(v => -v) as Vec3 }
}

const transformCurve = (curve: NurbsCurve, map: (q: Vec3) => Vec3): NurbsCurve => ({
  ...curve,
  controlPoints: curve.controlPoints.map(q => map(lift(q))),
})

/**
 * Solid sweep tool along an arc, circle or spline path (C9). The profile is placed at every
 * sampled station by a rotation-minimizing frame and the stations are lofted: an open path
 * gets planar end caps, a closed path becomes a ring whose last station repeats the first
 * (any frame holonomy of a non-planar loop is spread evenly over the stations, so the seam
 * closes without a twist jump). The profile must lie in the plane normal to the path start.
 */
export function buildSmoothSweepToolBrep(
  profile: NurbsCurve,
  path: NurbsCurve,
  ids: [string, string],
  toleranceMm = DEFAULT_SWEEP_PATH_TOLERANCE_MM,
): NurbsBrep {
  const [profileId, pathId] = ids
  const p = path.degree
  const start = lift(evaluateNurbsCurvePoints(path, [path.knots[p] ?? 0])[0] ?? [])
  const pa = profile.knots[profile.degree] ?? 0,
    pb = profile.knots[profile.knots.length - 1 - profile.degree] ?? 1
  const samples = evaluateNurbsCurvePoints(
    profile,
    Array.from({ length: 64 }, (_, i) => pa + ((pb - pa) * i) / 64),
  ).map(lift)
  const profileRadius = Math.max(...samples.map(q => len(sub(q, start))))
  const { points, closed, startTangent, endTangent } = sampleSweepPath(path, profileRadius, [pathId], toleranceMm)
  const scale = Math.max(1, profileRadius)
  if (profile.controlPoints.some(q => Math.abs(dot(sub(lift(q), start), startTangent)) > 1e-6 * scale))
    throw new DirectInputError(
      'DIRECT_SURFACE_INPUTS',
      'The profile must lie in the plane normal to the path at its start.',
      undefined,
      [profileId, pathId],
    )
  const loop = closed ? [...points, points[0]!] : points
  const axis = [0, 1, 2].reduce((m, i) => (Math.abs(startTangent[i]!) < Math.abs(startTangent[m]!) ? i : m), 0)
  const r0 = unit(cross(startTangent, [0, 1, 2].map(i => (i === axis ? 1 : 0)) as Vec3))
  const frames = sweepPathFrames(loop, r0, closed, closed ? undefined : startTangent, closed ? undefined : endTangent)
  const f0 = frames[0]!
  if (closed) {
    // Spread the holonomy so the last frame equals the first one.
    const last = frames.at(-1)!
    const holonomy = Math.atan2(dot(cross(last.r, f0.r), f0.t), dot(last.r, f0.r))
    const arc = [0]
    for (let i = 1; i < loop.length; i++) arc.push(arc[i - 1]! + len(sub(loop[i]!, loop[i - 1]!)))
    frames.forEach((f, i) => {
      const angle = (holonomy * arc[i]!) / arc.at(-1)!,
        c = Math.cos(angle),
        sn = Math.sin(angle)
      const r = add(f.r.map(v => v * c) as Vec3, f.s, sn)
      f.r = r
      f.s = cross(f.t, r)
    })
  }
  const place = (f: SweepPathFrame) =>
    transformCurve(profile, q => {
      const d = sub(q, f0.origin),
        x = dot(d, f0.r),
        y = dot(d, f0.s),
        z = dot(d, f0.t)
      return add(add(add(f.origin, f.r, x), f.s, y), f.t, z)
    })
  const base = transformCurve(profile, q => q)
  const sections = frames.map((f, i) => [[i === 0 || (closed && i === frames.length - 1) ? base : place(f)]])
  if (closed) return createPeriodicBrepSectionLoft(sections)
  return createRationalBrepSectionLoft(sections)
}
