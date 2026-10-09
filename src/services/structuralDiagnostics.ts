import {callGeometryRust} from './geometry/kernel'
import type {FrameMember, FrameRestraint, FrameSupport, FrameVector} from './frameAnalysis'
import type {TrussMember, TrussVector} from './trussAnalysis'

/** Why one degree of freedom is implicated in a singular stiffness matrix. */
export type DofIssue = 'unrestrained' | 'mechanism'
export interface DofDiagnosis {
  node: number
  /** 0-2 translations; frames add 3-5 rotations. */
  dof: number
  dofName: string
  /** 'unrestrained': no stiffness at all (isolated node, released direction).
   * 'mechanism': connected, but the factorization breaks down (hinge chain,
   * collinear bars, rigid-body mode). */
  issue: DofIssue
}
/** Structured answer to "why is this structure singular". */
export interface SingularityDiagnosis {
  stable: boolean
  /** Smallest normalized pivot over the free DOFs; a small value marks a
   * near-mechanism even in a stable structure. Null when a zero diagonal
   * stopped assembly before factorization. */
  minNormalizedPivot: number | null
  issues: DofDiagnosis[]
}

export interface TrussDiagnoseModel {
  nodesMm: TrussVector[]
  members: TrussMember[]
  restrained: [boolean, boolean, boolean][]
}

/** Explain why a truss would be refused as singular, or confirm it is stable.
 * Load-independent: the restraint mask and member layout decide. Requires the
 * geometry runtime; typed kernel failures propagate unchanged.
 */
export function diagnoseTruss(model: TrussDiagnoseModel): SingularityDiagnosis {
  return callGeometryRust<SingularityDiagnosis>('truss_diagnose', model)
}

export interface FrameDiagnoseModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Springs and unilateral contacts are all treated as engaged, matching the
   * first active-set iteration of solveFrame. */
  supports?: FrameSupport[]
}

/** Explain why a frame would be refused as singular, or confirm it is stable.
 * Load-independent: restraints, supports, and end releases decide. Requires
 * the geometry runtime; typed kernel failures propagate unchanged.
 */
export function diagnoseFrame(model: FrameDiagnoseModel): SingularityDiagnosis {
  return callGeometryRust<SingularityDiagnosis>('frame_diagnose', model)
}
