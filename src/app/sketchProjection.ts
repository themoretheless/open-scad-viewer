import { DirectInputError } from '../i18n/kernelErrors'
import { cross3, dot3, type Vec3 } from '../kernel/geometry/math3d'
import type { DirectSketch, Point2 } from './directModeling'
import { sampleCurve, worldPoint, xyPlane, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'

/** Planes whose normals differ by less than this (sine of the angle) count as parallel. */
export const PROJECTION_PARALLEL_TOLERANCE = 1e-9
const MIN_SEGMENT_MM = 1e-6

export interface ProjectedSketch {
  points: Point2[]
  closed: boolean
  analytic?: AnalyticCurve
  plane: SketchPlane
}

const sub = (a: Vec3, b: ArrayLike<number>): Vec3 => [a[0] - b[0]!, a[1] - b[1]!, a[2] - b[2]!]
const local = (p: Vec3, plane: SketchPlane): Point2 => {
  const d = sub(p, plane.origin)
  return [dot3(d, plane.u), dot3(d, plane.v)]
}
const refuseInput = (message: string) => new DirectInputError('DIRECT_SKETCH_PROJECTION_INPUT', message)

/**
 * Orthogonal projection of a sketch's edges onto `target` as a new sketch in target coordinates. Only exact
 * results are produced: polyline segments stay segments on any plane (a segment that collapses to a point is
 * refused), and circles and arcs are projected only from a plane parallel to the target, where projection is an
 * isometry (an arc seen from the opposite side keeps its geometry with the sweep reversed). A circle or arc on a
 * non-parallel plane would become an ellipse and is refused with `DIRECT_SKETCH_PROJECTION_NOT_EXACT`.
 */
export function projectSketchToPlane(
  source: Pick<DirectSketch, 'points' | 'closed' | 'analytic' | 'plane' | 'retainedProfile' | 'editablePath'>,
  target: SketchPlane,
): ProjectedSketch {
  if (source.retainedProfile || source.editablePath)
    throw refuseInput('Only polyline, circle and arc sketches can be projected.')
  const from = source.plane ?? xyPlane()
  const nTarget = cross3(target.u, target.v),
    nSource = cross3(from.u, from.v)
  const plane: SketchPlane = {
    origin: [...target.origin] as Vec3,
    u: [...target.u] as Vec3,
    v: [...target.v] as Vec3,
  }
  const curve = source.analytic
  if (curve) {
    const side = cross3(nSource, nTarget)
    if (Math.hypot(...side) > PROJECTION_PARALLEL_TOLERANCE)
      throw new DirectInputError(
        'DIRECT_SKETCH_PROJECTION_NOT_EXACT',
        'A circle or arc on a non-parallel plane projects to an ellipse; refusing an approximation.',
      )
    const center = local(worldPoint(curve.center, from), plane)
    const rad = (curve.start * Math.PI) / 180
    const dir: Vec3 = [0, 1, 2].map(k => Math.cos(rad) * from.u[k]! + Math.sin(rad) * from.v[k]!) as Vec3
    const start = (Math.atan2(dot3(dir, plane.v), dot3(dir, plane.u)) * 180) / Math.PI
    const flipped = dot3(nSource, nTarget) < 0
    const analytic: AnalyticCurve = {
      ...curve,
      center,
      start: curve.kind === 'circle' ? curve.start : start,
      sweep: flipped && curve.kind === 'arc' ? -curve.sweep : curve.sweep,
    }
    if (![center[0], center[1], analytic.start].every(Number.isFinite)) throw refuseInput('Projection is not finite.')
    return { points: sampleCurve(analytic), closed: source.closed === true, analytic, plane }
  }
  if (source.points.length < 2) throw refuseInput('A projected polyline needs at least two points.')
  const points = source.points.map(p => local(worldPoint(p, from), plane))
  if (!points.every(p => Number.isFinite(p[0]) && Number.isFinite(p[1]))) throw refuseInput('Projection is not finite.')
  const n = points.length
  for (let i = 0; i < (source.closed ? n : n - 1); i++) {
    const a = points[i]!,
      b = points[(i + 1) % n]!
    if (Math.hypot(b[0] - a[0], b[1] - a[1]) <= MIN_SEGMENT_MM)
      throw refuseInput('A projected segment collapses to a point.')
  }
  return { points, closed: source.closed === true, plane }
}
