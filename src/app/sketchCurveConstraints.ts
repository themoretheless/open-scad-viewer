import type { Point2 } from './directModeling'
import { sampleCurve, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'
import {
  solveRushGraphSketch,
  type SketchCircle,
  type SketchConstraint as SolverConstraint,
  type SketchPoint,
} from '../language/rushGraphSketch'
import { SKETCH_CONSTRAINT_TOLERANCE_MM } from './sketchConstraints'

/**
 * Persistent constraint of a circle or arc sketch against another sketch on the same plane. The constrained
 * sketch owns it and moves (centre and radius); the referenced sketch is held fixed.
 */
export type SketchCurveConstraint =
  | { kind: 'concentric' | 'equal_radius'; ref: string }
  | { kind: 'tangent_circle'; ref: string; internal: boolean }
  | { kind: 'tangent_line'; ref: string; a: number; b: number }

export const MAX_SKETCH_CURVE_CONSTRAINTS = 4

interface CurveSketch {
  id: string
  points: Point2[]
  closed?: boolean
  analytic?: AnalyticCurve
  plane?: SketchPlane
  retainedProfile?: unknown
  editablePath?: unknown
  dimensions?: { kind: string }[]
  curveConstraints?: SketchCurveConstraint[]
}

const samePlane = (p?: SketchPlane, q?: SketchPlane) => JSON.stringify(p ?? null) === JSON.stringify(q ?? null)

const isPolyline = (s: CurveSketch) => !s.analytic && !s.retainedProfile && !s.editablePath

/** Structural check of one sketch's curve constraints against the other sketches of the document. */
export function validateSketchCurveConstraints(sketch: CurveSketch, sketches: readonly CurveSketch[]): void {
  const list = sketch.curveConstraints
  if (list === undefined) return
  if (!Array.isArray(list) || list.length > MAX_SKETCH_CURVE_CONSTRAINTS)
    throw Error('Invalid sketch curve constraints.')
  if (!list.length) return
  if (!sketch.analytic) throw Error('Curve constraints require a circle or arc sketch.')
  for (const c of list) {
    if (!c || typeof c !== 'object' || typeof c.ref !== 'string' || c.ref === sketch.id)
      throw Error('Invalid sketch curve constraints.')
    const ref = sketches.find(s => s.id === c.ref)
    if (!ref) throw Error('A curve constraint references a missing sketch.')
    if (!samePlane(ref.plane, sketch.plane)) throw Error('A curve constraint needs both sketches on one plane.')
    const keys = Object.keys(c).sort().join(',')
    if (c.kind === 'concentric' || c.kind === 'equal_radius' || c.kind === 'tangent_circle') {
      if (keys !== (c.kind === 'tangent_circle' ? 'internal,kind,ref' : 'kind,ref'))
        throw Error('Invalid sketch curve constraints.')
      if (c.kind === 'tangent_circle' && typeof c.internal !== 'boolean')
        throw Error('Invalid sketch curve constraints.')
      if (!ref.analytic) throw Error('This curve constraint needs a circle or arc reference.')
    } else if (c.kind === 'tangent_line') {
      const n = ref.points.length
      const ok = (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) < n
      if (keys !== 'a,b,kind,ref' || !isPolyline(ref) || !ok(c.a) || !ok(c.b))
        throw Error('Invalid sketch curve constraints.')
      const segment = Math.abs(c.a - c.b) === 1 || (ref.closed === true && Math.abs(c.a - c.b) === n - 1)
      if (!segment) throw Error('A tangent constraint needs a segment of the reference polyline.')
    } else throw Error('Invalid sketch curve constraints.')
  }
  if (new Set(list.map(c => JSON.stringify(c))).size !== list.length) throw Error('Duplicate sketch curve constraint.')
}

/** Residual of one curve constraint in millimetres for the given circle. */
export function sketchCurveConstraintResidual(
  curve: Pick<AnalyticCurve, 'center' | 'radius'>,
  c: SketchCurveConstraint,
  ref: CurveSketch,
): number {
  const [cx, cy] = curve.center
  if (c.kind === 'tangent_line') {
    const a = ref.points[c.a]!,
      b = ref.points[c.b]!
    const dx = b[0] - a[0],
      dy = b[1] - a[1],
      len = Math.hypot(dx, dy)
    if (!(len > 0)) return Infinity
    return Math.abs(Math.abs((cx - a[0]) * dy - (cy - a[1]) * dx) / len - curve.radius)
  }
  const other = ref.analytic!
  const d = Math.hypot(cx - other.center[0], cy - other.center[1])
  if (c.kind === 'concentric') return d
  if (c.kind === 'equal_radius') return Math.abs(curve.radius - other.radius)
  return Math.abs(
    d -
      (c.kind === 'tangent_circle' && c.internal ? Math.abs(curve.radius - other.radius) : curve.radius + other.radius),
  )
}

/**
 * Moves the circle or arc of `sketch` so all its curve constraints hold with the referenced sketches fixed.
 * A radius or diameter dimension holds the radius. Throws a conflict error instead of returning a violating sketch.
 */
export function solveSketchCurveConstraints<T extends CurveSketch>(sketch: T, sketches: readonly CurveSketch[]): T {
  validateSketchCurveConstraints(sketch, sketches)
  const list = sketch.curveConstraints ?? []
  const curve = sketch.analytic
  if (!list.length || !curve) return sketch
  const points: SketchPoint[] = [{ id: 'c', position: [...curve.center] as Point2 }]
  const circles: SketchCircle[] = [{ id: 'k', center: 'c', radius: curve.radius }]
  const constraints: SolverConstraint[] = []
  const fixedPoint = (position: Point2) => {
    const id = `r${points.length}`
    points.push({ id, position: [...position] as Point2 })
    constraints.push({ id: `fix-${id}`, kind: 'fix', point: id, at: [...position] as Point2 })
    return id
  }
  const refCircles = new Map<string, string>()
  const refCircle = (ref: CurveSketch) => {
    const known = refCircles.get(ref.id)
    if (known) return known
    const id = `k${circles.length}`
    circles.push({ id, center: fixedPoint(ref.analytic!.center), radius: ref.analytic!.radius })
    constraints.push({ id: `radius-${id}`, kind: 'radius', circle: id, value: ref.analytic!.radius })
    refCircles.set(ref.id, id)
    return id
  }
  list.forEach((c, i) => {
    const ref = sketches.find(s => s.id === c.ref)!
    const id = `curve-${i}`
    if (c.kind === 'tangent_line')
      constraints.push({
        id,
        kind: 'tangent_line_circle',
        a: fixedPoint(ref.points[c.a]!),
        b: fixedPoint(ref.points[c.b]!),
        circle: 'k',
      })
    else if (c.kind === 'tangent_circle')
      constraints.push({ id, kind: 'tangent_circles', a: 'k', b: refCircle(ref), internal: c.internal })
    else constraints.push({ id, kind: c.kind, a: 'k', b: refCircle(ref) })
  })
  if (sketch.dimensions?.some(d => d.kind === 'radius' || d.kind === 'diameter'))
    constraints.push({ id: 'held-radius', kind: 'radius', circle: 'k', value: curve.radius })
  // The native solver needs at least three points; pad with held points far from nothing in particular.
  while (points.length < 3) fixedPoint([points.length * 1000, 0])
  const solved = solveRushGraphSketch(points, constraints, SKETCH_CONSTRAINT_TOLERANCE_MM, circles)
  const out = solved.circles?.[0]
  if (
    !out ||
    solved.status === 'inconsistent' ||
    solved.status === 'not_converged' ||
    solved.degenerate_constraints.length ||
    solved.maximum_residual_mm > SKETCH_CONSTRAINT_TOLERANCE_MM ||
    !(out.radius > 1e-4)
  )
    throw Error('Sketch curve constraints conflict or did not converge.')
  const analytic: AnalyticCurve = { ...curve, center: [...out.center] as Point2, radius: out.radius }
  for (const c of list)
    if (
      sketchCurveConstraintResidual(
        analytic,
        c,
        sketches.find(s => s.id === c.ref)!,
      ) > 1e-5
    )
      throw Error('Sketch curve constraints conflict or did not converge.')
  return { ...sketch, analytic, points: sampleCurve(analytic) }
}

/** Adds a curve constraint to `sketch` and solves it together with the existing ones. */
export function addSketchCurveConstraint<T extends CurveSketch>(
  sketch: T,
  constraint: SketchCurveConstraint,
  sketches: readonly CurveSketch[],
): T {
  return solveSketchCurveConstraints(
    { ...sketch, curveConstraints: [...(sketch.curveConstraints ?? []), constraint] },
    sketches,
  )
}

const CURVE_CONSTRAINT_NAMES: Record<SketchCurveConstraint['kind'], [string, string]> = {
  concentric: ['концентричность', 'concentric'],
  equal_radius: ['равный радиус', 'equal radius'],
  tangent_circle: ['касание окружностей', 'tangent circles'],
  tangent_line: ['касание отрезка', 'tangent to segment'],
}

/** Localized constraint name for panels and notices. */
export function sketchCurveConstraintName(c: SketchCurveConstraint, locale: string): string {
  const [ru, en] = CURVE_CONSTRAINT_NAMES[c.kind]
  const internal = c.kind === 'tangent_circle' && c.internal
  return locale === 'ru' ? `${ru}${internal ? ' (внутреннее)' : ''}` : `${en}${internal ? ' (internal)' : ''}`
}
