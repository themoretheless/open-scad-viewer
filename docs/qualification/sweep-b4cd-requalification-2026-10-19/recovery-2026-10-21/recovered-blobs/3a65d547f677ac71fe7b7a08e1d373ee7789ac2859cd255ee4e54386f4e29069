import {callGeometryRust} from './kernel'
import type {NurbsCurve} from '../nurbsCurve'

export type BrepProfileFillRule = 'material-left' | 'even-odd'
export type BrepProfileBooleanOperation = 'union' | 'intersection' | 'difference' | 'xor'
/** Retained positive-weight rational Bézier region; empty material has loops=[] and areaMm2=0. */
export interface BrepProfile {
  kind: 'brep-profile'
  loops: NurbsCurve[][]
  areaMm2: number
  toleranceMm: number
  geometryStatus: 'numerical_uncertified'|'bounded_approximation'
  minkowskiCertificate?:BrepMinkowskiCertificate
  maximumSourceDeviationMm?:number
  regionStatus?:'inclusion_verified'
}

/** Material-left input is checked as authored. Even-odd input reverses rings
 * according to complete analytic nesting; crossing/ambiguous rings are refused. */
export function validateBrepProfile(loops:NurbsCurve[][], fillRule:BrepProfileFillRule='material-left', toleranceMm=1e-7):BrepProfile {
  return callGeometryRust('brep_profile_validate',{loops,fillRule,toleranceMm})
}

/** Retains source NURBS trims; native geometry supplies every boundary decision. */
export function booleanBrepProfiles(a:BrepProfile,b:BrepProfile,operation:BrepProfileBooleanOperation,toleranceMm=Math.max(a.toleranceMm,b.toleranceMm)):BrepProfile {
  return callGeometryRust('brep_profile_boolean',{a:a.loops,b:b.loops,operation,toleranceMm})
}

/** Signed Green area of one connected retained curve loop, without a mesh. */
export function signedAreaBrepProfileLoop(loop:NurbsCurve[],toleranceMm=1e-7):number {
  return callGeometryRust('brep_profile_signed_area',{loop,toleranceMm})
}

export type AuthoredBrepProfile = {kind:'bezier';path:import('./path2d').BezierPathJson}| {kind:'circle';radius:number}|{kind:'rectangle';size:readonly number[];center:boolean}|{kind:'polygon';rings:readonly (readonly (readonly number[])[])[]}
export const authorBrepProfile=(definition:AuthoredBrepProfile):BrepProfile=>callGeometryRust('brep_profile_author',definition)

export const transformBrepProfile=(profile:BrepProfile,matrix:readonly number[]):BrepProfile=>callGeometryRust('brep_profile_transform',{loops:profile.loops,matrix,toleranceMm:profile.toleranceMm})

/** Round Euclidean region offset. Line/circle inputs retain their analytic route;
 * general curves use a nominal polygon plus checked exact region inclusions. */
export const offsetBrepProfile=(profile:BrepProfile,distance:number,maximumSourceDeviationMm=.02):BrepProfile=>callGeometryRust('brep_profile_offset',{loops:profile.loops,distance,maximumSourceDeviationMm,toleranceMm:profile.toleranceMm})

/** Split resolved self-intersections with even-odd fill, retaining source trims. */
export const normalizeBrepProfile=(loops:NurbsCurve[][],toleranceMm=1e-7):BrepProfile=>callGeometryRust('brep_profile_normalize',{loops,toleranceMm})

/** Portable proof witnesses. Rust verifies topology of these exact stored bits;
 * area, construction history and solid validity are separate claims. */
export interface BrepProfileCertificate {
  version:number
  scope:'represented-profile-topology'
  source:string
  subdivisionDepth:number
  monotoneAxes:number[]
  separatingNormals:number[]
  orientation:number[]
  orientationDepth:number|null
  nestingDepth:number[]
}
export const certifyBrepProfile=(profile:Pick<BrepProfile,'loops'|'toleranceMm'>):BrepProfileCertificate=>callGeometryRust('brep_profile_certify',{loops:profile.loops,toleranceMm:profile.toleranceMm})
export const verifyBrepProfileCertificate=(profile:Pick<BrepProfile,'loops'|'toleranceMm'>,certificate:BrepProfileCertificate):{status:'verified';scope:'represented-profile-topology';spans:number;pieces:number;nestingDepth:number[]}=>callGeometryRust('brep_profile_verify_certificate',{loops:profile.loops,toleranceMm:profile.toleranceMm,certificate})

/** Exact decimal integer ratios in mm²; UI must not relabel a rounded Number
 * as an exact bound. The scope still excludes construction and 3D validity. */
export interface BrepProfileAreaInterval {lower:[string,string];upper:[string,string]}
export interface BrepProfileMeasureCertificate {
  version:number
  scope:'represented-profile-topology-and-area'
  topology:BrepProfileCertificate
  areaDepth:number
  areaMm2:BrepProfileAreaInterval
}
export const certifyBrepProfileMeasure=(profile:Pick<BrepProfile,'loops'|'toleranceMm'>,maximumAreaWidthMm2:number):BrepProfileMeasureCertificate=>callGeometryRust('brep_profile_certify_measure',{loops:profile.loops,toleranceMm:profile.toleranceMm,maximumAreaWidthMm2})
export const verifyBrepProfileMeasureCertificate=(profile:Pick<BrepProfile,'loops'|'toleranceMm'>,certificate:BrepProfileMeasureCertificate):{status:'verified';scope:'represented-profile-topology-and-area';spans:number;pieces:number;nestingDepth:number[];areaMm2:BrepProfileAreaInterval}=>callGeometryRust('brep_profile_verify_measure_certificate',{loops:profile.loops,toleranceMm:profile.toleranceMm,certificate})

export interface BrepCurveAgreementCertificate {
  version:number
  scope:'represented-curve-affine-parameter-agreement'
  source:string
  commonSpans:number
}
export const certifyBrepCurveAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number]):BrepCurveAgreementCertificate=>callGeometryRust('brep_curve_certify_agreement',{first,second,firstDomain,secondDomain})
export const verifyBrepCurveAgreementCertificate=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number],certificate:BrepCurveAgreementCertificate):{status:'verified';scope:'represented-curve-affine-parameter-agreement';commonSpans:number}=>callGeometryRust('brep_curve_verify_agreement_certificate',{first,second,firstDomain,secondDomain,certificate})

export interface BrepCurveProjectiveAgreementCertificate {version:number;scope:'represented-curve-projective-parameter-agreement';source:string;commonSpans:number}
export const certifyBrepCurveProjectiveAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number],parameterScale:number):BrepCurveProjectiveAgreementCertificate=>callGeometryRust('brep_curve_certify_projective_agreement',{first,second,firstDomain,secondDomain,parameterScale})
export const verifyBrepCurveProjectiveAgreementCertificate=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number],parameterScale:number,certificate:BrepCurveProjectiveAgreementCertificate):{status:'verified';scope:'represented-curve-projective-parameter-agreement';commonSpans:number}=>callGeometryRust('brep_curve_verify_projective_agreement_certificate',{first,second,firstDomain,secondDomain,parameterScale,certificate})
export interface BrepCurveEndpointContactCertificate {version:number;scope:'represented-endpoint-intersection-set';source:string;separator:number;commonEndpoints:number}
export const certifyBrepCurveEndpointContacts=(first:NurbsCurve,second:NurbsCurve):BrepCurveEndpointContactCertificate=>callGeometryRust('brep_curve_certify_endpoint_contacts',{first,second})
export const verifyBrepCurveEndpointContactsCertificate=(first:NurbsCurve,second:NurbsCurve,certificate:BrepCurveEndpointContactCertificate):{status:'verified';scope:'represented-endpoint-intersection-set';commonEndpoints:number}=>callGeometryRust('brep_curve_verify_endpoint_contacts_certificate',{first,second,certificate})
export interface BrepCurveOffsetCertificate {version:number;scope:'represented-normal-offset-curve-approximation';source:string;parameters:number[];points:[number,number][]}
/** Approximation of the parameterized normal offset of a regular curve.
 * This does not assert filled-region joins, Boolean validity or a regular solid. */
export const offsetBrepCurveCertified=(curve:NurbsCurve,distanceMm:number,maximumDeviationMm:number):{curves:NurbsCurve[];certificate:BrepCurveOffsetCertificate;distanceMm:number;maximumDeviationMm:number;geometryStatus:'bounded_approximation'}=>callGeometryRust('brep_curve_offset_certified',{curve,distanceMm,maximumDeviationMm})
export const verifyBrepCurveOffsetCertificate=(curve:NurbsCurve,distanceMm:number,maximumDeviationMm:number,certificate:BrepCurveOffsetCertificate):{status:'verified';scope:'represented-normal-offset-curve-approximation';segments:number;maximumDeviationMm:number}=>callGeometryRust('brep_curve_verify_offset_certificate',{curve,distanceMm,maximumDeviationMm,certificate})

/** Algebraic correspondence witness; no supplied affine/projective map is assumed. */
export interface BrepCurveNonlinearAgreementCertificate {version:number;scope:'represented-curve-nonlinear-image-agreement';source:string;axis:number;relation:[string,string][][]}
export const certifyBrepCurveNonlinearAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number]):BrepCurveNonlinearAgreementCertificate=>callGeometryRust('brep_curve_certify_nonlinear_agreement',{first,second,firstDomain,secondDomain})
export const verifyBrepCurveNonlinearAgreementCertificate=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number],certificate:BrepCurveNonlinearAgreementCertificate):{status:'verified';scope:'represented-curve-nonlinear-image-agreement';commonSpans:number}=>callGeometryRust('brep_curve_verify_nonlinear_agreement_certificate',{first,second,firstDomain,secondDomain,certificate})
export interface BrepCurveIntersectionCertificate {version:number;scope:'represented-isolated-curve-intersection-set';source:string;selfPair:boolean;monotoneAxis:number|null;projectionParameters:number[];events:[number,number][];singularParameters:number[]}
export const certifyBrepCurveIntersections=(first:NurbsCurve,second:NurbsCurve,selfPair=false):BrepCurveIntersectionCertificate=>callGeometryRust('brep_curve_certify_intersections',{first,second,selfPair})
export const verifyBrepCurveIntersectionsCertificate=(first:NurbsCurve,second:NurbsCurve,certificate:BrepCurveIntersectionCertificate):{status:'verified';scope:'represented-isolated-curve-intersection-set';events:[number,number][];singularParameters:number[]}=>callGeometryRust('brep_curve_verify_intersections_certificate',{first,second,certificate})
export type BrepOffsetJoinStyle='bevel'|'miter'|'round'
export interface BrepOffsetWireCertificate {version:number;scope:'represented-joined-normal-offset-wire-approximation';source:string;closed:boolean;joinStyle:BrepOffsetJoinStyle;parts:{parameters:number[];points:[number,number][]}[];joins:[number,number][][]}
/** Joined one-sided offset wire with an independently checked Hausdorff bound.
 * Filled-region clipping/topology and solid validity remain separate claims. */
export const offsetBrepWireCertified=(curves:NurbsCurve[],distanceMm:number,maximumDeviationMm:number,closed:boolean,joinStyle:BrepOffsetJoinStyle,miterLimitMm:number):{curves:NurbsCurve[];certificate:BrepOffsetWireCertificate;distanceMm:number;maximumDeviationMm:number;geometryStatus:'bounded_approximation'}=>callGeometryRust('brep_profile_offset_wire_certified',{curves,distanceMm,maximumDeviationMm,closed,joinStyle,miterLimitMm})
export const verifyBrepOffsetWireCertificate=(curves:NurbsCurve[],distanceMm:number,maximumDeviationMm:number,miterLimitMm:number,certificate:BrepOffsetWireCertificate):{status:'verified';scope:'represented-joined-normal-offset-wire-approximation';segments:number;maximumDeviationMm:number}=>callGeometryRust('brep_profile_verify_offset_wire_certificate',{curves,distanceMm,maximumDeviationMm,miterLimitMm,certificate})

export type BrepExactRatio=[string,string]
export type BrepParameterInterval=[number,number]
/** Complete isolated event census. The roots are exact algebraic witnesses;
 * returned binary64 intervals are outward enclosures, never exact parameters. */
export interface BrepAlgebraicIntersectionCertificate {
  version:number;scope:'represented-algebraic-curve-intersection-set';source:string
  selfPair:boolean;maximumParameterWidth:number;injectiveAxis:number|null;projectionShear:number
  factors:{polynomial:BrepExactRatio[];parameterMap:BrepExactRatio[]|null;roots:{interval:[BrepExactRatio,BrepExactRatio];kind:'event'|'diagonal'|'reverse'|'outside'}[]}[]
}
export const certifyBrepAlgebraicIntersections=(first:NurbsCurve,second:NurbsCurve,selfPair=false,maximumParameterWidth=1e-12):BrepAlgebraicIntersectionCertificate=>callGeometryRust('brep_curve_certify_algebraic_intersections',{first,second,selfPair,maximumParameterWidth})
export const verifyBrepAlgebraicIntersections=(first:NurbsCurve,second:NurbsCurve,certificate:BrepAlgebraicIntersectionCertificate):{status:'verified';scope:'represented-algebraic-curve-intersection-set';stationaryCensusComplete:boolean;events:[BrepParameterInterval,BrepParameterInterval][];singularParameters:BrepParameterInterval[]}=>callGeometryRust('brep_curve_verify_algebraic_intersections_certificate',{first,second,certificate})

/** Image coverage across unequal knot partitions and nonlinear speeds. */
export interface BrepMultispanAgreementCertificate {
  version:number;scope:'represented-multispan-curve-image-agreement';source:string;subdivisionDepth:number
  steps:{firstCell:number;secondCell:number;axis:number;relation:BrepExactRatio[][];branches?:BrepBranchDecomposition}[]
}
export const certifyBrepMultispanAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number]):BrepMultispanAgreementCertificate=>callGeometryRust('brep_curve_certify_multispan_agreement',{first,second,firstDomain,secondDomain})
export const verifyBrepMultispanAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number],certificate:BrepMultispanAgreementCertificate):{status:'verified';scope:'represented-multispan-curve-image-agreement';commonSpans:number}=>callGeometryRust('brep_curve_verify_multispan_agreement_certificate',{first,second,firstDomain,secondDomain,certificate})

export interface BrepOffsetRegionCertificate {
  version:number;scope:'represented-offset-positive-region-regularization';source:string;joinStyle:BrepOffsetJoinStyle
  originalTopology:BrepProfileCertificate;wires:BrepOffsetWireCertificate[];exactRings:[BrepExactRatio,BrepExactRatio][][]
}
/** Positive-winding cleanup of bounded joined offset wires, with exact crossing
 * and overlap removal. This scope is distinct from a Minkowski-region proof. */
export const offsetBrepRegionsCertified=(profile:Pick<BrepProfile,'loops'|'toleranceMm'>,distanceMm:number,maximumDeviationMm:number,joinStyle:BrepOffsetJoinStyle='round',miterLimitMm=Math.max(1,Math.abs(distanceMm)*4)):{components:BrepProfile[];certificate:BrepOffsetRegionCertificate;distanceMm:number;maximumDeviationMm:number;geometryStatus:'bounded_approximation';regularizationStatus:'verified'}=>callGeometryRust('brep_profile_offset_regions_certified',{loops:profile.loops,toleranceMm:profile.toleranceMm,distanceMm,maximumDeviationMm,joinStyle,miterLimitMm})
export const verifyBrepOffsetRegions=(profile:Pick<BrepProfile,'loops'>,distanceMm:number,maximumDeviationMm:number,miterLimitMm:number,certificate:BrepOffsetRegionCertificate):{status:'verified';scope:'represented-offset-positive-region-regularization';components:number;rings:number}=>callGeometryRust('brep_profile_verify_offset_regions_certificate',{loops:profile.loops,distanceMm,maximumDeviationMm,miterLimitMm,certificate})


export type BrepExactRegionRings=[BrepExactRatio,BrepExactRatio][][]
/** Inclusion witnesses for filled regularized sets. The source/chord and disk
 * bounds are separate from any topology or Hausdorff claim for the final result. */
export interface BrepMinkowskiCertificate {
 version:number;scope:'represented-filled-minkowski-region-inclusions';source:string;operation:'sum'|'offset'
 maximumSourceDeviationMm:number;distanceMm:number;firstPartition:string[][];secondPartition:string[][]
 diskInner:BrepExactRegionRings;diskOuter:BrepExactRegionRings;nominal:BrepExactRegionRings;inner:BrepExactRegionRings;outer:BrepExactRegionRings
}
export interface BrepMinkowskiResult {
 components:BrepProfile[];certificate:BrepMinkowskiCertificate;nominalExactRings:BrepExactRegionRings;innerExactRings:BrepExactRegionRings;outerExactRings:BrepExactRegionRings
 exactSetResult:boolean;geometryStatus:'bounded_approximation';regionStatus:'inclusion_verified';modelingStatus:'validated'|'refused';modelingRefusal:string|null
}
export const minkowskiBrepProfiles=(a:Pick<BrepProfile,'loops'>,b:Pick<BrepProfile,'loops'>,maximumSourceDeviationMm=.02):BrepMinkowskiResult=>callGeometryRust('brep_profile_minkowski_sum',{a:a.loops,b:b.loops,maximumSourceDeviationMm})
export const minkowskiOffsetBrepProfile=(a:Pick<BrepProfile,'loops'>,distanceMm:number,maximumSourceDeviationMm=.02):BrepMinkowskiResult=>callGeometryRust('brep_profile_minkowski_offset',{loops:a.loops,distanceMm,maximumSourceDeviationMm})
export const verifyBrepMinkowskiCertificate=(a:Pick<BrepProfile,'loops'>,b:Pick<BrepProfile,'loops'>,certificate:BrepMinkowskiCertificate):{status:'verified';scope:'represented-filled-minkowski-region-inclusions';exactSetResult:boolean;nominalExactRings:BrepExactRegionRings;innerExactRings:BrepExactRegionRings;outerExactRings:BrepExactRegionRings}=>callGeometryRust('brep_profile_verify_minkowski_certificate',{a:a.loops,b:b.loops,certificate})

export interface BrepBranchDecomposition {dimension:1|2;projection:BrepExactRatio[];fiberRelation:BrepExactRatio[][];cuts:{parameter:[BrepExactRatio,BrepExactRatio];fibers:[BrepExactRatio,BrepExactRatio][];vertical:boolean;verticalArcs:[number,number][];left:number[];right:number[];excludedNodes:boolean[]}[];slabs:{sample:BrepExactRatio;roots:[BrepExactRatio,BrepExactRatio][];diagonalSide:number[]}[]}
export interface BrepSpanIntersectionCertificate{
 version:number;scope:'represented-multispan-curve-intersection-components';source:string;selfPair:boolean;maximumParameterWidth:number
 pairs:{first:number;second:number;kind:'separate'|'stationary'|'events'|'range'|'overlap'|'locus'|'whole-square';axis:number;shear:number;factors:BrepAlgebraicIntersectionCertificate['factors'];relation:BrepExactRatio[][];branches?:BrepBranchDecomposition}[]
}
export interface BrepCurveOverlap{firstSpan:number;secondSpan:number;firstDomain:BrepParameterInterval;secondDomain:BrepParameterInterval;axis:number;imageRange:[BrepExactRatio,BrepExactRatio];relation:BrepExactRatio[][]}
export const certifyBrepSpanIntersections=(first:NurbsCurve,second:NurbsCurve,selfPair=false,maximumParameterWidth=1e-12):BrepSpanIntersectionCertificate=>callGeometryRust('brep_curve_certify_span_intersections',{first,second,selfPair,maximumParameterWidth})
export const verifyBrepSpanIntersections=(first:NurbsCurve,second:NurbsCurve,certificate:BrepSpanIntersectionCertificate):{status:'verified';scope:'represented-multispan-curve-intersection-components';events:[BrepParameterInterval,BrepParameterInterval][];singularParameters:BrepParameterInterval[];overlaps:BrepCurveOverlap[];algebraicLoci:{firstSpan:number;secondSpan:number;firstDomain:BrepParameterInterval;secondDomain:BrepParameterInterval;wholeSquare:boolean;diagonalExcluded:boolean;relation:BrepExactRatio[][]}[];stationarySpans:BrepParameterInterval[];branchDecompositions:{firstSpan:number;secondSpan:number;decomposition:BrepBranchDecomposition}[];finiteEventsExhaustive:boolean}=>callGeometryRust('brep_curve_verify_span_intersections_certificate',{first,second,certificate})

/** Equality of compact curve image sets. Traversal multiplicity and orientation
 * are separate from this bidirectional real-root coverage proof. */
export interface BrepCurveImageCertificate {
 version:number;scope:'represented-curve-image-set-agreement';source:string
 forward:BrepImageCoverage[];backward:BrepImageCoverage[]
}
export interface BrepImageCoverage {
 relations:{targetCell:number;relation:BrepExactRatio[][]}[];projection:BrepExactRatio[]
 roots:[BrepExactRatio,BrepExactRatio][];sections:number[]
}
export const certifyBrepCurveImageAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number]):BrepCurveImageCertificate=>callGeometryRust('brep_curve_certify_image_agreement',{first,second,firstDomain,secondDomain})
export const verifyBrepCurveImageAgreement=(first:NurbsCurve,second:NurbsCurve,firstDomain:readonly [number,number],secondDomain:readonly [number,number],certificate:BrepCurveImageCertificate):{status:'verified';scope:'represented-curve-image-set-agreement';sourceCells:[number,number];coverageSections:number}=>callGeometryRust('brep_curve_verify_image_agreement_certificate',{first,second,firstDomain,secondDomain,certificate})
