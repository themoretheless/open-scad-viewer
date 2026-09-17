import {callNurbsRust} from './geometry/nurbs'
import type {NurbsCurve} from './nurbsCurve'
import type {NurbsSurface} from './nurbsSurface'

export interface NurbsToleranceContext {
  spec: {
    version: number
    linearAbsMm: number
    linearRelative: number
    onMm: number
    clearMm: number
    angularRadians: number
    parametricFloor: number
    ulpGuard: number
    maxEntityErrorMm: number
    policy: string
  }
  identity: {version: number; canonical: string}
}

export interface NurbsFoundationEvidence {
  toleranceIdentity: {version: number; canonical: string}
  linearAbsoluteMm: number
  linearRelative: number
  parametricFloor: number
  maxEntityErrorMm: number
}
export interface NurbsSpanCertificate {
  domain: [number, number]
  min: number[]
  max: number[]
  denominatorLower: number
  denominatorUpper: number
  regularity: {
    classification: 'certified_regular' | 'singular' | 'unresolved'
    derivativeNumeratorBounds: [number, number][]
  }
}
export interface NurbsCurveFoundationCertificate {
  version: 'nurbs-foundation/1'
  kind: 'curve'
  rounding: 'binary64-nextafter-outward'
  basis: 'positive-rational-convex-hull'
  periodic: boolean
  period: number | null
  spans: NurbsSpanCertificate[]
  evidence: NurbsFoundationEvidence
}
export interface NurbsSurfaceFoundationCertificate {
  version: 'nurbs-foundation/1'
  kind: 'surface'
  rounding: 'binary64-nextafter-outward'
  basis: 'positive-rational-convex-hull'
  cells: Array<{
    domainU: [number, number]
    domainV: [number, number]
    min: [number, number, number]
    max: [number, number, number]
    denominatorLower: number
    denominatorUpper: number
    normalRegularity: {
      classification: 'certified_regular' | 'certified_planar_regular' | 'singular' | 'singular_or_unresolved' | 'unresolved'
      normalNumeratorBounds?: [number, number][]
    }
  }>
  evidence: NurbsFoundationEvidence
}

const toleranceArgs = (tolerance?: NurbsToleranceContext) => tolerance ? {tolerance} : {}

export const certifyNurbsCurveFoundation = (curve: NurbsCurve, tolerance?: NurbsToleranceContext): NurbsCurveFoundationCertificate =>
  callNurbsRust('curve_certify_foundation', {curve, ...toleranceArgs(tolerance)})

export const certifyNurbsSurfaceFoundation = (surface: NurbsSurface, tolerance?: NurbsToleranceContext): NurbsSurfaceFoundationCertificate =>
  callNurbsRust('surface_certify_foundation', {surface, ...toleranceArgs(tolerance)})

export interface NurbsProjectionCertificate {
  version: 'nurbs-foundation/1' | 'nurbs-foundation/2' | 'nurbs-foundation/3' | 'nurbs-foundation/4' | 'nurbs-foundation/5'
  status: 'unique' | 'nonunique' | 'nonunique_or_unresolved' | 'isolated_candidate'
  globalDistanceUpper: number
  candidates: Array<{
    domain: [number, number]
    parameterInterval: [number, number]
    point: number[]
    distanceLower: number
    distanceUpper: number
    classification: 'endpoint' | 'simple_stationary' | 'multiple_or_clustered_stationary'
  }>
  coverage?: {method: string; endpointsIncluded: boolean; stationaryContinuum: boolean; resourceLimit: number}
  uniquenessProof?: {winnerIndex: number; method: string} | null
  evidence: NurbsFoundationEvidence
}
export const projectPointToNurbsCurveCertified = (curve: NurbsCurve, point: number[], tolerance?: NurbsToleranceContext): NurbsProjectionCertificate =>
  callNurbsRust('curve_project_certified', {curve, point, ...toleranceArgs(tolerance)})

export interface NurbsSurfaceProjectionCertificate {
  version: 'nurbs-foundation/2' | 'nurbs-foundation/3' | 'nurbs-foundation/4'
  status: 'unique' | 'isolated_candidate' | 'nonunique_or_unresolved'
  globalDistanceUpper: number
  candidates: Array<{
    parameterBox: [number, number, number, number]
    point: [number, number, number]
    distanceLower: number
    distanceUpper: number
    classification: 'certified_global_candidate_box' | 'certified_unique_affine_projection' | 'certified_unique_krawczyk_or_separated'
  }>
  boundaryReductions: Array<{edge: string; certificate: NurbsProjectionCertificate}>
  coverage: {method: string; complete: true; subdivisions: number; resourceLimit: number}
  uniquenessProof?: Record<string, unknown> | null
  evidence: NurbsFoundationEvidence
}
export const projectPointToNurbsSurfaceCertified = (surface: NurbsSurface, point: [number, number, number], tolerance?: NurbsToleranceContext): NurbsSurfaceProjectionCertificate =>
  callNurbsRust('surface_project_certified', {surface, point, ...toleranceArgs(tolerance)})

export interface CertifiedNurbsCurveEdit {
  curve: NurbsCurve
  certificate: {
    version: 'nurbs-foundation/1' | 'nurbs-foundation/2' | 'nurbs-foundation/3' | 'nurbs-foundation/4' | 'nurbs-foundation/5'
    operation?: string
    geometryErrorUpper?: number
    hausdorffErrorUpper?: number
    dataSiteErrorUpper?: number
    withinEntityTolerance?: boolean
    periodPreserved?: boolean
    accepted?: boolean
    rolledBack?: boolean
    exactZeroRecognized?: boolean
    budget?: number
    evidence: NurbsFoundationEvidence
  }
}
export const interpolateNurbsPolylineCertified = (points: number[][], tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_interpolate_certified', {points, ...toleranceArgs(tolerance)})
export const approximateNurbsCurveCertified = (curve: NurbsCurve, tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_approximate_certified', {curve, ...toleranceArgs(tolerance)})
export const insertNurbsKnotCertified = (curve: NurbsCurve, u: number, count = 1, tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_insert_certified', {curve, u, count, ...toleranceArgs(tolerance)})
export const elevateNurbsCurveCertified = (curve: NurbsCurve, degree: number, tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_elevate_certified', {curve, degree, ...toleranceArgs(tolerance)})
export const removeNurbsCurveKnotCertified = (curve: NurbsCurve, u: number, maxError: number, tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_remove_certified', {curve, u, maxError, ...toleranceArgs(tolerance)})
export const reduceNurbsCurveDegreeCertified = (curve: NurbsCurve, degree: number, maxError: number, tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_reduce_certified', {curve, degree, maxError, ...toleranceArgs(tolerance)})

export interface CertifiedNurbsSurfaceEdit {
  surface: NurbsSurface
  certificate: CertifiedNurbsCurveEdit['certificate']
}
export const removeNurbsSurfaceKnotCertified = (surface: NurbsSurface, axis: 'u'|'v', u: number, maxError: number, tolerance?: NurbsToleranceContext): CertifiedNurbsSurfaceEdit =>
  callNurbsRust('surface_remove_certified', {surface, axis, u, maxError, ...toleranceArgs(tolerance)})
export const reduceNurbsSurfaceDegreeCertified = (surface: NurbsSurface, axis: 'u'|'v', degree: number, maxError: number, tolerance?: NurbsToleranceContext): CertifiedNurbsSurfaceEdit =>
  callNurbsRust('surface_reduce_certified', {surface, axis, degree, maxError, ...toleranceArgs(tolerance)})

export type NurbsPeriodicEditOperation = 'insert'|'remove'|'elevate'|'reduce'
export const editPeriodicNurbsCurveCertified = (curve: NurbsCurve, operation: NurbsPeriodicEditOperation, options: {u?:number;degree?:number;maxError:number}, tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_periodic_edit_certified', {curve, operation, ...options, ...toleranceArgs(tolerance)})
export const splitPeriodicNurbsCurveCertified = (curve: NurbsCurve, u: number, tolerance?: NurbsToleranceContext): {curves:[NurbsCurve,NurbsCurve];certificate:Record<string,unknown>} =>
  callNurbsRust('curve_periodic_split_certified', {curve, u, ...toleranceArgs(tolerance)})
export const editPeriodicNurbsSurfaceCertified = (surface: NurbsSurface, axis: 'u'|'v', operation: NurbsPeriodicEditOperation, options: {u?:number;degree?:number;maxError:number}, tolerance?: NurbsToleranceContext): CertifiedNurbsSurfaceEdit =>
  callNurbsRust('surface_periodic_edit_certified', {surface, axis, operation, ...options, ...toleranceArgs(tolerance)})

export interface RationalReparameterizationPiece {
  domain:[number,number]
  range:[number,number]
  controlValues:number[]
  weights:number[]
}
export type RationalReparameterization = {pieces:RationalReparameterizationPiece[]} | {composition:RationalReparameterization[]}
export const certifyNurbsReparameterization = (mapping:RationalReparameterization,tolerance?:NurbsToleranceContext) =>
  callNurbsRust('curve_reparameterization_certify',{mapping,...toleranceArgs(tolerance)})
export const evaluateReparameterizedNurbsCurve = (curve:NurbsCurve,mapping:RationalReparameterization,u:number,tolerance?:NurbsToleranceContext) =>
  callNurbsRust('curve_reparameterized_evaluate',{curve,mapping,u,...toleranceArgs(tolerance)})
export const fitNurbsCurveCertified = (points:number[][],controlCount:number,tolerance?:NurbsToleranceContext):CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_fit_certified',{points,controlCount,...toleranceArgs(tolerance)})
export const interpolateNurbsSurfaceCertified = (points:[number,number,number][][],tolerance?:NurbsToleranceContext):CertifiedNurbsSurfaceEdit =>
  callNurbsRust('surface_interpolate_certified',{points,...toleranceArgs(tolerance)})
export const fitNurbsSurfaceCertified = (points:[number,number,number][][],tolerance?:NurbsToleranceContext):CertifiedNurbsSurfaceEdit =>
  callNurbsRust('surface_fit_certified',{points,...toleranceArgs(tolerance)})
export const materializeReparameterizedNurbsCurve = (curve:NurbsCurve,mapping:RationalReparameterization,tolerance?:NurbsToleranceContext):CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_materialize_reparameterization',{curve,mapping,...toleranceArgs(tolerance)})
export const fitNurbsCurveCloudCertified = (points:number[][],controlCount:number,tolerance?:NurbsToleranceContext):CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_fit_cloud_certified',{points,controlCount,...toleranceArgs(tolerance)})
export const fitNurbsSurfaceCloudCertified = (points:[number,number,number][],controlsU:number,controlsV:number,tolerance?:NurbsToleranceContext):CertifiedNurbsSurfaceEdit =>
  callNurbsRust('surface_fit_cloud_certified',{points,controlsU,controlsV,...toleranceArgs(tolerance)})
export const reparameterizeNurbsCurveExact = (curve: NurbsCurve, domain: [number, number], tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_reparameterize_exact', {curve, domain, ...toleranceArgs(tolerance)})

export type NurbsContactClass =
  | 'transverse'
  | 'odd_tangency'
  | 'even_tangency'
  | 'higher_order_contact'
  | 'coincident'
  | 'boundary'
  | 'pole_or_singular'

export interface NurbsCoedgeTrim {
  curveParameter: [number, number]
  pcurveParameter: [number, number]
  periodicLift: [[number, number], [number, number]]
}

export interface NurbsIntersectionCoverage {
  method: string
  complete: boolean
  boxesVisited: number
  bernsteinExcluded: number
  krawczykIsolated: number
  resourceLimit: number
}

export interface NurbsCurveCurveIntersectionCertificate {
  version: 'nurbs-foundation/5'
  kind: 'curve_curve'
  coverage: NurbsIntersectionCoverage
  components: Array<Record<string, unknown>>
  unresolved: Array<Record<string, unknown>>
  rounding: 'binary64-nextafter-outward'
  evidence: NurbsFoundationEvidence
}

export interface NurbsCurveSurfaceIntersectionCertificate {
  version: 'nurbs-foundation/5'
  kind: 'curve_surface'
  coverage: NurbsIntersectionCoverage
  components: Array<Record<string, unknown>>
  unresolved: Array<Record<string, unknown>>
  rounding: 'binary64-nextafter-outward'
  evidence: NurbsFoundationEvidence
}

export const intersectNurbsCurveCurveCertified = (
  first: NurbsCurve,
  second: NurbsCurve,
  tolerance?: NurbsToleranceContext,
): NurbsCurveCurveIntersectionCertificate =>
  callNurbsRust('curve_curve_intersect_certified', {first, second, ...toleranceArgs(tolerance)})

export const intersectNurbsCurveSurfaceCertified = (
  curve: NurbsCurve,
  surface: NurbsSurface,
  tolerance?: NurbsToleranceContext,
): NurbsCurveSurfaceIntersectionCertificate =>
  callNurbsRust('curve_surface_intersect_certified', {curve, surface, ...toleranceArgs(tolerance)})

export interface NurbsSurfaceSurfaceIntersectionCertificate {
  version: 'nurbs-ss/1'
  kind: 'surface_surface'
  coverage: NurbsIntersectionCoverage & {missedBranchProof?: boolean}
  components: Array<Record<string, unknown>>
  branchGraph: {
    components: Array<Record<string, unknown>>
    certificate: Record<string, unknown>
    permitsTopologyAuthorship: boolean
  }
  uvArrangement: Record<string, unknown>
  unresolved: Array<Record<string, unknown>>
  rounding: 'binary64-nextafter-outward'
  evidence: NurbsFoundationEvidence
  topologyAuthority: Record<string, unknown>
  booleanMutationAuthority: false
}

export const intersectNurbsSurfaceSurfaceCertified = (
  first: NurbsSurface,
  second: NurbsSurface,
  tolerance?: NurbsToleranceContext,
): NurbsSurfaceSurfaceIntersectionCertificate =>
  callNurbsRust('surface_surface_intersect_certified', {first, second, ...toleranceArgs(tolerance)})

export const verifyNurbsSurfaceSurfaceCoverage = (report: NurbsSurfaceSurfaceIntersectionCertificate) =>
  callNurbsRust('surface_surface_verify_coverage', {report})
