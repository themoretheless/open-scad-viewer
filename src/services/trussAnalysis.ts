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

export interface TrussBucklingMode {
  /** Signed factor on the reference loads; negative means the truss buckles
   * under the reversed load. */
  loadFactor: number
  /** Mode shape, max |component| = 1; restrained DOFs are zero. */
  displacementsMm: TrussVector[]
  relativeResidual: number
}
export interface TrussBucklingResponse {
  /** Ascending |loadFactor|; the first entry is the critical mode. */
  modes: TrussBucklingMode[]
  /** Member axial forces of the reference state (tension positive). */
  axialForcesN: number[]
  freeDofs: number
}

/** Linear (eigenvalue) buckling of a pin-jointed truss under its own load
 * vector as the reference state. The bar geometric stiffness is transverse
 * only, so lattice shear flexibility is part of the answer. Not a certified
 * stability calculation. Requires the geometry runtime.
 */
export function solveTrussBuckling(model: TrussModel, modes: number): TrussBucklingResponse {
  return callGeometryRust<TrussBucklingResponse>('truss_buckling', {...model, modes})
}

/** How member mass is distributed: half per end ('lumped') or the isotropic
 * ρAL/6 block ('consistent'). */
export type TrussMassModel = 'lumped' | 'consistent'
export interface TrussModalMode {
  frequencyHz: number
  /** Angular frequency ω = 2πf. */
  omegaRadS: number
  /** Mode shape, max |component| = 1; restrained DOFs are zero. */
  displacementsMm: TrussVector[]
  relativeResidual: number
}
export interface TrussModalResponse {
  /** Ascending frequency. */
  modes: TrussModalMode[]
  /** Total bar mass in tonnes (1 t·mm/s² = 1 N with mm units). */
  totalMassT: number
  freeDofs: number
}

/** Small-displacement modal analysis: Kφ = ω²Mφ; densities in t/mm³ (steel
 * ≈ 7.85e-9), zero means a massless bar. Massless free DOFs have no finite
 * frequency and do not appear among the modes. Not a certified dynamic
 * calculation. Requires the geometry runtime.
 */
export function solveTrussModal(
  structure: Omit<TrussModel, 'forcesN'>,
  densitiesTMm3: number[],
  massModel: TrussMassModel,
  modes: number,
): TrussModalResponse {
  return callGeometryRust<TrussModalResponse>('truss_modal', {...structure, densitiesTMm3, massModel, modes})
}

/** Tuning of the corotational nonlinear solve. */
export interface TrussNonlinearOptions {
  /** Requested equal load increments (1-200); they subdivide adaptively. */
  steps: number
  /** Free-DOF residual ∞-norm relative to the force level (≤ 1e-3). */
  tolerance: number
  /** Newton iterations per increment (1-200). */
  maxIterations: number
}
export interface TrussNonlinearStep {
  loadFactor: number
  iterations: number
  relativeResidual: number
}
export interface TrussNonlinearResponse {
  displacementsMm: TrussVector[]
  reactionsN: TrussVector[]
  /** Positive means tension, from the current bar lengths. */
  axialForcesN: number[]
  axialStressesMpa: number[]
  /** Converged increments in order. */
  steps: TrussNonlinearStep[]
  /** Reached fraction of the reference load; 1 on full convergence. */
  loadFactor: number
  /** False when load control stalled at a limit point (snap-through): the
   * returned state is the last converged equilibrium on the path. */
  converged: boolean
}

/** Geometrically nonlinear corotational truss solve: large rotations and
 * displacements with small axial strain, N = EA·(L−L₀)/L₀ in the current
 * configuration. Newton iterations on the tangent stiffness advance in load
 * increments that halve when a step fails, tracing the equilibrium path up to
 * a load-controlled limit point. Linear elastic material; no buckling or
 * strength check. Requires the geometry runtime.
 */
export function solveTrussNonlinear(
  model: TrussModel,
  options: TrussNonlinearOptions,
): TrussNonlinearResponse {
  return callGeometryRust<TrussNonlinearResponse>('truss_nonlinear', {...model, ...options})
}

/** One weighted topology load case: per-node forces, or explicit wrenches
 * with the same schema as `solveTruss` (`loads`). */
export type TrussTopologyLoadCase = {weight: number} & (
  | {forcesN: TrussVector[]}
  | {loads: TrussNodalWrench[]}
)
export interface TrussTopologyOptions {
  /** Target material fraction of the ground-structure volume, (0, 1]. Default 0.5. */
  volumeFraction?: number
  /** SIMP stiffness penalty exponent, 1-10. Default 3. */
  penalty?: number
  /** OC iteration budget, 1-1000. Default 200. */
  maxIterations?: number
  /** Per-iteration density move limit, (0, 1]. Default 0.2. */
  moveLimit?: number
  /** Convergence on max |Δρ|, (0, 1). Default 1e-3. */
  tolerance?: number
}
export interface TrussTopologyResponse {
  /** Final densities in input member order, each in [1e-4, 1]. */
  densities: number[]
  /** Physical areas ρ·A_ref, matching the reported axial forces. */
  areasMm2: number[]
  volumeMm3: number
  targetVolumeMm3: number
  /** Per-case compliance of the final penalized model, N·mm. */
  compliancesNmm: number[]
  initialComplianceNmm: number
  complianceNmm: number
  iterations: number
  /** False means the iteration budget was exhausted, not invalid input. */
  converged: boolean
  maxDensityChange: number
  /** Members with final density ≥ 0.5, ascending input order. */
  activeMembers: number[]
  /** Per-case axial forces (tension positive) of the FINAL LINEAR model
   * with areas ρ·A_ref; composes with trussScreen. */
  axialForcesN: number[][]
}

/** Ground-structure SIMP topology optimization of a pin-jointed truss:
 * minimizes weighted compliance Σ w_c·F_cᵀu_c under a material volume
 * fraction constraint. `members.areaMm2` is the unit-density reference area.
 * The ground structure must be kinematically stable at full density.
 * Compliance layout estimate only — no buckling, joint or normative checks.
 * Requires the geometry runtime.
 */
export function optimizeTrussTopology(
  model: Omit<TrussModel, 'forcesN'>,
  cases: TrussTopologyLoadCase[],
  options: TrussTopologyOptions = {},
): TrussTopologyResponse {
  return callGeometryRust<TrussTopologyResponse>('truss_topology', {
    ...model,
    cases,
    options: {
      volumeFraction: options.volumeFraction ?? 0.5,
      penalty: options.penalty ?? 3,
      maxIterations: options.maxIterations ?? 200,
      moveLimit: options.moveLimit ?? 0.2,
      tolerance: options.tolerance ?? 1e-3,
    },
  })
}
