import {callGeometryRust} from './geometry/kernel'

export type FrameVector = [number, number, number]
/** Rotational restraint mask: 3 translations, then 3 rotations. */
export type FrameRestraint = [boolean, boolean, boolean, boolean, boolean, boolean]
export interface FrameMember {
  nodes: [number, number]
  youngMpa: number
  poisson: number
  areaMm2: number
  /** Second moment about the local y axis (bending in the xz plane). */
  iyyMm4: number
  /** Second moment about the local z axis (bending in the xy plane). */
  izzMm4: number
  /** Torsion constant about the local x axis. */
  jMm4: number
  /** Omit for the exact Euler–Bernoulli member. */
  shearAreaYMm2?: number
  shearAreaZMm2?: number
  /** Preferred local z axis; default global +Z with automatic fallback. */
  localZHint?: FrameVector
  /** Rotational end releases: [torsion, about local y, about local z]. */
  releaseA?: [boolean, boolean, boolean]
  releaseB?: [boolean, boolean, boolean]
}
export type FrameMemberLoad =
  | {type: 'pointForce'; member: number; atMm: number; forceN: FrameVector; localAxes: boolean}
  | {type: 'pointMoment'; member: number; atMm: number; momentNmm: FrameVector; localAxes: boolean}
  | {type: 'uniform'; member: number; forceNPerMm: FrameVector; localAxes: boolean}
  | {type: 'trapezoidal'; member: number; fromNPerMm: FrameVector; toNPerMm: FrameVector; localAxes: boolean}
/** Nodal support beyond the rigid restraint mask. Springs are bidirectional;
 * lower/upper springs engage only under contact; lower/upper bounds are rigid
 * unilateral contacts solved by an active-set iteration. `dof`: 0-2
 * translations, 3-5 rotations. Unilateral supports make the response
 * nonlinear; envelopes then solve every combination directly.
 */
export type FrameSupport =
  | {type: 'spring'; node: number; dof: number; stiffness: number}
  | {type: 'lowerSpring'; node: number; dof: number; stiffness: number}
  | {type: 'upperSpring'; node: number; dof: number; stiffness: number}
  | {type: 'lowerBound'; node: number; dof: number}
  | {type: 'upperBound'; node: number; dof: number}
export interface FrameModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Optional springs and unilateral contacts (omit for rigid supports). */
  supports?: FrameSupport[]
  forcesN: FrameVector[]
  momentsNmm: FrameVector[]
  loads: FrameMemberLoad[]
}
export interface FrameStation {
  xMm: number
  axialN: number
  shearYN: number
  shearZN: number
  torsionNmm: number
  momentYNmm: number
  momentZNmm: number
}
export interface FrameResponse {
  displacementsMm: FrameVector[]
  rotationsRad: FrameVector[]
  reactionsN: FrameVector[]
  reactionMomentsNmm: FrameVector[]
  /** 21 stations per member, endpoints included, in member-local axes. */
  members: {stations: FrameStation[]}[]
  maxDeflectionMm: number
  maxRelativeResidual: number
  freeDofs: number
}

/** Linear Timoshenko frame analysis: small displacements, no buckling, P-Δ,
 * plasticity, or certified strength. Station resultants use the crate sign
 * convention (positive axial = tension; sagging is negative moment about the
 * local y axis for a member along +x with local z up). Requires the geometry
 * runtime; typed kernel failures propagate unchanged.
 */
export function solveFrame(model: FrameModel): FrameResponse {
  return callGeometryRust<FrameResponse>('frame_solve', model)
}

/** Nodal and member loads of one load case; the structure is shared. */
export interface FrameLoadCase {
  forcesN: FrameVector[]
  momentsNmm: FrameVector[]
  loads: FrameMemberLoad[]
}
/** A named row of factors, one per load case, positional. Zero admitted. */
export interface FrameCombination {
  name: string
  factors: number[]
}
/** Min/max of one scalar across combinations, with governing indices. */
export interface FrameMinMax {
  min: number
  max: number
  minCombination: number
  maxCombination: number
}
export interface FrameStationEnvelope {
  xMm: number
  axialN: FrameMinMax
  shearYN: FrameMinMax
  shearZN: FrameMinMax
  torsionNmm: FrameMinMax
  momentYNmm: FrameMinMax
  momentZNmm: FrameMinMax
}
export interface FrameEnvelopeModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Optional springs and unilateral contacts (omit for rigid supports). */
  supports?: FrameSupport[]
  /** 1-32 load cases against the shared structure. */
  cases: FrameLoadCase[]
  /** 1-64 combinations; factors are the caller's engineering decision. */
  combinations: FrameCombination[]
}
export interface FrameEnvelopeResponse {
  loadCases: number
  /** Combination names in input order. */
  combinations: string[]
  displacementsMm: FrameMinMax[][]
  rotationsRad: FrameMinMax[][]
  reactionsN: FrameMinMax[][]
  reactionMomentsNmm: FrameMinMax[][]
  /** 21 station envelopes per member, endpoints included. */
  members: {stations: FrameStationEnvelope[]}[]
  /** Envelope of each combination's maximum nodal displacement norm. */
  maxDeflectionMm: FrameMinMax
  maxRelativeResidual: number
  freeDofs: number
}

/** Solve load cases against one shared structure (the stiffness is factored
 * once) and envelope every response quantity across combinations. Same
 * analysis contract as solveFrame.
 */
export function solveFrameEnvelope(model: FrameEnvelopeModel): FrameEnvelopeResponse {
  return callGeometryRust<FrameEnvelopeResponse>('frame_envelope', model)
}

export interface FrameBucklingMode {
  /** Signed factor on the reference loads; negative means the structure
   * buckles under the reversed load. */
  loadFactor: number
  /** Mode shape, max |component| = 1; restrained DOFs are zero. */
  displacementsMm: FrameVector[]
  rotationsRad: FrameVector[]
  relativeResidual: number
}
export interface FrameBucklingResponse {
  /** Ascending |loadFactor|; the first entry is the critical mode. */
  modes: FrameBucklingMode[]
  /** Member axial forces of the reference state (tension positive) that the
   * geometric stiffness is built from. */
  axialForcesN: number[]
  freeDofs: number
}
export interface FrameBucklingModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Rigid or bidirectional springs only; unilateral contacts are refused
   * because they make the reference state nonlinear. */
  supports?: FrameSupport[]
  /** Reference load state; the geometric stiffness uses its axial forces. */
  reference: FrameLoadCase
  /** 1-8 modes. */
  modes: number
}

/** Linear (eigenvalue) buckling of a frame under a reference load set:
 * (K + λ·Kg)φ = 0 with Kg from the reference axial forces. Members with axial
 * member loads vary N along their length — split them so the constant-N
 * assumption holds per element. Not a certified stability calculation.
 * Requires the geometry runtime; typed kernel failures propagate unchanged.
 */
export function solveFrameBuckling(model: FrameBucklingModel): FrameBucklingResponse {
  return callGeometryRust<FrameBucklingResponse>('frame_buckling', model)
}

/** How member mass is distributed in modal analysis. */
export type MassModel = 'lumped' | 'consistent'
export interface FrameModalMode {
  frequencyHz: number
  /** Angular frequency ω = 2πf. */
  omegaRadS: number
  /** Mode shape, max |component| = 1; restrained DOFs are zero. */
  displacementsMm: FrameVector[]
  rotationsRad: FrameVector[]
  relativeResidual: number
}
export interface FrameModalResponse {
  /** Ascending frequency. */
  modes: FrameModalMode[]
  /** Total member mass in tonnes (1 t·mm/s² = 1 N with mm units). */
  totalMassT: number
  freeDofs: number
}
export interface FrameModalModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Rigid or bidirectional springs only; unilateral contacts are refused. */
  supports?: FrameSupport[]
  /** Per member, in t/mm³ (steel ≈ 7.85e-9); zero means a massless member. */
  densitiesTMm3: number[]
  massModel: MassModel
  /** 1-8 modes. */
  modes: number
}

/** Small-displacement modal analysis: Kφ = ω²Mφ. Massless free DOFs have no
 * finite frequency and do not appear among the modes. Not a certified dynamic
 * calculation. Requires the geometry runtime.
 */
export function solveFrameModal(model: FrameModalModel): FrameModalResponse {
  return callGeometryRust<FrameModalResponse>('frame_modal', model)
}

/** One plastic hinge in formation order. */
export interface PlasticHinge {
  member: number
  /** True when the hinge formed at the member's node A (x = 0). */
  atNodeA: boolean
  /** Cumulative load factor at which the hinge formed. */
  loadFactor: number
}
/** How a collapse analysis terminated. */
export type CollapseStatus = 'mechanism' | 'hingeLimit' | 'elasticUnlimited'
export interface FrameCollapseResponse {
  /** Hinges in formation order; simultaneous hinges share a load factor. */
  hinges: PlasticHinge[]
  status: CollapseStatus
  /** Collapse load factor (status `mechanism`), else null. */
  collapseLoadFactor: number | null
}
export interface FrameCollapseModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Rigid or bidirectional springs only; unilateral contacts are refused. */
  supports?: FrameSupport[]
  /** Reference load set; the analysis scales it by a single factor λ. */
  reference: FrameLoadCase
  /** Plastic moment per member (N·mm); null marks an elastic member. */
  plasticMomentsNMm: (number | null)[]
  /** Hinge insertion budget, 1-256. */
  maxHinges: number
}

/** Step-by-step plastic hinge analysis: each step solves elastically, finds
 * the member end that first reaches its plastic moment, and inserts a bending
 * hinge there (the moment at a hinge stays at Mp). Ends when the structure
 * becomes a mechanism (collapse), the hinge budget runs out, or no yieldable
 * section sees further moment. Hinges form at member ends only — refine the
 * mesh where a mid-span hinge is expected under distributed loads.
 * Elastic-perfectly-plastic small-displacement model; not a certified
 * ultimate-limit-state calculation. Requires the geometry runtime.
 */
export function solveFrameCollapse(model: FrameCollapseModel): FrameCollapseResponse {
  return callGeometryRust<FrameCollapseResponse>('frame_collapse', model)
}

/** Response quantity whose influence line is requested. */
export type InfluenceTarget =
  | {type:'displacement';node:number;/** 0-2 translation, 3-5 rotation. */dof:number}
  | {type:'reaction';node:number;/** Restrained DOF. */dof:number}
  | {type:'memberResultant';member:number;atMm:number;
     resultant:'axial'|'shearY'|'shearZ'|'torsion'|'momentY'|'momentZ'}
/** One load position: member index and distance from its node A. */
export interface InfluencePosition {member:number;atMm:number}
export interface FrameInfluenceModel {
  nodesMm: FrameVector[]
  members: FrameMember[]
  restrained: FrameRestraint[]
  /** Rigid or bidirectional springs only; unilateral contacts are refused. */
  supports?: FrameSupport[]
  /** The moving force vector; values are the target response under it. */
  forceN: FrameVector
  /** Load positions (1-1024), each on its member. */
  positions: InfluencePosition[]
  target: InfluenceTarget
}
export interface FrameInfluenceResponse {
  /** Target response per load position, in input order. */
  values: number[]
}

/** Influence line of one response quantity: the structure is solved exactly
 * for the moving force at every requested position — one independent solve
 * per position, no reciprocity shortcuts, so springs and end releases behave
 * identically to solveFrame. Member resultants are evaluated exactly at the
 * requested section, not interpolated from the diagram grid. Requires the
 * geometry runtime.
 */
export function solveFrameInfluence(model: FrameInfluenceModel): FrameInfluenceResponse {
  return callGeometryRust<FrameInfluenceResponse>('frame_influence', model)
}
