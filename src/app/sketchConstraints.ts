import type { Point2 } from './directModeling'
import { solveRushGraphSketch, type SketchConstraint as SolverConstraint } from '../language/rushGraphSketch'

/** Persistent direction constraints of a polyline sketch; indices name sketch points. */
export type SketchConstraint =
  | { kind: 'horizontal' | 'vertical'; a: number; b: number }
  /** Two non-adjacent vertices of the same polyline share one position. */
  | { kind: 'coincident'; a: number; b: number }
  | { kind: 'parallel' | 'perpendicular' | 'equal_length'; a: number; b: number; c: number; d: number }

export const MAX_SKETCH_CONSTRAINTS = 32
export const SKETCH_CONSTRAINT_TOLERANCE_MM = 1e-6
const PAIR = new Set(['horizontal', 'vertical', 'coincident'])
const QUAD = new Set(['parallel', 'perpendicular', 'equal_length'])

interface ConstrainedSketch {
  points: Point2[]
  analytic?: unknown
  retainedProfile?: unknown
  editablePath?: unknown
  closed?: boolean
  constraints?: SketchConstraint[]
}

/** Structural check: polyline sketch with 3..16 points, known kinds, in-range distinct segment ends. */
export function validateSketchConstraints(sketch: ConstrainedSketch): void {
  const list = sketch.constraints
  if (list === undefined) return
  if (!Array.isArray(list) || list.length > MAX_SKETCH_CONSTRAINTS) throw Error('Invalid sketch constraints.')
  if (!list.length) return
  if (
    sketch.analytic ||
    sketch.retainedProfile ||
    sketch.editablePath ||
    sketch.points.length < 3 ||
    sketch.points.length > 16
  )
    throw Error('Sketch constraints require a polyline sketch with 3 to 16 points.')
  const n = sketch.points.length
  const index = (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) < n
  for (const c of list) {
    if (!c || typeof c !== 'object') throw Error('Invalid sketch constraints.')
    if (PAIR.has(c.kind)) {
      const keys = Object.keys(c).sort().join(',')
      if (keys !== 'a,b,kind' || !index(c.a) || !index(c.b) || c.a === c.b) throw Error('Invalid sketch constraints.')
      if (c.kind === 'coincident') {
        const gap = Math.abs(c.a - c.b)
        // Neighbouring vertices would collapse their segment.
        if (gap === 1 || (sketch.closed === true && gap === n - 1))
          throw Error('Sketch constraints conflict: coincident vertices bound one segment.')
        if (
          list.some(
            o =>
              o !== c &&
              o.kind === 'coincident' &&
              Math.min(o.a, o.b) === Math.min(c.a, c.b) &&
              Math.max(o.a, o.b) === Math.max(c.a, c.b),
          )
        )
          throw Error('Duplicate coincident constraint.')
      }
    } else if (QUAD.has(c.kind)) {
      const q = c as { a: number; b: number; c: number; d: number }
      const keys = Object.keys(c).sort().join(',')
      if (keys !== 'a,b,c,d,kind' || ![q.a, q.b, q.c, q.d].every(index) || q.a === q.b || q.c === q.d)
        throw Error('Invalid sketch constraints.')
      const same = (q.a === q.c && q.b === q.d) || (q.a === q.d && q.b === q.c)
      if (same) throw Error('A constraint needs two different segments.')
    } else throw Error('Invalid sketch constraints.')
  }
}

/** Converts persistent constraints to solver constraints over points named `p<i>`. */
export function solverConstraints(constraints: readonly SketchConstraint[]): SolverConstraint[] {
  return constraints.map((c, i) =>
    'c' in c
      ? { id: `constraint-${i}`, kind: c.kind, a: `p${c.a}`, b: `p${c.b}`, c: `p${c.c}`, d: `p${c.d}` }
      : { id: `constraint-${i}`, kind: c.kind, a: `p${c.a}`, b: `p${c.b}` },
  )
}

/**
 * Extra solver terms held together with persistent constraints. Auxiliary points are named `x<i>`
 * and dropped from the result; `check` rejects solutions on the wrong branch (for example a sign flip).
 */
export interface SolverExtras {
  constraints: readonly SolverConstraint[]
  points?: readonly Point2[]
  check?: (points: readonly Point2[]) => boolean
}

/**
 * Solves points so every persistent constraint and the given extra constraints hold.
 * Throws a conflict error when the set is inconsistent or the solver does not converge.
 */
export function solveConstrainedPoints(
  points: readonly Point2[],
  constraints: readonly SketchConstraint[],
  extra: readonly SolverConstraint[] | SolverExtras = [],
): Point2[] {
  const extras: SolverExtras = Array.isArray(extra) ? { constraints: extra } : (extra as SolverExtras)
  const solved = solveRushGraphSketch(
    [
      ...points.map((position, i) => ({ id: `p${i}`, position: [...position] as Point2 })),
      ...(extras.points ?? []).map((position, i) => ({ id: `x${i}`, position: [...position] as Point2 })),
    ],
    [...solverConstraints(constraints), ...extras.constraints],
    SKETCH_CONSTRAINT_TOLERANCE_MM,
  )
  if (
    solved.status === 'inconsistent' ||
    solved.status === 'not_converged' ||
    solved.degenerate_constraints.length ||
    solved.maximum_residual_mm > SKETCH_CONSTRAINT_TOLERANCE_MM
  )
    throw Error('Sketch constraints conflict or did not converge.')
  const result = solved.points.slice(0, points.length).map(p => p.position)
  if (extras.check && !extras.check(result)) throw Error('Sketch constraints conflict or did not converge.')
  // A segment collapsed to a point satisfies every direction equation; treat it as a conflict.
  const length = (a: number, b: number) => Math.hypot(result[b]![0] - result[a]![0], result[b]![1] - result[a]![1])
  const n = result.length
  const collapsed = constraints.some(c =>
    c.kind === 'coincident'
      ? [c.a, c.b].some(k => (k + 1 < n && length(k, k + 1) <= 1e-4) || (k > 0 && length(k - 1, k) <= 1e-4))
      : length(c.a, c.b) <= 1e-4 || ('c' in c && length(c.c, c.d) <= 1e-4),
  )
  if (collapsed) throw Error('Sketch constraints conflict: a constrained segment collapsed to a point.')
  return result
}

/** Residual of one constraint in millimetres (direction residuals are scaled by segment lengths). */
export function sketchConstraintResidual(points: readonly Point2[], c: SketchConstraint): number {
  const v = (a: number, b: number): Point2 => [points[b]![0] - points[a]![0], points[b]![1] - points[a]![1]]
  const u = v(c.a, c.b)
  if (c.kind === 'coincident') return Math.hypot(u[0], u[1])
  if (!('c' in c)) return Math.abs(c.kind === 'horizontal' ? u[1] : u[0])
  const w = v(c.c, c.d)
  const lu = Math.hypot(u[0], u[1]),
    lw = Math.hypot(w[0], w[1])
  if (c.kind === 'equal_length') return Math.abs(lu - lw)
  if (!(lu > 0 && lw > 0)) return Infinity
  const cross = (u[0] * w[1] - u[1] * w[0]) / lw,
    dot = (u[0] * w[0] + u[1] * w[1]) / lw
  return Math.abs(c.kind === 'parallel' ? cross : dot)
}

/** Adds a constraint and moves the points to satisfy it together with the existing ones. */
export function addSketchConstraint<T extends ConstrainedSketch>(
  sketch: T,
  constraint: SketchConstraint,
  extra: readonly SolverConstraint[] | SolverExtras = [],
): T {
  const constraints = [...(sketch.constraints ?? []), constraint]
  validateSketchConstraints({ ...sketch, constraints })
  return { ...sketch, constraints, points: solveConstrainedPoints(sketch.points, constraints, extra) }
}

const SAME_POINT_MM = 1e-9
const samePoint = (p: Point2, q: Point2) =>
  Math.abs(p[0] - q[0]) <= SAME_POINT_MM && Math.abs(p[1] - q[1]) <= SAME_POINT_MM

/**
 * Re-solves persistent constraints after an edit that kept the point count (vertex drag, extend,
 * offset). Points the edit moved are held fixed; when every point moved (offset) none is held and the
 * solver starts from the edited shape. Throws a conflict error instead of returning a violating sketch.
 */
export function resolveEditedSketchConstraints<T extends ConstrainedSketch>(
  before: ConstrainedSketch,
  after: T,
  extras: SolverExtras = { constraints: [] },
): T {
  const constraints = after.constraints
  if (!constraints?.length) return after
  if (after.points.length !== before.points.length)
    throw Error('Sketch constraints need an index remap when the point count changes.')
  validateSketchConstraints(after)
  const moved = after.points.flatMap((p, i) => (samePoint(p, before.points[i]!) ? [] : [i]))
  const fixed: SolverConstraint[] =
    moved.length === after.points.length
      ? []
      : moved.map(i => ({ id: `edit-${i}`, kind: 'fix', point: `p${i}`, at: [...after.points[i]!] as Point2 }))
  return {
    ...after,
    points: solveConstrainedPoints(after.points, constraints, {
      ...extras,
      constraints: [...extras.constraints, ...fixed],
    }),
  }
}

/** Result of carrying persistent constraints onto sketches whose points were split or renumbered. */
export interface RemappedSketchConstraints<T> {
  sketches: T[]
  /** Source constraints that no longer name surviving points; they are removed, never kept stale. */
  dropped: SketchConstraint[]
}

/**
 * Carries constraints from `before` onto `pieces` (for example the chains left by trim). A source point
 * survives in a piece when exactly one point of that piece has the same coordinates; a constraint is kept
 * on the first piece where all of its points survive and the piece can carry it, otherwise it is dropped.
 */
export function remapSketchConstraints<T extends ConstrainedSketch>(
  before: ConstrainedSketch,
  pieces: readonly T[],
): RemappedSketchConstraints<T> {
  const source = before.constraints ?? []
  const kept = new Set<number>()
  const sketches = pieces.map(piece => {
    const next = { ...piece }
    delete next.constraints
    if (!source.length) return next
    const map = before.points.map(p => {
      const hits = piece.points.flatMap((q, j) => (samePoint(p, q) ? [j] : []))
      return hits.length === 1 ? hits[0]! : -1
    })
    const list: SketchConstraint[] = []
    source.forEach((c, i) => {
      if (kept.has(i)) return
      const to = ('c' in c ? [c.a, c.b, c.c, c.d] : [c.a, c.b]).map(k => map[k] ?? -1)
      if (to.some(k => k < 0)) return
      // Constraints name segments: both ends must still bound one segment of the piece.
      const n = piece.points.length
      const segment = (i: number, j: number) =>
        Math.abs(i - j) === 1 || (piece.closed === true && Math.abs(i - j) === n - 1)
      if ((c.kind !== 'coincident' && !segment(to[0]!, to[1]!)) || (to.length === 4 && !segment(to[2]!, to[3]!))) return
      const candidate: SketchConstraint =
        'c' in c ? { kind: c.kind, a: to[0]!, b: to[1]!, c: to[2]!, d: to[3]! } : { kind: c.kind, a: to[0]!, b: to[1]! }
      try {
        validateSketchConstraints({ ...piece, constraints: [...list, candidate] })
      } catch {
        return
      }
      list.push(candidate)
      kept.add(i)
    })
    if (list.length) next.constraints = list
    return next
  })
  return { sketches, dropped: source.filter((_, i) => !kept.has(i)) }
}

const CONSTRAINT_NAMES: Record<SketchConstraint['kind'], [string, string]> = {
  horizontal: ['горизонталь', 'horizontal'],
  vertical: ['вертикаль', 'vertical'],
  parallel: ['параллельность', 'parallel'],
  perpendicular: ['перпендикулярность', 'perpendicular'],
  equal_length: ['равная длина', 'equal length'],
  coincident: ['совпадение вершин', 'coincident vertices'],
}

/** User-facing notice for constraints an edit removed (for example trim); empty when nothing was dropped. */
export function droppedConstraintsNotice(dropped: readonly SketchConstraint[], locale: string): string {
  if (!dropped.length) return ''
  const ru = locale === 'ru'
  const names = dropped.map(c => {
    const name = CONSTRAINT_NAMES[c.kind]?.[ru ? 0 : 1] ?? c.kind
    const seg = (a: number, b: number) => `${a + 1}–${b + 1}`
    return `${name} (${'c' in c ? `${seg(c.a, c.b)}, ${seg(c.c, c.d)}` : seg(c.a, c.b)})`
  })
  return ru
    ? `Обрезка сняла ограничения эскиза (${dropped.length}): ${names.join('; ')}.`
    : `Trim removed sketch constraints (${dropped.length}): ${names.join('; ')}.`
}
