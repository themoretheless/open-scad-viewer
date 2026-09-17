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
    normalRegularity: {classification: 'certified_planar_regular' | 'singular_or_unresolved' | 'unresolved'}
  }>
  evidence: NurbsFoundationEvidence
}

const toleranceArgs = (tolerance?: NurbsToleranceContext) => tolerance ? {tolerance} : {}

export const certifyNurbsCurveFoundation = (curve: NurbsCurve, tolerance?: NurbsToleranceContext): NurbsCurveFoundationCertificate =>
  callNurbsRust('curve_certify_foundation', {curve, ...toleranceArgs(tolerance)})

export const certifyNurbsSurfaceFoundation = (surface: NurbsSurface, tolerance?: NurbsToleranceContext): NurbsSurfaceFoundationCertificate =>
  callNurbsRust('surface_certify_foundation', {surface, ...toleranceArgs(tolerance)})

export interface NurbsProjectionCertificate {
  version: 'nurbs-foundation/1'
  status: 'unique' | 'nonunique_or_unresolved' | 'isolated_candidate'
  globalDistanceUpper: number
  candidates: Array<{
    domain: [number, number]
    parameterInterval: [number, number]
    point: number[]
    distanceLower: number
    distanceUpper: number
    classification: 'certified_unique_on_span' | 'candidate'
  }>
  evidence: NurbsFoundationEvidence
}
export const projectPointToNurbsCurveCertified = (curve: NurbsCurve, point: number[], tolerance?: NurbsToleranceContext): NurbsProjectionCertificate =>
  callNurbsRust('curve_project_certified', {curve, point, ...toleranceArgs(tolerance)})

export interface CertifiedNurbsCurveEdit {
  curve: NurbsCurve
  certificate: {
    version: 'nurbs-foundation/1'
    operation?: string
    geometryErrorUpper?: number
    hausdorffErrorUpper?: number
    dataSiteErrorUpper?: number
    withinEntityTolerance?: boolean
    periodPreserved?: boolean
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
export const reparameterizeNurbsCurveExact = (curve: NurbsCurve, domain: [number, number], tolerance?: NurbsToleranceContext): CertifiedNurbsCurveEdit =>
  callNurbsRust('curve_reparameterize_exact', {curve, domain, ...toleranceArgs(tolerance)})
