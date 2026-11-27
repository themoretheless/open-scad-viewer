import { COINCIDENT_EDIT_REFUSED_MESSAGE, CURVE_DRAG_REFUSED_MESSAGE } from './sketchCurveConstraintNotices'
import type { Point2 } from './directModeling'
import { sampleCurve, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'
import {
  solveRushGraphSketch,
  type SketchCircle,
  type SketchConstraint as SolverConstraint,
  type SketchPoint,
} from '../language/rushGraphSketch'
import { SKETCH_CONSTRAINT_TOLERANCE_MM, solveConstrainedPoints, type SketchConstraint } from './sketchConstraints'
import { dimensionSolverExtras } from './directDimensions'
import type { DirectSketch } from './directModeling'

/**
 * Persistent constraint of a sketch against another sketch on the same plane. The constrained sketch owns it
 * and moves; the referenced sketch is held fixed. Circle and arc sketches own the curve kinds (their centre and
 * radius move); polyline sketches own `coincident`: vertex `point` lies on vertex `target` of a reference
 * polyline or on the centre of a reference circle or arc.
 */
export type SketchCurveConstraint =
  | { kind: 'concentric' | 'equal_radius'; ref: string }
  | { kind: 'tangent_circle'; ref: string; internal: boolean }
  /** `point` (one of `a`, `b`) also keeps the arc's start point on that polyline vertex (a tangent arc). */
  | { kind: 'tangent_line'; ref: string; a: number; b: number; point?: number }
  | SketchCoincidentConstraint

export type SketchCoincidentConstraint = { kind: 'coincident'; ref: string; point: number; target: number | 'center' }

export const MAX_SKETCH_CURVE_CONSTRAINTS = 4
export const MAX_SKETCH_COINCIDENT_CONSTRAINTS = 16

interface CurveSketch {
  id: string
  points: Point2[]
  closed?: boolean
  analytic?: AnalyticCurve
  constraints?: SketchConstraint[]
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
  if (!Array.isArray(list)) throw Error('Invalid sketch curve constraints.')
  if (!list.length) return
  const polyline = isPolyline(sketch)
  if (!sketch.analytic && !polyline) throw Error('Curve constraints require a circle or arc sketch.')
  if (list.length > (sketch.analytic ? MAX_SKETCH_CURVE_CONSTRAINTS : MAX_SKETCH_COINCIDENT_CONSTRAINTS))
    throw Error('Invalid sketch curve constraints.')
  for (const c of list) {
    if (!c || typeof c !== 'object' || typeof c.ref !== 'string' || c.ref === sketch.id)
      throw Error('Invalid sketch curve constraints.')
    const ref = sketches.find(s => s.id === c.ref)
    if (!ref) throw Error('A curve constraint references a missing sketch.')
    if (!samePlane(ref.plane, sketch.plane)) throw Error('A curve constraint needs both sketches on one plane.')
    const keys = Object.keys(c).sort().join(',')
    if (c.kind === 'coincident') {
      if (!polyline) throw Error('A coincident constraint requires a polyline sketch.')
      const own = Number.isInteger(c.point) && c.point >= 0 && c.point < sketch.points.length
      if (keys !== 'kind,point,ref,target' || !own) throw Error('Invalid sketch curve constraints.')
      if (c.target === 'center') {
        if (!ref.analytic) throw Error('A coincident centre needs a circle or arc reference.')
      } else if (!isPolyline(ref) || !Number.isInteger(c.target) || c.target < 0 || c.target >= ref.points.length)
        throw Error('A coincident vertex needs a vertex of a reference polyline.')
      continue
    }
    if (!sketch.analytic) throw Error('Curve constraints require a circle or arc sketch.')
    if (c.kind === 'concentric' || c.kind === 'equal_radius' || c.kind === 'tangent_circle') {
      if (keys !== (c.kind === 'tangent_circle' ? 'internal,kind,ref' : 'kind,ref'))
        throw Error('Invalid sketch curve constraints.')
      if (c.kind === 'tangent_circle' && typeof c.internal !== 'boolean')
        throw Error('Invalid sketch curve constraints.')
      if (!ref.analytic) throw Error('This curve constraint needs a circle or arc reference.')
    } else if (c.kind === 'tangent_line') {
      const n = ref.points.length
      const ok = (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) < n
      if (!(keys === 'a,b,kind,ref' || keys === 'a,b,kind,point,ref') || !isPolyline(ref) || !ok(c.a) || !ok(c.b))
        throw Error('Invalid sketch curve constraints.')
      if (c.point !== undefined && ((c.point !== c.a && c.point !== c.b) || sketch.analytic.kind !== 'arc'))
        throw Error('A tangent arc start needs an arc and an end of the tangent segment.')
      const segment = Math.abs(c.a - c.b) === 1 || (ref.closed === true && Math.abs(c.a - c.b) === n - 1)
      if (!segment) throw Error('A tangent constraint needs a segment of the reference polyline.')
    } else throw Error('Invalid sketch curve constraints.')
  }
  if (new Set(list.map(c => JSON.stringify(c))).size !== list.length) throw Error('Duplicate sketch curve constraint.')
}

/** Position a coincident constraint pins its vertex to, or `undefined` when the reference cannot supply it. */
export function coincidentTarget(c: SketchCoincidentConstraint, ref: CurveSketch): Point2 | undefined {
  return c.target === 'center' ? ref.analytic?.center : ref.points[c.target]
}

/** Distance in millimetres between a coincident vertex and its target. */
export function sketchCoincidentResidual(
  points: readonly Point2[],
  c: SketchCoincidentConstraint,
  ref: CurveSketch,
): number {
  const p = points[c.point],
    q = coincidentTarget(c, ref)
  return p && q ? Math.hypot(p[0] - q[0], p[1] - q[1]) : Infinity
}

/** Residual of one constraint in millimetres for the owning sketch (circle centre and radius, or points). */
export function sketchConstraintOwnerResidual(sketch: CurveSketch, c: SketchCurveConstraint, ref: CurveSketch): number {
  if (c.kind === 'coincident') return sketchCoincidentResidual(sketch.points, c, ref)
  return sketch.analytic ? sketchCurveConstraintResidual(sketch.analytic, c, ref) : Infinity
}

/** Residual of one curve constraint in millimetres for the given circle. */
export function sketchCurveConstraintResidual(
  curve: Pick<AnalyticCurve, 'center' | 'radius'> & { start?: number },
  c: Exclude<SketchCurveConstraint, SketchCoincidentConstraint>,
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
    const tangency = Math.abs(Math.abs((cx - a[0]) * dy - (cy - a[1]) * dx) / len - curve.radius)
    const p = c.point === undefined ? undefined : ref.points[c.point]
    if (!p) return c.point === undefined ? tangency : Infinity
    if (curve.start === undefined) return Infinity
    const rad = (curve.start * Math.PI) / 180
    const start = Math.hypot(cx + curve.radius * Math.cos(rad) - p[0], cy + curve.radius * Math.sin(rad) - p[1])
    return Math.max(tangency, start)
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
export function solveSketchCurveConstraints<T extends CurveSketch>(
  sketch: T,
  sketches: readonly CurveSketch[],
  held: readonly number[] = [],
): T {
  validateSketchCurveConstraints(sketch, sketches)
  const all = sketch.curveConstraints ?? []
  const curve = sketch.analytic
  if (all.length && !curve) return solveCoincidentPolyline(sketch, sketches, held)
  const list = all.filter(c => c.kind !== 'coincident')
  if (!list.length || !curve) return sketch
  const anchored = list.find(c => c.kind === 'tangent_line' && c.point !== undefined)
  if (anchored) return solveAnchoredTangentArc(sketch, anchored as TangentLine, list, sketches)
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

type TangentLine = Extract<SketchCurveConstraint, { kind: 'tangent_line' }>

/**
 * A tangent arc anchored at a polyline vertex has a closed form: the arc starts at the vertex, leaves it along the
 * segment direction, keeps its radius and sweep, and its centre sits on the side given by the sweep sign. Any other
 * curve constraint on the same arc must already hold at that position, otherwise the edit is refused.
 */
function solveAnchoredTangentArc<T extends CurveSketch>(
  sketch: T,
  c: TangentLine,
  list: readonly SketchCurveConstraint[],
  sketches: readonly CurveSketch[],
): T {
  const curve = sketch.analytic!
  const ref = sketches.find(s => s.id === c.ref)!
  const p = ref.points[c.point!]!,
    q = ref.points[c.point === c.a ? c.b : c.a]!
  const len = Math.hypot(p[0] - q[0], p[1] - q[1])
  if (!(len > 1e-9)) throw Error('Sketch curve constraints conflict or did not converge.')
  const tx = (p[0] - q[0]) / len,
    ty = (p[1] - q[1]) / len,
    signed = curve.sweep > 0 ? curve.radius : -curve.radius
  const center: Point2 = [p[0] - ty * signed, p[1] + tx * signed]
  const start = (Math.atan2(p[1] - center[1], p[0] - center[0]) * 180) / Math.PI
  const analytic: AnalyticCurve = { ...curve, center, start }
  for (const other of list)
    if (
      other.kind !== 'coincident' &&
      sketchCurveConstraintResidual(
        analytic,
        other,
        sketches.find(s => s.id === other.ref)!,
      ) > 1e-5
    )
      throw Error('Sketch curve constraints conflict or did not converge.')
  return { ...sketch, analytic, points: sampleCurve(analytic) }
}

/** Adds a curve constraint to `sketch` and solves it together with the existing ones. */
export const COINCIDENT_CONFLICT_MESSAGE = 'Sketch coincident constraints conflict or did not converge.'

/**
 * Moves the points of a polyline so each coincident vertex lies on its target, with the target sketches fixed,
 * the polyline's own constraints kept, its dimensions held at their current values and the `held` points fixed.
 */
function solveCoincidentPolyline<T extends CurveSketch>(
  sketch: T,
  sketches: readonly CurveSketch[],
  held: readonly number[],
  dimensionSource: CurveSketch = sketch,
): T {
  const list = (sketch.curveConstraints ?? []) as SketchCoincidentConstraint[]
  const pinned = new Map<number, Point2>()
  const fixes: SolverConstraint[] = []
  const pin = (point: number, at: Point2, id: string) => {
    const known = pinned.get(point)
    if (known) {
      if (Math.hypot(known[0] - at[0], known[1] - at[1]) > SKETCH_CONSTRAINT_TOLERANCE_MM)
        throw Error(COINCIDENT_CONFLICT_MESSAGE)
      return
    }
    pinned.set(point, at)
    fixes.push({ id, kind: 'fix', point: `p${point}`, at: [...at] as Point2 })
  }
  // Held points first: an edit that moved a coincident vertex off its target conflicts instead of snapping back.
  for (const i of held) {
    const p = sketch.points[i]
    if (p) pin(i, p, `held-${i}`)
  }
  const refOf = (c: SketchCoincidentConstraint) => sketches.find(s => s.id === c.ref)!
  const targets = list.map(c => coincidentTarget(c, refOf(c))!)
  list.forEach((c, i) => pin(c.point, targets[i]!, `coincident-${i}`))
  let points: Point2[]
  try {
    const extras = dimensionSource.dimensions?.length
      ? dimensionSolverExtras(dimensionSource as unknown as DirectSketch)
      : { constraints: [] as SolverConstraint[], points: [] as Point2[] }
    const aux = [...(extras.points ?? [])]
    const constraints = [...extras.constraints, ...fixes]
    // The native solver needs at least three points; pad with held auxiliary points.
    while (sketch.points.length + aux.length < 3) {
      const at: Point2 = [aux.length * 1000 + 1000, 0]
      constraints.push({ id: `pad-${aux.length}`, kind: 'fix', point: `x${aux.length}`, at })
      aux.push(at)
    }
    points = solveConstrainedPoints(sketch.points, sketch.constraints ?? [], { ...extras, points: aux, constraints })
  } catch {
    throw Error(COINCIDENT_CONFLICT_MESSAGE)
  }
  // The solver meets its tolerance; put the pinned vertices exactly on their targets.
  list.forEach((c, i) => (points[c.point] = [...targets[i]!] as Point2))
  const n = points.length
  for (let i = 0; i < (sketch.closed ? n : n - 1); i++) {
    const a = points[i]!,
      b = points[(i + 1) % n]!
    if (Math.hypot(b[0] - a[0], b[1] - a[1]) <= 1e-4) throw Error(COINCIDENT_CONFLICT_MESSAGE)
  }
  for (const c of list)
    if (sketchCoincidentResidual(points, c, refOf(c)) > 1e-5) throw Error(COINCIDENT_CONFLICT_MESSAGE)
  return { ...sketch, points }
}

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

/**
 * Drops curve constraints whose referenced sketch is missing or no longer on the same plane (for example
 * after a delete). Returns the same array when nothing changed.
 */
export function pruneSketchCurveConstraints<T extends CurveSketch>(sketches: readonly T[]): T[] {
  const byId = new Map(sketches.map(s => [s.id, s]))
  let changed = false
  const out = sketches.map(s => {
    if (!Array.isArray(s.curveConstraints) || !s.curveConstraints.length) return s
    const kept = s.curveConstraints.filter(c => {
      const ref = c && typeof c === 'object' ? byId.get(c.ref) : undefined
      if (ref === undefined || ref === s || !samePlane(ref.plane, s.plane)) return false
      // A coincident whose vertex or target vanished (the reference changed kind or lost points) is dropped too.
      if (c.kind === 'coincident')
        return (
          isPolyline(s) &&
          c.point >= 0 &&
          c.point < s.points.length &&
          (c.target === 'center' ? !!ref.analytic : isPolyline(ref) && c.target >= 0 && c.target < ref.points.length)
        )
      return true
    })
    if (kept.length === s.curveConstraints.length) return s
    changed = true
    const next = { ...s }
    if (kept.length) next.curveConstraints = kept
    else delete next.curveConstraints
    return next
  })
  return changed ? out : (sketches as T[])
}

/**
 * Re-solves every circle or arc sketch whose curve constraints reference one of the `changed` sketches, and
 * then the sketches that depend on those, so dependents follow an edit of the referenced geometry. Each
 * dependent is solved once (a reference cycle stops there). Throws when a dependent cannot follow, so the
 * caller refuses the whole edit. Returns the same array when no sketch depends on the changed ones.
 */
export function resolveDependentCurveSketches<T extends CurveSketch>(
  sketches: readonly T[],
  changed: readonly string[],
): T[] {
  let out = sketches as T[]
  const moved = new Set(changed)
  const solved = new Set(changed)
  for (let progress = true; progress;) {
    progress = false
    for (let i = 0; i < out.length; i++) {
      const s = out[i]!
      if (solved.has(s.id) || !s.curveConstraints?.some(c => moved.has(c.ref))) continue
      let next: T
      try {
        next = solveSketchCurveConstraints(s, out)
      } catch {
        throw Error(`A dependent circle or arc sketch cannot follow this edit; the edit was not applied.`)
      }
      if (out === sketches) out = [...sketches]
      out[i] = next
      solved.add(s.id)
      moved.add(s.id)
      progress = true
    }
  }
  return out
}

/**
 * Drops the curve constraints that reference `ref` (for example a polyline whose points a trim renumbered)
 * and reports them, so the caller can tell the user.
 */
export function dropCurveConstraintsTo<T extends CurveSketch>(
  sketches: readonly T[],
  ref: string,
): { sketches: T[]; dropped: SketchCurveConstraint[] } {
  const dropped: SketchCurveConstraint[] = []
  const out = sketches.map(s => {
    if (!s.curveConstraints?.some(c => c.ref === ref)) return s
    dropped.push(...s.curveConstraints.filter(c => c.ref === ref))
    const kept = s.curveConstraints.filter(c => c.ref !== ref)
    const next = { ...s }
    if (kept.length) next.curveConstraints = kept
    else delete next.curveConstraints
    return next
  })
  return { sketches: dropped.length ? out : (sketches as T[]), dropped }
}

/** Notice for curve constraints a trim removed, or an empty string. */
/**
 * Re-solves the curve constraints of a circle or arc sketch after one of its own handles was dragged, starting
 * from the dragged shape. Returns the sketch unchanged when it has no curve constraints; throws
 * `CURVE_DRAG_REFUSED_MESSAGE` instead of returning a violating sketch.
 */
export function solveDraggedCurveSketch<T extends CurveSketch>(sketch: T, sketches: readonly CurveSketch[]): T {
  if (!sketch.analytic || !sketch.curveConstraints?.length) return sketch
  try {
    return solveSketchCurveConstraints(
      sketch,
      sketches.map(o => (o.id === sketch.id ? sketch : o)),
    )
  } catch {
    throw Error(CURVE_DRAG_REFUSED_MESSAGE)
  }
}

/**
 * Keeps the coincident constraints of a polyline after an edit of its own points (vertex drag, extend, offset):
 * the points the edit moved are held unless every point moved, and the coincident vertices are pinned to their
 * targets again. Returns `after` unchanged without coincident constraints; throws
 * `COINCIDENT_EDIT_REFUSED_MESSAGE` instead of returning a violating sketch.
 */
export function carryCoincidentConstraints<T extends CurveSketch>(
  before: CurveSketch,
  after: T,
  sketches: readonly CurveSketch[],
): T {
  if (after.analytic || !after.curveConstraints?.some(c => c.kind === 'coincident')) return after
  try {
    // Dimensions are held at their values before the edit, as `carrySketchConstraints` does.
    if (after.points.length !== before.points.length) throw Error(COINCIDENT_EDIT_REFUSED_MESSAGE)
    const moved = after.points.flatMap((p, i) =>
      p[0] === before.points[i]![0] && p[1] === before.points[i]![1] ? [] : [i],
    )
    const all = moved.length === after.points.length
    if (all && before.dimensions?.length) throw Error(COINCIDENT_EDIT_REFUSED_MESSAGE)
    const others = sketches.map(o => (o.id === after.id ? after : o))
    validateSketchCurveConstraints(after, others)
    return solveCoincidentPolyline(after, others, all ? [] : moved, before)
  } catch {
    throw Error(COINCIDENT_EDIT_REFUSED_MESSAGE)
  }
}

export {
  COINCIDENT_EDIT_REFUSED_MESSAGE,
  CURVE_DRAG_REFUSED_MESSAGE,
  coincidentEditRefusedNotice,
  curveDragRefusedNotice,
  droppedCurveConstraintsNotice,
  sketchCurveConstraintName,
} from './sketchCurveConstraintNotices'
