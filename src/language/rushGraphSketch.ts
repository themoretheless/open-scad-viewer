import {callGeometryRust} from '../kernel/geometry/kernel'
import type {SketchSolution, SketchConstraint as NativeConstraint} from '../kernel/geometry/sketch'
export type SketchPoint = { id: string; position: [number, number] }
export type SketchConstraint =
  | { id: string; kind: 'fix'; point: string; at: [number, number] }
  | { id: string; kind: 'horizontal' | 'vertical' | 'coincident'; a: string; b: string }
  | { id: string; kind: 'distance'; a: string; b: string; value: number }
  | { id: string; kind: 'parallel' | 'perpendicular' | 'equal_length'; a: string; b: string; c: string; d: string }
  | { id: string; kind: 'radius'; circle: string; value: number }
  | { id: string; kind: 'concentric' | 'equal_radius'; a: string; b: string }
  | { id: string; kind: 'point_on_circle'; point: string; circle: string }
  | { id: string; kind: 'tangent_line_circle'; a: string; b: string; circle: string }
  | { id: string; kind: 'tangent_circles'; a: string; b: string; internal?: boolean }
export type SketchCircle = { id: string; center: string; radius: number }
const CIRCLE_PAIR = new Set(['concentric', 'equal_radius', 'tangent_circles'])
/** Bounded native solve; named point and constraint identities belong to the host. */
export function solveRushGraphSketch(points: SketchPoint[], constraints: SketchConstraint[], tolerance = 1e-6, circles: SketchCircle[] = []) {
  if (points.length < 3 || points.length > 16 || circles.length > 8 || constraints.length > 48 || !(tolerance >= 1e-8 && tolerance <= 0.1)) throw new Error('Sketch limits exceeded.')
  const index = new Map(points.map((p, i) => [p.id, i]))
  const circleIndex = new Map(circles.map((c, i) => [c.id, i]))
  if (index.size !== points.length || circleIndex.size !== circles.length || circles.some(c => index.has(c.id)) || new Set(constraints.map(c => c.id)).size !== constraints.length) throw new Error('Sketch point and constraint IDs must be unique.')
  for (const circle of circles) {
    if (!index.has(circle.center)) throw new Error('Sketch circle center must name a sketch point.')
    if (!(circle.radius > 0) || !(circle.radius <= 1e6)) throw new Error('Sketch circle radius must be positive.')
  }
  const p = (ref: string) => { const i = index.get(ref); if (i === undefined) throw new Error(`Unknown sketch point ${ref}.`); return i }
  const k = (ref: string) => { const i = circleIndex.get(ref); if (i === undefined) throw new Error(`Unknown sketch circle ${ref}.`); return i }
  for (const constraint of constraints) {
    if ('circle' in constraint) { k(constraint.circle); if ('point' in constraint) p(constraint.point); else if ('a' in constraint) { p(constraint.a); p(constraint.b) } }
    else if (CIRCLE_PAIR.has(constraint.kind)) { const c = constraint as { a: string; b: string }; k(c.a); k(c.b) }
    else for (const ref of constraint.kind === 'fix' ? [constraint.point] : 'c' in constraint ? [constraint.a, constraint.b, constraint.c, constraint.d] : [(constraint as { a: string }).a, (constraint as { b: string }).b]) p(ref)
    if (constraint.kind === 'radius' && (!(constraint.value > 0) || !Number.isFinite(constraint.value))) throw new Error('Radius must be positive.')
    if (constraint.kind === 'fix' && constraint.at.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) throw new Error('Invalid fixed target.')
    if (constraint.kind === 'distance' && (!(constraint.value > 0) || !Number.isFinite(constraint.value))) throw new Error('Distance must be positive; use coincident for zero distance.')
  }
  const x = points.flatMap(p => p.position)
  if (x.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) throw new Error('Invalid sketch coordinates.')
  const native: NativeConstraint[] = constraints.map(c => {
    if (c.kind === 'radius') return {kind: c.kind, circle: k(c.circle), value: c.value}
    if (c.kind === 'point_on_circle') return {kind: c.kind, point: p(c.point), circle: k(c.circle)}
    if (c.kind === 'tangent_line_circle') return {kind: c.kind, a: p(c.a), b: p(c.b), circle: k(c.circle)}
    if (c.kind === 'tangent_circles') return {kind: c.kind, a: k(c.a), b: k(c.b), internal: c.internal ?? false}
    if (c.kind === 'concentric' || c.kind === 'equal_radius') return {kind: c.kind, a: k(c.a), b: k(c.b)}
    if (c.kind === 'fix') return {kind: c.kind, point: index.get(c.point)!, at: c.at}
    if ('c' in c) return {kind: c.kind, a: index.get(c.a)!, b: index.get(c.b)!, c: index.get(c.c)!, d: index.get(c.d)!}
    if (c.kind === 'distance') return {kind: c.kind, a: index.get(c.a)!, b: index.get(c.b)!, value: c.value}
    return {kind: c.kind, a: index.get(c.a)!, b: index.get(c.b)!}
  })
  const {solution, diagnostics} = callGeometryRust<{solution: SketchSolution; diagnostics: {redundantEquations: number; degenerateConstraints: number[]; constraintResiduals: number[]; inconsistent: boolean}}>('sketch_solve_diagnostics', {sketch: {points: points.map(p => p.position), circles: circles.map(c => ({center: index.get(c.center)!, radius: c.radius})), constraints: native}, tolerance})
  const degenerate = new Set(diagnostics.degenerateConstraints)
  const status = diagnostics.inconsistent ? 'inconsistent' : solution.status === 'degenerate' ? 'not_converged' : solution.status
  return {
    status, iterations: solution.iterations, tolerance_mm: tolerance,
    degrees_of_freedom: solution.degreesOfFreedom,
    redundant_equations: diagnostics.redundantEquations,
    maximum_residual_mm: solution.maxResidual,
    degenerate_constraints: diagnostics.degenerateConstraints.map(i => constraints[i]!.id),
    points: points.map((p, i) => ({id: p.id, position: solution.sketch.points[i]! as [number, number]})),
    ...(circles.length ? {circles: circles.map((c, i) => ({id: c.id, center: solution.sketch.points[index.get(c.center)!]! as [number, number], radius: solution.sketch.circles![i]!.radius}))} : {}),
    constraints: constraints.map((c, i) => ({id: c.id, residual_mm: diagnostics.constraintResiduals[i]!, satisfied: diagnostics.constraintResiduals[i]! <= tolerance && !degenerate.has(i)})),
  }
}
