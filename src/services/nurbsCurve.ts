/** Public curve types and Rust/WASM adapters. All spline mathematics lives in crates/nurbs-core. */
import { callNurbsRust } from './geometry/nurbs'
export { NurbsCurveError } from './geometry/nurbs'

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
export interface NurbsPointTrimResult {
  curve:NurbsCurve
  parameter:number
  parameterInterval:[number,number]
  point:number[]
  distanceUpperMm:number
  keptDomain:[number,number]
  projection:import('./nurbsFoundation').NurbsProjectionCertificate
}
export type NurbsScreenProjection = [[number,number,number,number],[number,number,number,number]]
/** Projection-certificate distances use CSS pixels; the returned point and curve stay in model coordinates. */
export function trimNurbsCurveAtScreenPoint(curve:NurbsCurve,point:[number,number],matrix:NurbsScreenProjection,keep:'start'|'end',radius:number):Omit<NurbsPointTrimResult,'distanceUpperMm'>&{distanceUpperPx:number;projectionSpace:'css-pixels'} {
  return callNurbsRust('curve_trim_screen_point',{curve,point,matrix,keep,radius})
}
export function trimNurbsCurveAtPoint(curve:NurbsCurve,point:number[],keep:'start'|'end',maxDistance:number):NurbsPointTrimResult {
  return callNurbsRust('curve_trim_point',{curve,point,keep,maxDistance})
}
export function trimNurbsCurve(curve: NurbsCurve, a: number, b: number): NurbsCurve { return callNurbsRust('curve_trim', { curve, a, b }) }
export function splitNurbsCurve(curve: NurbsCurve, u: number): [NurbsCurve, NurbsCurve] { return callNurbsRust('curve_split', { curve, u }) }
export function reverseNurbsCurve(curve: NurbsCurve): NurbsCurve { return callNurbsRust('curve_reverse', { curve }) }
export function decomposeNurbsCurve(curve: NurbsCurve): { curve: NurbsCurve; domain: [number, number] }[] { return callNurbsRust('curve_decompose', { curve }) }
export function elevateNurbsCurve(curve: NurbsCurve, degree: number): NurbsCurve { return callNurbsRust('curve_elevate', { curve, degree }) }
export function nurbsCurveBounds(curve: NurbsCurve): { min: number[]; max: number[] } { return callNurbsRust('curve_bounds', { curve }) }

/** Global distance interval on the original curves; convergence does not imply a unique pair. */
export interface NurbsCurveDistance {
  method:'interval-de-boor-pair-subdivision'
  distanceIntervalMm:[number,number]
  parameters:[number,number]
  points:[number[],number[]]
  pointEnclosures:[Array<[number,number]>,Array<[number,number]>]
  converged:boolean
  reason:'tolerance'|'work-limit'|'precision-limit'
  cells:number
  maxCells:number
  toleranceMm:number
}
export function measureNurbsCurveDistance(a:NurbsCurve,b:NurbsCurve,toleranceMm=0.001,maxCells=10000):NurbsCurveDistance {
  return callNurbsRust('curve_distance',{a,b,toleranceMm,maxCells})
}
