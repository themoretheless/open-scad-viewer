import type {NurbsSurface} from '../../nurbsSurface'
import type {NurbsCurve} from '../../nurbsCurve'
import type {NurbsScaleLaw,ProgressiveGuidedSurfaceSweepOptions} from '../../nurbsConstructors'
import {callNurbsRust} from '../../geometry/nurbs'

const progressiveSweepSourcePayload=(path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions)=>
 callNurbsRust<Record<string,unknown>>('brep_sweep_law_payload',{kind:'source',path,scale,twist,options})

export interface ProgressiveRetainedStationSeams {
 requestedOrder:1|2
 allStationSeamsCertified:boolean
 exactWork:number
 seams:{patch:number;station:number;closure:boolean;c0Identity:boolean;certified:boolean;regularityCertified:boolean;exactWork:number;reason:string}[]
 reason:string|null
 method:'exact-retained-station-strip-jets'
 scope:'retained-station-seams-only'
 sourceFrameSmoothnessCertified:false
 profileJoinsCertified:false
 capJoinsCertified:false
 solidCertified:false
}
/** Native full-domain retained seam audit; source-frame and Solid proofs remain separate. */
export const inspectProgressiveRetainedStationSeams=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,order:1|2,maxExactWork:number):ProgressiveRetainedStationSeams=>
 callNurbsRust('surface_progressive_sweep_station_seams',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,order,maxExactWork})
export interface ProgressiveRetainedProfileJoin {
 requestedOrder:1|2
 leftPatch:number
 rightPatch:number
 profilePatchRanges:[number,number][]
 certified:boolean
 exactIdentity:boolean
 regularityCertified:boolean
 exactWork:number
 reason:string
 method:'exact-retained-profile-strip-jets'
 scope:'explicit-retained-profile-join-only'
 allProfileJoinsCertified:false
 sourceFrameSmoothnessCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Rust reconstructs the requested level and checks this explicit uMax/uMin pair. */
export const inspectProgressiveRetainedProfileJoin=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,leftPatch:number,rightPatch:number,order:1|2,transverseScale:number,maxExactWork:number):ProgressiveRetainedProfileJoin=>
 callNurbsRust('surface_progressive_sweep_profile_join',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,leftPatch,rightPatch,order,transverseScale,maxExactWork})
export interface ProgressiveRetainedDecompositionJoins {
 requestedOrder:1|2
 expectedJoins:number
 checkedJoins:number
 coverageComplete:boolean
 decompositionJoinsCertified:boolean
 exactWork:number
 joins:{leftPatch:number;rightPatch:number;certified:boolean;exactIdentity:boolean;regularityCertified:boolean;exactWork:number;reason:string}[]
 reason:string|null
 profilePatchRanges:[number,number][]
 method:'exact-retained-decomposition-strip-jets'
 scope:'within-source-profile-decomposition-only'
 allProfileJoinsCertified:false
 closedProfileSeamsCertified:false
 sourceFrameSmoothnessCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Native ownership extraction and all internal decomposition joins share one budget. */
export const inspectProgressiveRetainedDecompositionJoins=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,order:1|2,transverseScale:number,maxExactWork:number):ProgressiveRetainedDecompositionJoins=>
 callNurbsRust('surface_progressive_sweep_decomposition_joins',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,order,transverseScale,maxExactWork})
export interface ProgressiveRetainedDecompositionSmoothness {
 g2:Omit<ProgressiveRetainedDecompositionJoins,'profilePatchRanges'>
 g1:Omit<ProgressiveRetainedDecompositionJoins,'profilePatchRanges'>|null
 decompositionG1Certified:boolean
 exactWork:number
 maxExactWork:number
 profilePatchRanges:[number,number][]
 method:'exact-retained-decomposition-smoothness'
 scope:'within-source-profile-decomposition-only'
 allProfileJoinsCertified:false
 closedProfileSeamsCertified:false
 sourceFrameSmoothnessCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Native G2 with budget reserved for an independently complete G1 fallback. */
export const inspectProgressiveRetainedDecompositionSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,transverseScale:number,maxExactWork:number):ProgressiveRetainedDecompositionSmoothness=>
 callNurbsRust('surface_progressive_sweep_decomposition_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,transverseScale,maxExactWork})
export interface ProgressiveOriginalFrameSmoothness {
 requestedOrder:1|2
 sourceFrameSmoothnessCertified:boolean
 cells:number
 exactWork:number
 reason:string|null
 method:'original-frame-continuity-and-nondegeneracy-cover'
 scope:'open-original-frame-only'
 retainedSeamsCertified:false
 profileJoinsCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Original frame proof is computed exclusively by Rust, independently of retained surface seams. */
export const inspectProgressiveOriginalFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveOriginalFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveClosedAuthoredFrameSmoothness {
 requestedOrder:1|2
 closedSourceFrameSmoothnessCertified:boolean
 cells:number
 exactWork:number
 reason:string|null
 method:'exact-original-authored-endpoint-jets-and-frame-cover'
 scope:'closed-original-authored-frame-only'
 pathSeamCertified:false
 retainedSeamsCertified:false
 profileJoinsCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Rust original closed-frame proof; it is independent of the path and retained surface seam. */
export const inspectProgressiveClosedAuthoredFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveClosedAuthoredFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_closed_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveClosedPathFrameSmoothness extends Omit<ProgressiveClosedAuthoredFrameSmoothness,'method'|'scope'> {
 method:'exact-original-path-twist-endpoint-jets-and-frame-cover'
 scope:'closed-original-path-frame-only'
}
/** Rust proof of the closed original path frame; retained joins remain separate. */
export const inspectProgressiveClosedPathFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveClosedPathFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_closed_path_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveClosedGuidedFrameSmoothness extends Omit<ProgressiveClosedAuthoredFrameSmoothness,'method'|'scope'> {
 method:'exact-original-guided-endpoint-jets-and-joint-frame-cover'
 scope:'closed-original-guided-frame-only'
}
/** Rust proof of the closed original guided frame; retained joins remain separate. */
export const inspectProgressiveClosedGuidedFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveClosedGuidedFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_closed_guided_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveSweepIdealCapDomains {
 idealCapDomainsCertified:boolean
 localDomainCertified:boolean
 sourcePlaneAxis:number|null
 endpointFrameAxes:[[[number,number],[number,number],[number,number]],[[number,number],[number,number],[number,number]]]|null
 cells:number;pairs:number;exactWork:number;reason:string|null
 method:'original-progressive-endpoint-material-domains'
 continuousBound:false;retainedCapRegionsCertified:false;globalEmbeddingCertified:false;solidCertified:false
}
/** Original material domains only. All geometry/proofs are evaluated in Rust. */
export function inspectProgressiveSweepIdealCapDomains(
 profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,loopSizes:number[],
 budgets={tolerance:.001,maxPairs:1000,maxCells:10000,maxExactWork:1000000},
):ProgressiveSweepIdealCapDomains {
 return callNurbsRust('surface_progressive_sweep_cap_domains',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),loopSizes,...budgets})
}
export interface ProgressiveSweepCapProjection {
 capProjectionCertified:boolean;idealCapDomainsCertified:boolean
 endpointNormals:ProgressiveSweepIdealCapDomains['endpointFrameAxes']
 normalDots:[[number,number],[number,number]]|null;reversesOrientation:[boolean,boolean]|null
 cells:number;exactWork:number;reason:string|null
 method:'original-progressive-endpoint-plane-projection'
 continuousBound:false;retainedCapRegionsCertified:false;globalEmbeddingCertified:false;solidCertified:false
}
/** Plane projection only; no cap ownership or boundary error promotion. */
export function inspectProgressiveSweepCapProjection(
 profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,loopSizes:number[],caps:[NurbsSurface,NurbsSurface],
 budgets={tolerance:.001,maxPairs:1000,maxCells:10000,maxExactWork:1000000},
):ProgressiveSweepCapProjection {
 return callNurbsRust('surface_progressive_sweep_cap_projection',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),loopSizes,caps,...budgets})
}
