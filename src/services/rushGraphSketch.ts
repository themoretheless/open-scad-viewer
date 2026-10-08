import {callGeometryRust} from './geometry/kernel'
import type {SketchSolution, SketchConstraint as NativeConstraint} from './geometry/sketch'
export type SketchPoint = { id: string; position: [number, number] }
export type SketchConstraint =
  | { id: string; kind: 'fix'; point: string; at: [number, number] }
  | { id: string; kind: 'horizontal' | 'vertical' | 'coincident'; a: string; b: string }
  | { id: string; kind: 'distance'; a: string; b: string; value: number }
  | { id: string; kind: 'parallel' | 'perpendicular' | 'equal_length'; a: string; b: string; c: string; d: string }
/** Bounded native solve; named point and constraint identities belong to the host. */
export function solveRushGraphSketch(points: SketchPoint[], constraints: SketchConstraint[], tolerance = 1e-6) {
  if (points.length < 3 || points.length > 16 || constraints.length > 48 || !(tolerance >= 1e-8 && tolerance <= 0.1)) throw new Error('Sketch limits exceeded.')
  const index = new Map(points.map((p, i) => [p.id, i]))
  if (index.size !== points.length || new Set(constraints.map(c => c.id)).size !== constraints.length) throw new Error('Sketch point and constraint IDs must be unique.')
  for (const constraint of constraints) {
    const refs = constraint.kind === 'fix' ? [constraint.point] : 'c' in constraint ? [constraint.a, constraint.b, constraint.c, constraint.d] : [constraint.a, constraint.b]
    for (const ref of refs) if (!index.has(ref)) throw new Error(`Unknown sketch point ${ref}.`)
    if (constraint.kind === 'fix' && constraint.at.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) throw new Error('Invalid fixed target.')
    if (constraint.kind === 'distance' && (!(constraint.value > 0) || !Number.isFinite(constraint.value))) throw new Error('Distance must be positive; use coincident for zero distance.')
  }
  const x = points.flatMap(p => p.position)
  if (x.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) throw new Error('Invalid sketch coordinates.')
  const native: NativeConstraint[] = constraints.map(c => {
    if (c.kind === 'fix') return {kind: c.kind, point: index.get(c.point)!, at: c.at}
    if ('c' in c) return {kind: c.kind, a: index.get(c.a)!, b: index.get(c.b)!, c: index.get(c.c)!, d: index.get(c.d)!}
    if (c.kind === 'distance') return {kind: c.kind, a: index.get(c.a)!, b: index.get(c.b)!, value: c.value}
    return {kind: c.kind, a: index.get(c.a)!, b: index.get(c.b)!}
  })
  const {solution, diagnostics} = callGeometryRust<{solution: SketchSolution; diagnostics: {redundantEquations: number; degenerateConstraints: number[]; constraintResiduals: number[]; inconsistent: boolean}}>('sketch_solve_diagnostics', {sketch: {points: points.map(p => p.position), constraints: native}, tolerance})
  const degenerate = new Set(diagnostics.degenerateConstraints)
  const status = diagnostics.inconsistent ? 'inconsistent' : solution.status === 'degenerate' ? 'not_converged' : solution.status
  return {
    status, iterations: solution.iterations, tolerance_mm: tolerance,
    degrees_of_freedom: solution.degreesOfFreedom,
    redundant_equations: diagnostics.redundantEquations,
    maximum_residual_mm: solution.maxResidual,
    degenerate_constraints: diagnostics.degenerateConstraints.map(i => constraints[i]!.id),
    points: points.map((p, i) => ({id: p.id, position: solution.sketch.points[i]! as [number, number]})),
    constraints: constraints.map((c, i) => ({id: c.id, residual_mm: diagnostics.constraintResiduals[i]!, satisfied: diagnostics.constraintResiduals[i]! <= tolerance && !degenerate.has(i)})),
  }
}
