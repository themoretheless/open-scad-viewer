import { callNurbsRust } from './geometry/nurbs'
import type { NurbsCurve } from './nurbsCurve'
import type { NurbsSurface } from './nurbsSurface'
import type { RationalReparameterization } from './nurbsFoundation'

export interface CartesianGordonResult {
  surface: NurbsSurface
  certificate: {
    operation: 'cartesian-gordon'
    exact: false
    fittedToExactPromotion: false
    tangents?: BoundaryVTangentCertificate[]
    curves: Array<{
      accepted: true
      exact: false
      tolerance: number
      errorUpper: number
      cells: number
      mapEvaluations: number
    }>
  }
}

/** Retains each authored curve with a bounded whole-domain numerical audit. */
export function createCartesianGordonSurface(
  uCurves: NurbsCurve[],
  vCurves: NurbsCurve[],
  parametersU: number[],
  parametersV: number[],
  errorBudget = 1e-6,
  maxCells = 50000,
  maxMapEvaluations = 200000,
): CartesianGordonResult {
  return callNurbsRust<CartesianGordonResult>('surface_gordon_cartesian', {
    u_curves: uCurves,
    v_curves: vCurves,
    parameters_u: parametersU,
    parameters_v: parametersV,
    errorBudget,
    maxCells,
    maxMapEvaluations,
  })
}

export interface CartesianGuidedLoftResult {
  surface: NurbsSurface
  sections?: NurbsCurve[]
  section_mapping_certificates?: Array<Record<string, unknown>>
  original_section_certificates?: Array<Record<string, unknown>>
  certificate: {
    operation: 'cartesian-guided-loft'
    exact: false
    fittedToExactPromotion: false
    tangents?: BoundaryVTangentCertificate[]
    curves: CartesianGordonResult['certificate']['curves']
  }
}

/** Audit budgets apply to each authored curve and generated boundary guide. */
export function createCartesianGuidedLoft(
  curves: NurbsCurve[],
  parameters: number[],
  guides: NurbsCurve[],
  guideParameters: number[],
  errorBudget = 1e-6,
  maxCells = 50000,
  maxMapEvaluations = 200000,
  sectionMappings?: Array<RationalReparameterization | null>,
): CartesianGuidedLoftResult {
  return callNurbsRust<CartesianGuidedLoftResult>('surface_guided_loft_cartesian', {
    ...(sectionMappings ? { section_mappings: sectionMappings } : {}),
    curves,
    parameters,
    guides,
    guide_parameters: guideParameters,
    errorBudget,
    maxCells,
    maxMapEvaluations,
  })
}

export interface AutoCartesianGuidedLoftResult {
  surface: NurbsSurface
  guides: NurbsCurve[]
  guide_parameters: number[]
  guide_order: number[]
  reversed: boolean[]
  section_error_upper: number[]
  guide_error_upper: number[]
  sections?: NurbsCurve[]
  section_mapping_certificates?: Array<Record<string, unknown> | null>
  original_section_certificates?: Array<Record<string, unknown>>
  certificate: {
    operation: 'cartesian-auto-guided-loft'
    exact: false
    fittedToExactPromotion: false
    tangents?: BoundaryVTangentCertificate[]
    curves: CartesianGordonResult['certificate']['curves']
  }
}

export function createAutoCartesianGuidedLoft(
  curves: NurbsCurve[],
  parameters: number[],
  guides: NurbsCurve[],
  budget = 1e-6,
  parameterTolerance = 1e-8,
  maxCells = 50000,
  maxMapEvaluations = 200000,
  sectionMappings?: Array<RationalReparameterization | null>,
): AutoCartesianGuidedLoftResult {
  return callNurbsRust<AutoCartesianGuidedLoftResult>('surface_auto_guided_loft_cartesian', {
    curves,
    parameters,
    guides,
    budget,
    parameter_tolerance: parameterTolerance,
    maxCells,
    maxMapEvaluations,
    ...(sectionMappings ? { section_mappings: sectionMappings } : {}),
  })
}

export interface BoundaryVTangentCertificate {
  operation: 'cartesian-boundary-tangent-retention'
  exact: false
  accepted: boolean
  errorUpper: number | null
  tolerance: number
  cells: number
  maxCells: number
  domain: [number, number]
  end: boolean
  unresolvedReason: string | null
  method: 'outward-original-tensor-quotient-rule-Bernstein-residual'
  targetUnits?: 'authored-dP/dt'
  stationDomain?: [number, number]
  normalizedScaleEnclosure?: [number, number]
}

/** Targets describe dP/dv and must cover the complete authored U domain. */
export function certifyNurbsSurfaceBoundaryVTangent(
  surface: NurbsSurface,
  targets: NurbsCurve[],
  end = false,
  tolerance = 1e-6,
  maxCells = 50000,
): BoundaryVTangentCertificate {
  return callNurbsRust<BoundaryVTangentCertificate>('surface_boundary_v_tangent_certify', {
    surface,
    targets,
    end,
    tolerance,
    maxCells,
  })
}

/** Boundary fields describe dP/dV with both U and V normalized to [0,1]. */
export function createClampedCartesianGordonSurface(
  uCurves: NurbsCurve[],
  vCurves: NurbsCurve[],
  parametersU: number[],
  parametersV: number[],
  boundaryTangents: [NurbsCurve, NurbsCurve],
  errorBudget = 1e-6,
  maxCells = 50000,
  maxMapEvaluations = 200000,
): CartesianGordonResult {
  return callNurbsRust<CartesianGordonResult>('surface_gordon_cartesian', {
    u_curves: uCurves,
    v_curves: vCurves,
    parameters_u: parametersU,
    parameters_v: parametersV,
    boundary_tangents: boundaryTangents,
    errorBudget,
    maxCells,
    maxMapEvaluations,
  })
}

/** Boundary fields use normalized dP/dV, independently of authored section stations. */
export function createClampedCartesianGuidedLoft(
  curves: NurbsCurve[],
  parameters: number[],
  guides: NurbsCurve[],
  guideParameters: number[],
  boundaryTangents: [NurbsCurve, NurbsCurve],
  errorBudget = 1e-6,
  maxCells = 50000,
  maxMapEvaluations = 200000,
  sectionMappings?: Array<RationalReparameterization | null>,
): CartesianGuidedLoftResult {
  return callNurbsRust<CartesianGuidedLoftResult>('surface_guided_loft_cartesian', {
    ...(sectionMappings ? { section_mappings: sectionMappings } : {}),
    curves,
    parameters,
    guides,
    guide_parameters: guideParameters,
    boundary_tangents: boundaryTangents,
    errorBudget,
    maxCells,
    maxMapEvaluations,
  })
}

/** Targets are normalized dP/dV fields after guide orientation and station mapping. */
export function createAutoClampedCartesianGuidedLoft(
  curves: NurbsCurve[],
  parameters: number[],
  guides: NurbsCurve[],
  boundaryTangents: [NurbsCurve, NurbsCurve],
  budget = 1e-6,
  parameterTolerance = 1e-8,
  maxCells = 50000,
  maxMapEvaluations = 200000,
  sectionMappings?: Array<RationalReparameterization | null>,
): AutoCartesianGuidedLoftResult {
  return callNurbsRust<AutoCartesianGuidedLoftResult>('surface_auto_guided_loft_cartesian', {
    curves,
    parameters,
    guides,
    boundary_tangents: boundaryTangents,
    budget,
    parameter_tolerance: parameterTolerance,
    maxCells,
    maxMapEvaluations,
    ...(sectionMappings ? { section_mappings: sectionMappings } : {}),
  })
}

/** Controls use the aligned section U basis and authored station dP/dt units. */
export function createControlTangentCartesianGuidedLoft(
  curves: NurbsCurve[],
  parameters: number[],
  guides: NurbsCurve[],
  guideParameters: number[],
  startTangents: [number, number, number][],
  endTangents: [number, number, number][],
  errorBudget = 1e-6,
  maxCells = 50000,
  maxMapEvaluations = 200000,
  sectionMappings?: Array<RationalReparameterization | null>,
): CartesianGuidedLoftResult {
  return callNurbsRust<CartesianGuidedLoftResult>('surface_guided_loft_cartesian', {
    ...(sectionMappings ? { section_mappings: sectionMappings } : {}),
    curves,
    parameters,
    guides,
    guide_parameters: guideParameters,
    start_tangents: startTangents,
    end_tangents: endTangents,
    errorBudget,
    maxCells,
    maxMapEvaluations,
  })
}

/** Original dP/dt controls are certified after automatic guide alignment. */
export function createAutoControlTangentCartesianGuidedLoft(
  curves: NurbsCurve[],
  parameters: number[],
  guides: NurbsCurve[],
  startTangents: [number, number, number][],
  endTangents: [number, number, number][],
  budget = 1e-6,
  parameterTolerance = 1e-8,
  maxCells = 50000,
  maxMapEvaluations = 200000,
  sectionMappings?: Array<RationalReparameterization | null>,
): AutoCartesianGuidedLoftResult {
  return callNurbsRust<AutoCartesianGuidedLoftResult>('surface_auto_guided_loft_cartesian', {
    curves,
    parameters,
    guides,
    start_tangents: startTangents,
    end_tangents: endTangents,
    budget,
    parameter_tolerance: parameterTolerance,
    maxCells,
    maxMapEvaluations,
    ...(sectionMappings ? { section_mappings: sectionMappings } : {}),
  })
}

/** A mapped natural loft carries whole-domain audits against the authored sections. */
export interface MappedNaturalLoftResult {
  surface: NurbsSurface
  sections: NurbsCurve[]
  section_mapping_certificates: Array<Record<string, unknown>>
  original_section_certificates: Array<Record<string, unknown>>
  certificate: { operation: 'mapped-natural-loft'; exact: false; fittedToExactPromotion: false }
}
export function createMappedNaturalLoft(
  curves: NurbsCurve[],
  parameters: number[],
  sectionMappings: Array<RationalReparameterization | null>,
): MappedNaturalLoftResult {
  return callNurbsRust<MappedNaturalLoftResult>('surface_natural_loft_checked', {
    curves,
    parameters,
    section_mappings: sectionMappings,
  })
}
