/** Public curve types and Rust/WASM adapters. All spline mathematics lives in crates/nurbs-kernel. */
import { callNurbsRust } from './nurbsRustKernel'
export { NurbsCurveError } from './nurbsRustKernel'

export interface NurbsCurve {
  degree: number
  knots: number[]
  controlPoints: number[][]
  weights: number[]
  /**
   * A periodic curve explicitly includes its p wrapped control points, weights,
   * and exterior knots. The last p controls equal the first p; knot intervals
   * repeat with the period knots[N]-knots[p]. A mere closed endpoint is not a
   * periodic representation. Bounded edits clamp one period and clear this flag.
   */
  periodic?: boolean
}

export type NurbsDerivativeStatus = 'available' | 'insufficient_continuity'
export type NurbsDerivativeSide = 'two_sided' | 'right' | 'left'

export interface NurbsBasisEvaluation {
  basis: number[]
  d1: number[]
  d2: number[]
  domain: [number, number]
  /** Null means that the parameter is inside a smooth polynomial span. */
  continuity: number | null
  derivative_status: NurbsDerivativeStatus
  derivative_side: NurbsDerivativeSide
}

export interface NurbsCurveEvaluation {
  point: number[]
  d1: number[] | null
  d2: number[] | null
  domain: [number, number]
  continuity: number | null
  derivative_status: NurbsDerivativeStatus
  derivative_side: NurbsDerivativeSide
}


export function validateNurbsCurve(curve: NurbsCurve): void { callNurbsRust('curve_validate', { curve }) }
export function nurbsBasisDerivatives(degree: number, knots: number[], controlCount: number, u: number, periodic = false): NurbsBasisEvaluation {
  return callNurbsRust('basis', { degree, knots, controlCount, u, periodic })
}
export function evaluateNurbsCurve(curve: NurbsCurve, u: number): NurbsCurveEvaluation { return callNurbsRust('curve_evaluate', { curve, u }) }
export function insertNurbsKnot(curve: NurbsCurve, u: number, count = 1): NurbsCurve { return callNurbsRust('curve_insert', { curve, u, count }) }
export function trimNurbsCurve(curve: NurbsCurve, a: number, b: number): NurbsCurve { return callNurbsRust('curve_trim', { curve, a, b }) }
export function splitNurbsCurve(curve: NurbsCurve, u: number): [NurbsCurve, NurbsCurve] { return callNurbsRust('curve_split', { curve, u }) }
export function reverseNurbsCurve(curve: NurbsCurve): NurbsCurve { return callNurbsRust('curve_reverse', { curve }) }
export function decomposeNurbsCurve(curve: NurbsCurve): { curve: NurbsCurve; domain: [number, number] }[] { return callNurbsRust('curve_decompose', { curve }) }
export function elevateNurbsCurve(curve: NurbsCurve, degree: number): NurbsCurve { return callNurbsRust('curve_elevate', { curve, degree }) }
export function nurbsCurveBounds(curve: NurbsCurve): { min: number[]; max: number[] } { return callNurbsRust('curve_bounds', { curve }) }
