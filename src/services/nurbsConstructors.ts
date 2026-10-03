/** Surface construction is implemented by the own Rust kernel. */
import { type NurbsCurve } from './nurbsCurve'
import { type NurbsSurface } from './nurbsSurface'
import type { RationalReparameterization } from './nurbsFoundation'
import { callNurbsRust } from './geometry/nurbs'
export function loftNurbsCurves(curves: NurbsCurve[]): NurbsSurface { return callNurbsRust('loft', { curves }) }
export function extrudeNurbsCurve(curve: NurbsCurve, vector: number[]): NurbsSurface { return callNurbsRust('extrude', { curve, vector }) }
export function revolveNurbsCurve(curve: NurbsCurve, origin: number[], axis: number[], angle: number): NurbsSurface { return callNurbsRust('revolve', { curve, origin, axis, angle }) }
/** Exact translation sweep; the profile frame does not rotate along the path. */
export const sweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve):NurbsSurface=>callNurbsRust('surface_sweep',{profile,path})
export const loftAlignedNurbsCurves=(curves:NurbsCurve[]):NurbsSurface=>callNurbsRust('loft_aligned',{curves})

import {validateSculptBrush,type GeometryDeformation,type GeometryBrush,type SculptBrush} from './geometryEditing'
/** Sculpts the control polygon/net; knots and weights are untouched. */
export const sculptNurbsCurve=(curve:NurbsCurve,brush:SculptBrush):NurbsCurve=>{validateSculptBrush(brush);return callNurbsRust('nurbs_sculpt_curve',{curve,brush})}
export const sculptNurbsSurface=(surface:NurbsSurface,brush:SculptBrush):NurbsSurface=>{validateSculptBrush(brush);return callNurbsRust('nurbs_sculpt_surface',{surface,brush})}
/** Nonlinear edits affect control points, not the exact pointwise surface image. */
export const deformNurbsCurve=(curve:NurbsCurve,deformation:GeometryDeformation):NurbsCurve=>callNurbsRust('nurbs_deform_curve',{curve,deformation})
export const deformNurbsSurface=(surface:NurbsSurface,deformation:GeometryDeformation):NurbsSurface=>callNurbsRust('nurbs_deform_surface',{surface,deformation})
export const brushNurbsCurve=(curve:NurbsCurve,brush:GeometryBrush):NurbsCurve=>callNurbsRust('nurbs_brush_curve',{curve,brush})
export const brushNurbsSurface=(surface:NurbsSurface,brush:GeometryBrush):NurbsSurface=>callNurbsRust('nurbs_brush_surface',{surface,brush})

/** Homogeneous Coons patch with compatible corner weights and positive control weights; boundaries: bottom, top, left, right. */
export const coonsNurbsPatch=(boundaries:NurbsCurve[]):NurbsSurface=>callNurbsRust('surface_coons_patch',{boundaries})

export interface FramedSweepResult {
 surface:NurbsSurface|null
 report:{accepted:boolean;sampledControlDeviation:number;budget:number;stations:number;sections:number;closedPath?:boolean;seamContinuity?:'C0'|'open';continuousBound:false;method:'double-reflection-fourfold-section-refinement'}
}
/** Sampled refinement diagnostic only; not a certified continuous error bound. */
export const framedSweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve,normal:[number,number,number],sections:number,maxDeviation:number):FramedSweepResult=>
 callNurbsRust('surface_framed_sweep',{profile,path,normal,sections,maxDeviation})

export type SurfaceJetBoundary='uMin'|'uMax'|'vMin'|'vMax'
export interface SurfaceSeamRegularity {
 certified:boolean
 unresolvedIntervals?:[number,number][]
 reason?:string
}
export interface SurfaceJetMatchResult {
 surface:NurbsSurface
 report:{
  continuityOrder:1|2
  normalScale:number
  method:'homogeneous-normalized-boundary-jets'
  regularityCertified:boolean
  referenceRegularity?:SurfaceSeamRegularity
  editedRegularity?:SurfaceSeamRegularity
  seamBasis:'exact-affine-knot-correspondence'
  modifiedLayers:number
  accepted:boolean
  maxError:number
  errorUpper:number
  tangentialSmoothnessCertified:boolean
  reason:'accepted'|'unproven-regularity'|'unproven-tangential-smoothness'|'deviation-exceeds-budget'
  errorBounds:{
   wholeSeam:true
   normalizedParameters:true
   method:'outward-homogeneous-jet-difference-hull'
   positionUpper:number
   firstDerivativeUpper:number
   secondDerivativeUpper:number|null
   mixedDerivativeUpper:number|null
  }
 }
}
/** Native whole-seam regularity and derivative-error gate; apply only accepted results. */
export const matchNurbsSurfaceJets=(reference:NurbsSurface,edited:NurbsSurface,referenceBoundary:SurfaceJetBoundary,editedBoundary:SurfaceJetBoundary,order:1|2,scale=1,reverse=false,maxError=1e-6):SurfaceJetMatchResult=>
 callNurbsRust('surface_match_jets',{reference,edited,referenceBoundary,editedBoundary,order,scale,reverse,maxError})

export interface SurfacePreparationResult {
 reference:NurbsSurface
 edited:NurbsSurface
 report:{accepted:boolean;reason:'accepted'|'deviation-exceeds-budget'|'unproven-parameter-normalization';budget:number;referenceErrorUpper?:number;editedErrorUpper?:number;wholeSurface?:true;method?:'outward-homogeneous-Bernstein-difference'}
 basis:{degree:number;controlCount:number;normalizedSeam:true;editedReversed:boolean;referenceDomain:[number,number];editedDomain:[number,number];periodicityRemoved:{reference:boolean;edited:boolean}}
}
export const prepareNurbsSurfaceSeams=(reference:NurbsSurface,edited:NurbsSurface,referenceBoundary:SurfaceJetBoundary,editedBoundary:SurfaceJetBoundary,reverse:boolean,maxError:number,openPeriodic=false):SurfacePreparationResult=>
 callNurbsRust('surface_prepare_seams',{reference,edited,referenceBoundary,editedBoundary,reverse,maxError,openPeriodic})

export interface CurveMatchResult {
 curve:NurbsCurve
 report:{accepted:boolean;positionErrorUpper:number;sineAngleUpper:number|null;angleDegreesUpper:number|null;maxAngleDegrees:number;regularityCertified:boolean;orientationCertified:boolean;method:'outward-endpoint-handle-wedge';reason:string}
}
export const matchNurbsCurveG1=(reference:NurbsCurve,edited:NurbsCurve,referenceEnd:'start'|'end',editedEnd:'start'|'end',maxAngleDegrees=1e-6):CurveMatchResult=>
 callNurbsRust('curve_match_g1',{reference,edited,referenceEnd,editedEnd,maxAngleDegrees})

export interface CoonsBoundaryPreparation {
 curve:NurbsCurve
 report:{accepted:boolean;errorUpper:number;budget:number;wholeCurve:true;method:'outward-homogeneous-Bernstein-difference'}
}
/** Preserve boundary parameters; return the source definition if the bound exceeds budget. */
export const prepareCoonsBoundaryWeights=(curve:NurbsCurve,maxError:number):CoonsBoundaryPreparation=>
 {const result=callNurbsRust<CoonsBoundaryPreparation>('curve_prepare_coons_weights',{curve,maxError});return result.report.accepted?result:{...result,curve:structuredClone(curve)}}

export const bezierNurbsCurve = (points: [number,number,number][], weights?: number[]): NurbsCurve => callNurbsRust('curve_bezier',{points,weights})
export const lineNurbsCurve = (start: [number,number,number], end: [number,number,number]): NurbsCurve => callNurbsRust('curve_line',{start,end})
export const circleNurbsCurve = (center: [number,number,number], normal: [number,number,number], radius: number): NurbsCurve => callNurbsRust('curve_circle',{center,normal,radius})
export const clampedLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[],startTangent:[number,number,number],endTangent:[number,number,number]):NurbsSurface=>callNurbsRust('surface_clamped_loft',{curves,parameters,start_tangent:startTangent,end_tangent:endTangent})
export const closedLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[]):NurbsSurface=>callNurbsRust('surface_closed_loft',{curves,parameters})
export const naturalLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[],sectionMappings?:Array<RationalReparameterization|null>):NurbsSurface=>callNurbsRust('surface_natural_loft',{curves,parameters,section_mappings:sectionMappings})
export const controlTangentLoftNurbsCurves = (curves: NurbsCurve[], parameters: number[], startTangents: [number,number,number][], endTangents: [number,number,number][]): NurbsSurface => callNurbsRust('surface_control_tangent_loft', {curves, parameters, start_tangents:startTangents, end_tangents:endTangents})
export const guidedLoftNurbsCurves = (curves: NurbsCurve[], parameters: number[], guides: NurbsCurve[], guideParameters: number[], startTangents?: [number,number,number][], endTangents?: [number,number,number][]): NurbsSurface => callNurbsRust('surface_guided_loft', {curves, parameters, guides, guide_parameters:guideParameters, start_tangents:startTangents, end_tangents:endTangents})

export interface AlignedNurbsLoft {
 surface:NurbsSurface
 guides:NurbsCurve[]
 guide_parameters:number[]
 guide_order:number[]
 reversed:boolean[]
 section_error_upper:number[]
 guide_error_upper:number[]
 sections?:NurbsCurve[]
 section_mapping_certificates?:unknown[]
 original_section_certificates?:Array<Record<string,unknown>>
}
/** Automatic isolated intersections, guide reversal/sorting and piecewise V mapping. */
export const autoGuidedLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[],guides:NurbsCurve[],budget:number,parameterTolerance=1e-8,sectionMappings?:Array<RationalReparameterization|null>):AlignedNurbsLoft=>callNurbsRust('surface_auto_guided_loft',{curves,parameters,guides,budget,parameter_tolerance:parameterTolerance,section_mappings:sectionMappings})
export interface LoftEndConstraint {
 reference:NurbsSurface
 boundary:'uMin'|'uMax'|'vMin'|'vMax'
 order:1|2
 scale:number
 reverse?:boolean
}
export interface MatchedNurbsLoft {
 surface:NurbsSurface
 seams:unknown[]
 section_error_upper:number[]
 guide_error_upper:number[]
}
/** Certified scaled boundary jets with whole-curve section/guide retention bounds. */
export const matchNurbsLoftEnds=(surface:NurbsSurface,curves:NurbsCurve[],parameters:number[],budget:number,start?:LoftEndConstraint,end?:LoftEndConstraint,guides:NurbsCurve[]=[],guideParameters:number[]=[]):MatchedNurbsLoft=>callNurbsRust('surface_loft_match_ends',{surface,curves,parameters,budget,start,end,guides,guide_parameters:guideParameters})
