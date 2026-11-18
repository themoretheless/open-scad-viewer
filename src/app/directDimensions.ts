import { requirePolylineSketch } from './retainedSketchProfile'
import { DirectInputError } from '../i18n/kernelErrors'
import { callGeometryRust } from '../kernel/geometry/kernel'
import type { DirectSketch, Point2 } from './directModeling'
import { sampleCurve } from './directSketchGeometry'
import type { NurbsCurve } from '../kernel/nurbsCurve'
import { solveRushGraphSketch, type SketchConstraint as SolverConstraint } from '../language/rushGraphSketch'
import {
  solveConstrainedPoints,
  solverConstraints,
  type SketchConstraint,
  type SolverExtras,
} from './sketchConstraints'
export type SketchDimension =
  | { kind: 'length'; a: number; b: number }
  | { kind: 'angle'; a: number; b: number; c: number }
  | { kind: 'horizontal' | 'vertical'; a: number; b: number }
  | { kind: 'radius' | 'diameter' }
export interface DimensionResult {
  points: Point2[]
  measurements: { value: number | null; label: Point2; lines: Point2[][] }[]
}
const measuredCache = new WeakMap<DirectSketch, DimensionResult>()
export function sketchDimensions(sketch: DirectSketch, edit?: { index: number; value: number }): DimensionResult {
  if (edit || sketch.dimensions?.length) requirePolylineSketch(sketch)
  if (!edit && (!sketch.dimensions || sketch.dimensions.length === 0)) {
    return { points: sketch.points, measurements: [] }
  }
  if (!edit && measuredCache.has(sketch)) {
    return measuredCache.get(sketch)!
  }
  if (sketch.analytic && (sketch.dimensions ?? []).every(d => d.kind === 'radius' || d.kind === 'diameter')) {
    const radius = edit
      ? edit.value > 0
        ? sketch.dimensions![edit.index]?.kind === 'diameter'
          ? edit.value / 2
          : edit.value
        : (() => {
            throw new DirectInputError('DIRECT_DIMENSION_POSITIVE', 'Radius must be positive.', 'dimension', [
              sketch.id,
            ])
          })()
      : sketch.analytic.radius
    const result = (sketch.dimensions ?? []).map(d => {
      const value = d.kind === 'diameter' ? radius * 2 : radius
      const offset = [radius * 1.2, 0] as Point2
      const label: Point2 = [sketch.analytic!.center[0] + offset[0], sketch.analytic!.center[1]]
      const lines: Point2[][] = [
        [
          [sketch.analytic!.center[0], sketch.analytic!.center[1]],
          [sketch.analytic!.center[0] + radius, sketch.analytic!.center[1]],
        ],
      ]
      return { value, label, lines }
    })
    const out = { points: sketch.points, measurements: result }
    if (!edit) measuredCache.set(sketch, out)
    return out
  }
  // Measurement never runs the solver or changes the geometry being measured.
  const measured = callGeometryRust('cad_dimensions', {
    points: sketch.points,
    dimensions: sketch.dimensions ?? [],
    ...(edit ? { edit } : {}),
  }) as DimensionResult
  const dimensions = sketch.dimensions ?? []
  if (
    edit &&
    dimensions.length > 1 &&
    dimensions.every(d => d.kind === 'length') &&
    sketch.points.length >= 3 &&
    sketch.points.length <= 16
  ) {
    const baseline =
      measuredCache.get(sketch) ??
      (callGeometryRust('cad_dimensions', { points: sketch.points, dimensions }) as DimensionResult)
    const constraints: SolverConstraint[] = dimensions.map((d, i) => ({
      id: `dimension-${i}`,
      kind: 'distance',
      a: `p${(d as { a: number }).a}`,
      b: `p${(d as { b: number }).b}`,
      value: i === edit.index ? edit.value : baseline.measurements[i]!.value!,
    }))
    const solved = solveRushGraphSketch(
      sketch.points.map((position, i) => ({ id: `p${i}`, position: [...position] as Point2 })),
      constraints,
    )
    if (solved.status === 'inconsistent' || solved.status === 'not_converged')
      throw Error('Sketch dimensions conflict or did not converge.')
    const points = solved.points.map(p => p.position)
    const result = callGeometryRust('cad_dimensions', {
      points,
      dimensions: sketch.dimensions ?? [],
    }) as DimensionResult
    return { points, measurements: result.measurements }
  }
  if (!edit) measuredCache.set(sketch, measured)
  return measured
}
export function editSketchDimension(sketch: DirectSketch, index: number, value: number): DirectSketch {
  if (sketch.analytic) {
    const dimension = sketch.dimensions?.[index]
    if (!dimension || (dimension.kind !== 'radius' && dimension.kind !== 'diameter'))
      throw new DirectInputError(
        'DIRECT_DIMENSION_ANALYTIC',
        'Analytic curves support radius and diameter dimensions only.',
        'dimension',
      )
    const radius = dimension.kind === 'diameter' ? value / 2 : value
    if (!(radius > 0 && Number.isFinite(radius)))
      throw new DirectInputError('DIRECT_DIMENSION_POSITIVE', 'Radius must be positive.', 'dimension', [sketch.id])
    const analytic = { ...sketch.analytic, radius }
    return { ...sketch, analytic, points: sampleCurve(analytic) }
  }
  if (sketch.constraints?.length) {
    const extras = dimensionSolverExtras(sketch, { index, value })
    // An angle edit starts from C rotated to the new angle so the solver lands on the requested branch.
    const start =
      sketch.dimensions?.[index]?.kind === 'angle'
        ? (
            callGeometryRust('cad_dimensions', {
              points: sketch.points,
              dimensions: sketch.dimensions,
              edit: { index, value },
            }) as DimensionResult
          ).points
        : sketch.points
    return { ...sketch, points: solveConstrainedPoints(start, sketch.constraints, extras) }
  }
  return { ...sketch, points: sketchDimensions(sketch, { index, value }).points }
}
/**
 * Length, horizontal and vertical dimensions held at their measured values (the edited one at its new value)
 * while persistent constraints are solved. A signed horizontal/vertical offset becomes an auxiliary corner point
 * `x<i>` level with `a` and plumb with `b` (or the reverse), at the offset's distance from `a`; the result check
 * rejects a solution whose offset flipped sign. A zero offset becomes a vertical/horizontal constraint.
 */
export function dimensionSolverExtras(sketch: DirectSketch, edit?: { index: number; value: number }): SolverExtras {
  const dimensions = sketch.dimensions ?? []
  if (
    dimensions.some(d => d.kind !== 'length' && d.kind !== 'horizontal' && d.kind !== 'vertical' && d.kind !== 'angle')
  )
    throw Error(
      'Sketch constraints can be combined only with length, horizontal, vertical and angle dimensions; the change conflicts with the other dimensions.',
    )
  if (edit) {
    const kind = dimensions[edit.index]?.kind
    if (!Number.isFinite(edit.value) || (kind === 'length' && !(edit.value > 0)))
      throw new DirectInputError('DIRECT_DIMENSION_POSITIVE', 'Length must be positive.', 'dimension', [sketch.id])
    if (kind === 'angle' && !(edit.value >= -180 && edit.value <= 180))
      throw Error('Signed angle must be between -180 and 180 degrees.')
  }
  const measured = dimensions.length ? sketchDimensions(sketch).measurements : []
  const constraints: SolverConstraint[] = [],
    points: Point2[] = [],
    targets: { kind: 'horizontal' | 'vertical'; a: number; b: number; value: number }[] = [],
    angles: { a: number; b: number; c: number; value: number }[] = []
  dimensions.forEach((d, i) => {
    const { a, b } = d as { a: number; b: number }
    const value = i === edit?.index ? edit.value : measured[i]!.value!
    const id = `dimension-${i}`
    if (d.kind === 'length') {
      constraints.push({ id, kind: 'distance', a: `p${a}`, b: `p${b}`, value })
      return
    }
    if (d.kind === 'angle') {
      if (value === null) throw Error('Angle requires two nonzero segments.')
      angles.push({ a, b, c: d.c, value })
      pushAngleTerms(sketch.points, { a, b, c: d.c, value }, id, constraints, points)
      return
    }
    const kind = d.kind as 'horizontal' | 'vertical'
    targets.push({ kind, a, b, value })
    if (Math.abs(value) <= 1e-9) {
      constraints.push({ id, kind: kind === 'horizontal' ? 'vertical' : 'horizontal', a: `p${a}`, b: `p${b}` })
      return
    }
    const pa = sketch.points[a]!,
      x = `x${points.length}`
    points.push(kind === 'horizontal' ? [pa[0] + value, pa[1]] : [pa[0], pa[1] + value])
    constraints.push(
      { id: `${id}-level`, kind: kind === 'horizontal' ? 'horizontal' : 'vertical', a: `p${a}`, b: x },
      { id: `${id}-plumb`, kind: kind === 'horizontal' ? 'vertical' : 'horizontal', a: x, b: `p${b}` },
      { id, kind: 'distance', a: `p${a}`, b: x, value: Math.abs(value) },
    )
  })
  const check = (result: readonly Point2[]) =>
    targets.every(t => {
      const axis = t.kind === 'horizontal' ? 0 : 1
      return Math.abs(result[t.b]![axis] - result[t.a]![axis] - t.value) <= 1e-5
    }) && angles.every(t => angleError(result, t) <= 1e-4)
  return { constraints, points, check }
}
/** Signed angle a-b-c in degrees, matching `cad_dimensions` (counter-clockwise from b→a to b→c). */
export function signedSketchAngle(points: readonly Point2[], a: number, b: number, c: number): number | null {
  const u = [points[a]![0] - points[b]![0], points[a]![1] - points[b]![1]],
    w = [points[c]![0] - points[b]![0], points[c]![1] - points[b]![1]]
  if (!(Math.hypot(u[0]!, u[1]!) > 1e-9 && Math.hypot(w[0]!, w[1]!) > 1e-9)) return null
  return (Math.atan2(u[0]! * w[1]! - u[1]! * w[0]!, u[0]! * w[0]! + u[1]! * w[1]!) * 180) / Math.PI
}
function angleError(points: readonly Point2[], t: { a: number; b: number; c: number; value: number }): number {
  const angle = signedSketchAngle(points, t.a, t.b, t.c)
  if (angle === null) return Infinity
  const diff = Math.abs(angle - t.value) % 360
  return Math.min(diff, 360 - diff)
}
/**
 * The solver has no angle term, so an angle a-b-c becomes two auxiliary points at radius r from b, one parallel to
 * b→a and one parallel to b→c, with the chord between them 2r·sin(|θ|/2). Parallel terms also accept the opposite
 * direction and the chord fixes only |θ|, so the result check rejects the wrong branch. 0° and ±180° become a
 * parallel constraint between the two segments.
 */
function pushAngleTerms(
  source: readonly Point2[],
  t: { a: number; b: number; c: number; value: number },
  id: string,
  constraints: SolverConstraint[],
  points: Point2[],
): void {
  const { a, b, c, value } = t
  if (Math.abs(value) <= 1e-9 || Math.abs(Math.abs(value) - 180) <= 1e-9) {
    constraints.push({ id, kind: 'parallel', a: `p${b}`, b: `p${a}`, c: `p${b}`, d: `p${c}` })
    return
  }
  const pb = source[b]!,
    u = [source[a]![0] - pb[0], source[a]![1] - pb[1]],
    w = [source[c]![0] - pb[0], source[c]![1] - pb[1]]
  const lu = Math.hypot(u[0]!, u[1]!),
    lw = Math.hypot(w[0]!, w[1]!)
  if (!(lu > 1e-9 && lw > 1e-9)) throw Error('Angle requires two nonzero segments.')
  const r = Math.min(lu, lw),
    start = Math.atan2(u[1]!, u[0]!),
    target = start + (value * Math.PI) / 180
  const xu = `x${points.length}`,
    xw = `x${points.length + 1}`
  points.push(
    [pb[0] + r * Math.cos(start), pb[1] + r * Math.sin(start)],
    [pb[0] + r * Math.cos(target), pb[1] + r * Math.sin(target)],
  )
  constraints.push(
    { id: `${id}-ray-a`, kind: 'parallel', a: `p${b}`, b: `p${a}`, c: `p${b}`, d: xu },
    { id: `${id}-ray-c`, kind: 'parallel', a: `p${b}`, b: `p${c}`, c: `p${b}`, d: xw },
    { id: `${id}-radius-a`, kind: 'distance', a: `p${b}`, b: xu, value: r },
    { id: `${id}-radius-c`, kind: 'distance', a: `p${b}`, b: xw, value: r },
    { id, kind: 'distance', a: xu, b: xw, value: 2 * r * Math.sin((Math.abs(value) * Math.PI) / 360) },
  )
}
export function bridgeNurbsCurves(
  a: NurbsCurve,
  b: NurbsCurve,
  endA: 'start' | 'end' = 'end',
  endB: 'start' | 'end' = 'start',
  tension = 1,
): NurbsCurve {
  return callGeometryRust('cad_bridge_curve', { a, b, endA, endB, tension })
}

const dimensionStatusCache = new WeakMap<DirectSketch, ReturnType<typeof solveRushGraphSketch> | null>()
/** State of the length constraints preserved together by dimension editing. */
export function sketchDimensionStatus(sketch: DirectSketch) {
  if (dimensionStatusCache.has(sketch)) return dimensionStatusCache.get(sketch)!
  const dimensions = sketch.dimensions ?? []
  const constraints: readonly SketchConstraint[] = sketch.constraints ?? []
  if (
    sketch.analytic ||
    sketch.points.length < 3 ||
    sketch.points.length > 16 ||
    !(dimensions.length || constraints.length) ||
    !dimensions.every(
      d =>
        d.kind === 'length' ||
        (constraints.length > 0 && (d.kind === 'horizontal' || d.kind === 'vertical' || d.kind === 'angle')),
    )
  ) {
    dimensionStatusCache.set(sketch, null)
    return null
  }
  const extras = dimensionSolverExtras(sketch)
  const result = solveRushGraphSketch(
    [
      ...sketch.points.map((position, i) => ({ id: `p${i}`, position: [...position] as Point2 })),
      ...(extras.points ?? []).map((position, i) => ({ id: `x${i}`, position: [...position] as Point2 })),
    ],
    [...extras.constraints, ...solverConstraints(constraints)],
  )
  dimensionStatusCache.set(sketch, result)
  return result
}
