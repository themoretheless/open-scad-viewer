import type { Point2 } from './directModeling'
import { solveRushGraphSketch, type SketchConstraint as SolverConstraint } from '../language/rushGraphSketch'

/** Persistent direction constraints of a polyline sketch; indices name sketch points. */
export type SketchConstraint =
  | { kind: 'horizontal' | 'vertical'; a: number; b: number }
  | { kind: 'parallel' | 'perpendicular' | 'equal_length'; a: number; b: number; c: number; d: number }

export const MAX_SKETCH_CONSTRAINTS = 32
export const SKETCH_CONSTRAINT_TOLERANCE_MM = 1e-6
const PAIR = new Set(['horizontal', 'vertical'])
const QUAD = new Set(['parallel', 'perpendicular', 'equal_length'])

interface ConstrainedSketch { points: Point2[]; analytic?: unknown; retainedProfile?: unknown; editablePath?: unknown; constraints?: SketchConstraint[] }

/** Structural check: polyline sketch with 3..16 points, known kinds, in-range distinct segment ends. */
export function validateSketchConstraints(sketch: ConstrainedSketch): void {
  const list = sketch.constraints
  if (list === undefined) return
  if (!Array.isArray(list) || list.length > MAX_SKETCH_CONSTRAINTS) throw Error('Invalid sketch constraints.')
  if (!list.length) return
  if (sketch.analytic || sketch.retainedProfile || sketch.editablePath || sketch.points.length < 3 || sketch.points.length > 16) throw Error('Sketch constraints require a polyline sketch with 3 to 16 points.')
  const n = sketch.points.length
  const index = (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) < n
  for (const c of list) {
    if (!c || typeof c !== 'object') throw Error('Invalid sketch constraints.')
    if (PAIR.has(c.kind)) {
      const keys = Object.keys(c).sort().join(',')
      if (keys !== 'a,b,kind' || !index(c.a) || !index(c.b) || c.a === c.b) throw Error('Invalid sketch constraints.')
    } else if (QUAD.has(c.kind)) {
      const q = c as { a: number; b: number; c: number; d: number }
      const keys = Object.keys(c).sort().join(',')
      if (keys !== 'a,b,c,d,kind' || ![q.a, q.b, q.c, q.d].every(index) || q.a === q.b || q.c === q.d) throw Error('Invalid sketch constraints.')
      const same = (q.a === q.c && q.b === q.d) || (q.a === q.d && q.b === q.c)
      if (same) throw Error('A constraint needs two different segments.')
    } else throw Error('Invalid sketch constraints.')
  }
}

/** Converts persistent constraints to solver constraints over points named `p<i>`. */
export function solverConstraints(constraints: readonly SketchConstraint[]): SolverConstraint[] {
  return constraints.map((c, i) => 'c' in c
    ? { id: `constraint-${i}`, kind: c.kind, a: `p${c.a}`, b: `p${c.b}`, c: `p${c.c}`, d: `p${c.d}` }
    : { id: `constraint-${i}`, kind: c.kind, a: `p${c.a}`, b: `p${c.b}` })
}

/**
 * Solves points so every persistent constraint and the given extra constraints hold.
 * Throws a conflict error when the set is inconsistent or the solver does not converge.
 */
export function solveConstrainedPoints(points: readonly Point2[], constraints: readonly SketchConstraint[], extra: readonly SolverConstraint[] = []): Point2[] {
  const solved = solveRushGraphSketch(points.map((position, i) => ({ id: `p${i}`, position: [...position] as Point2 })), [...solverConstraints(constraints), ...extra], SKETCH_CONSTRAINT_TOLERANCE_MM)
  if (solved.status === 'inconsistent' || solved.status === 'not_converged' || solved.degenerate_constraints.length || solved.maximum_residual_mm > SKETCH_CONSTRAINT_TOLERANCE_MM) throw Error('Sketch constraints conflict or did not converge.')
  const result = solved.points.map(p => p.position)
  // A segment collapsed to a point satisfies every direction equation; treat it as a conflict.
  const length = (a: number, b: number) => Math.hypot(result[b]![0] - result[a]![0], result[b]![1] - result[a]![1])
  if (constraints.some(c => length(c.a, c.b) <= 1e-4 || ('c' in c && length(c.c, c.d) <= 1e-4))) throw Error('Sketch constraints conflict: a constrained segment collapsed to a point.')
  return result
}

/** Residual of one constraint in millimetres (direction residuals are scaled by segment lengths). */
export function sketchConstraintResidual(points: readonly Point2[], c: SketchConstraint): number {
  const v = (a: number, b: number): Point2 => [points[b]![0] - points[a]![0], points[b]![1] - points[a]![1]]
  const u = v(c.a, c.b)
  if (!('c' in c)) return Math.abs(c.kind === 'horizontal' ? u[1] : u[0])
  const w = v(c.c, c.d)
  const lu = Math.hypot(u[0], u[1]), lw = Math.hypot(w[0], w[1])
  if (c.kind === 'equal_length') return Math.abs(lu - lw)
  if (!(lu > 0 && lw > 0)) return Infinity
  const cross = (u[0] * w[1] - u[1] * w[0]) / lw, dot = (u[0] * w[0] + u[1] * w[1]) / lw
  return Math.abs(c.kind === 'parallel' ? cross : dot)
}

/** Adds a constraint and moves the points to satisfy it together with the existing ones. */
export function addSketchConstraint<T extends ConstrainedSketch>(sketch: T, constraint: SketchConstraint, extra: readonly SolverConstraint[] = []): T {
  const constraints = [...(sketch.constraints ?? []), constraint]
  validateSketchConstraints({ ...sketch, constraints })
  return { ...sketch, constraints, points: solveConstrainedPoints(sketch.points, constraints, extra) }
}
