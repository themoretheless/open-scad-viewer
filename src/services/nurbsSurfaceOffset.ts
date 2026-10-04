import type {NurbsSurface} from './nurbsSurface'
import {callNurbsRust} from './geometry/nurbs'

type Interval=[number,number]
type Point3=[number,number,number]
export type SurfaceDomain=[Interval,Interval]
export type OffsetPairDomain=[SurfaceDomain,SurfaceDomain]
export interface OffsetEvaluation {
 method:'normal-offset-numerical-jets';point:Point3;du:Point3;dv:Point3;sourceUnitNormal:Point3
 certified:false;topologyAuthority:false
}
interface OffsetBoundsBase {
 image:[Interval,Interval,Interval]|null;normalSpanVisits:number;reason:string
 offsetRegularityCertified:false;continuityCertified:false;topologyAuthority:false
}
export interface OffsetBounds extends OffsetBoundsBase {
 method:'interval-source-normal-offset';scope:'incident-span-offset-images'
 unitNormals:[Interval,Interval,Interval]|null
}
export interface OffsetJacobianBounds extends OffsetBoundsBase {
 method:'interval-source-normal-offset-jets';scope:'incident-span-offset-jets'
 derivatives:[[Interval,Interval,Interval],[Interval,Interval,Interval]]|null
}
export interface OffsetContactOptions {
 a:NurbsSurface;b:NurbsSurface;distances:[number,number];fixedAxis:0|1;fixed:number
 firstOther:Interval;secondDomain:SurfaceDomain;maxSpans:number
}
export interface OffsetContactSection {
 method:'interval-offset-section-krawczyk';scope:'fixed-parameter-offset-section'
 status:'unique-contact'|'excluded'|'unresolved'
 witness:{firstUV:SurfaceDomain;secondUV:SurfaceDomain;centerIntervalMm:[Interval,Interval,Interval];contractionUpper:number}|null
 rootExistenceProven:boolean;uniqueInSection:boolean
 wholeCurveComplete:false;trimMembershipProven:false;topologyAuthority:false
}
export interface OffsetCandidateOptions {
 a:NurbsSurface;b:NurbsSurface;domains:OffsetPairDomain;distances:[number,number]
 parameterTolerance:number;maxBoxes:number;maxSpans:number
}
export interface OffsetCandidates {
 method:'interval-offset-pair-exclusion';scope:'untrimmed-offset-carriers'
 candidateBoxes:OffsetPairDomain[];pendingBoxes:OffsetPairDomain[]
 visitedBoxes:number;excludedBoxes:number;normalSpanVisits:number;reason:string
 rootExistenceProven:false;wholeCurveComplete:false;trimMembershipProven:false;topologyAuthority:false
}
/** Numerical proposal only; interval inclusion is a separate query. */
export const evaluateNurbsSurfaceOffset=(surface:NurbsSurface,parameters:[number,number],distance:number):OffsetEvaluation=>
 callNurbsRust('surface_offset_evaluate',{surface,parameters,distance})
/** Encloses incident span sides; does not prove a unique normal across a knot. */
export const boundNurbsSurfaceOffset=(surface:NurbsSurface,domain:SurfaceDomain,distance:number,maxSpans:number):OffsetBounds=>
 callNurbsRust('surface_offset_bounds',{surface,domain,distance,maxSpans})
export const boundNurbsSurfaceOffsetJacobian=(surface:NurbsSurface,domain:SurfaceDomain,distance:number,maxSpans:number):OffsetJacobianBounds=>
 callNurbsRust('surface_offset_jacobian_bounds',{surface,domain,distance,maxSpans})
/** A section center is not a complete center curve or an admitted fillet. */
export const certifyNurbsOffsetContactSection=(options:OffsetContactOptions):OffsetContactSection=>
 callNurbsRust('surface_offset_contact_section',options)
export const findNurbsOffsetCandidateBoxes=(options:OffsetCandidateOptions):OffsetCandidates=>
 callNurbsRust('surface_offset_candidates',options)
