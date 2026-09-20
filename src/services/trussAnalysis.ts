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
export function solveTruss(model: TrussModel): TrussResponse {
  return callGeometryRust<TrussResponse>('truss_solve', model)
}
