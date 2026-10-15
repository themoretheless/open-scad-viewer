import type {NurbsCurve} from '../../nurbsCurve'
import type {NurbsSurface} from '../../nurbsSurface'
import type {RationalReparameterization} from '../../nurbsFoundation'
import {callNurbsRust} from '../../geometry/nurbs'

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

