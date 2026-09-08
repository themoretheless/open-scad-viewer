/** Surface types and Rust/WASM adapters. */
import { type NurbsCurve } from './nurbsCurve'
import { callNurbsRust, createRustSurfaceEvaluator, decodeNurbsResult } from './nurbsRustKernel'

export type NurbsSurface = {
  degreeU: number
  degreeV: number
  knotsU: number[]
  knotsV: number[]
  /** Outer index is U, inner index is V; each point is [x, y, z]. */
  controlPoints: number[][][]
  weights: number[][]
  periodicU?: boolean
  periodicV?: boolean
}
export type NurbsSurfaceAxis = 'u' | 'v'
type Point3 = [number, number, number]

export type NurbsSurfaceEvaluation = {
  point: Point3
  du: Point3 | null
  dv: Point3 | null
  duu: Point3 | null
  duv: Point3 | null
  dvv: Point3 | null
  normal: Point3 | null
  gaussianCurvature: number | null
  /** Signed with respect to the Su cross Sv normal. */
  meanCurvature: number | null
  derivative_status: 'available' | 'insufficient_continuity' | 'singular'
  derivative_side_u: 'two_sided' | 'left' | 'right'
  derivative_side_v: 'two_sided' | 'left' | 'right'
  domainU: [number, number]
  domainV: [number, number]
}


export function validateNurbsSurface(surface: NurbsSurface): void { callNurbsRust('surface_validate', { surface }) }
export function evaluateNurbsSurface(surface: NurbsSurface, u: number, v: number): NurbsSurfaceEvaluation { return callNurbsRust('surface_evaluate', { surface, u, v }) }
/** Immutable validated Rust snapshot. Release explicitly after bounded sampling. */
export function createNurbsSurfaceEvaluator(surface: NurbsSurface): ((u: number, v: number) => NurbsSurfaceEvaluation) & { dispose(): void } {
  const evaluator = createRustSurfaceEvaluator(surface)
  let disposed = false
  const sample = (u: number, v: number): NurbsSurfaceEvaluation => {
    if (disposed) throw new Error('NURBS surface evaluator is disposed.')
    return decodeNurbsResult(evaluator.evaluate(u, v))
  }
  sample.dispose = () => { if (!disposed) { disposed = true; evaluator.free() } }
  return sample
}
export function insertNurbsSurfaceKnot(surface: NurbsSurface, axis: NurbsSurfaceAxis, value: number, count = 1): NurbsSurface { return callNurbsRust('surface_insert', { surface, axis, u: value, count }) }
export function elevateNurbsSurface(surface: NurbsSurface, axis: NurbsSurfaceAxis, targetDegree: number): NurbsSurface { return callNurbsRust('surface_elevate', { surface, axis, degree: targetDegree }) }
export function reverseNurbsSurface(surface: NurbsSurface, axis: NurbsSurfaceAxis): NurbsSurface { return callNurbsRust('surface_reverse', { surface, axis }) }
export function trimNurbsSurface(surface: NurbsSurface, bounds: [number, number, number, number]): NurbsSurface
export function trimNurbsSurface(surface: NurbsSurface, uMin: number, uMax: number, vMin: number, vMax: number): NurbsSurface
export function trimNurbsSurface(surface: NurbsSurface, first: number | [number, number, number, number], uMax?: number, vMin?: number, vMax?: number): NurbsSurface {
  return callNurbsRust('surface_trim', { surface, bounds: Array.isArray(first) ? first : [first, uMax, vMin, vMax] })
}
export function isoNurbsCurve(surface: NurbsSurface, direction: NurbsSurfaceAxis, parameter: number): NurbsCurve { return callNurbsRust('surface_iso', { surface, axis: direction, u: parameter }) }
export function nurbsSurfaceBounds(surface: NurbsSurface): { min: Point3; max: Point3 } { return callNurbsRust('surface_bounds', { surface }) }
