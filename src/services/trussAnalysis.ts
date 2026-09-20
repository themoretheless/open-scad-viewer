import {callGeometryRust} from './geometry/kernel'

export type TrussVector = [number, number, number]
export interface TrussMember {
  nodes: [number, number]
  youngMpa: number
  areaMm2: number
}
export interface TrussModel {
  nodesMm: TrussVector[]
  members: TrussMember[]
  /** Explicit zero-displacement XYZ restraints, one mask per node. */
  restrained: [boolean, boolean, boolean][]
  forcesN: TrussVector[]
}
export interface TrussNodalWrench {
  /** Explicit unique node indices, never inferred from a bounding-box face. */
  nodes: number[]
  originMm: TrussVector
  forceN: TrussVector
  /** Moment about originMm in global XYZ, in N mm (not N m). */
  momentNmm: TrussVector
}
export type TrussWrenchModel = Omit<TrussModel, 'forcesN'> & {loads:TrussNodalWrench[]}
export type TrussInput = TrussModel | TrussWrenchModel
export interface TrussResponse {
  displacementsMm: TrussVector[]
  /** Signed global XYZ support reactions; unrestrained components are zero. */
  reactionsN: TrussVector[]
  /** Positive means tension; preserves input member order. */
  axialForcesN: number[]
  axialStressesMpa: number[]
  maxDeflectionMm: number
  maxRelativeResidual: number
  freeDofs: number
}

/** Linear axial bars only, not bending, buckling or certified strength.
 * Requires the geometry runtime; typed kernel failures propagate unchanged.
 */
export function solveTruss(model: TrussInput): TrussResponse {
  return callGeometryRust<TrussResponse>('loads' in model ? 'truss_solve_wrenches' : 'truss_solve', model)
}
