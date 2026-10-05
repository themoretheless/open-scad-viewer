import {type MiterProfileSmoothness} from '../certificates/miterProfileSmoothness'
import {type SweepBoundaryCertificate} from '../certificates/sweepBoundaryCertificate'
import {type SweepSectionCorrection} from './nurbsSectionProjection'
import {type SweepRetainedCaps,type SweepRetainedCorrespondence} from '../certificates/sweepRetainedCorrespondence'
import {type SweepEmbeddingAudit,type SweepVolumeAudit} from '../certificates/nurbsSweepEmbedding'
import {callGeometryRust} from '../../geometry/kernel'
import type {NurbsCurve} from '../../nurbsCurve'
import type {NurbsSurface} from '../../nurbsSurface'
import type {NurbsBrep} from '../../geometry/brep/core'
import {type ProgressiveMiterOptions,type ProgressiveMiterResult} from '../../nurbsConstructors'
import {type SweepProfileRegularityAudit,type SweepContourAudit} from '../certificates/nurbsSweepAudit'
import {type SweepRetainedChartEvidence} from '../certificates/nurbsSweepRetainedCharts'
import {type SweepCapPairEvidence,type SweepCapContactEvidence} from '../certificates/nurbsSweepCapContacts'
import type {NurbsScaleLaw,ProgressiveGuidedSurfaceSweepOptions,ProgressiveMultiSweepResult} from '../../nurbsConstructors'

/** Ordered rational loops per section, with audited planar cap trim regions.
 * Preserves manifold incidence; global embedding/self-intersections are unproven. */
export const createNaturalBrepSectionLoft=(sections:NurbsCurve[][][],parameters:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_natural_section_loft',{sections,parameters})
export const createCappedBrepLoftSurfaces=(start:NurbsCurve[][],end:NurbsCurve[][],sides:NurbsSurface[][]):NurbsBrep=>callGeometryRust('brep_nurbs_capped_loft_surfaces',{start,end,sides})
/** Retains authored nonlinear walls on every span; Solid requires a separate audit. */
export const createBrepSectionLoftSurfaces=(sections:NurbsCurve[][][],sides:NurbsSurface[][],closed=false):NurbsBrep=>callGeometryRust('brep_nurbs_section_loft_surfaces',{sections,sides,closed})
export interface SmoothStationWallCandidate {
 sides:NurbsSurface[][]|null
 wallDisplacementUpper:number|null
 work:number
 reason:string
}
/** Bounded candidate only: no regularity, embedding or Solid certificate. */
export const proposeSmoothStationWalls=(sections:NurbsCurve[][][],sharp:number[],closed:boolean,quantum:number,tolerance:number,maxWork:number):SmoothStationWallCandidate=>callGeometryRust('brep_nurbs_smooth_station_walls',{sections,sharp,closed,quantum,tolerance,maxWork})

export interface ProgressiveMiterBrepBody {profileSmoothness:MiterProfileSmoothness;boundaryCertificate:SweepBoundaryCertificate;retainedCapDecomposition:import('../certificates/sweepRetainedCorrespondence').SweepRetainedCapDecomposition|null;retainedDecomposition:import('../certificates/sweepRetainedCorrespondence').SweepRetainedDecomposition|null;capParallelism:import('../../nurbsConstructors').ProgressiveMiterCapParallelism|null;boundaryErrorWithinBudget:boolean|null;boundaryErrorUpper:number|null;filledCapErrorUpper:[number,number]|null;idealCapDomains:import('../../nurbsConstructors').ProgressiveMiterIdealCapDomains|null;capProjection:import('../../nurbsConstructors').ProgressiveMiterCapProjection|null;retainedWallErrorUpper:number|null;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;model:NurbsBrep;approximation:ProgressiveMiterResult;wallAudit:import('../certificates/nurbsSweepAudit').SweepWallAudit;retainedCorrespondence:SweepRetainedCorrespondence;retainedCaps:SweepRetainedCaps|null;retainedWallCharts:SweepRetainedChartEvidence;capDomains:[SweepContourAudit,SweepContourAudit]|null;capContacts:SweepCapContactEvidence[]|null;capPairs:SweepCapPairEvidence|null;embedding:SweepEmbeddingAudit|null;volume:SweepVolumeAudit;globalEmbeddingCertified:false}
// Bind source certificates to the exact constructor-owned geometry. Mutation,
// JSON copies or caller-authored evidence cannot transfer an old boundary bound.
export interface CertifiedMiterBoundaryOwner {model:NurbsBrep;boundaryCertificate:SweepBoundaryCertificate}

/** Reconstruct final corrected sections owned by the source constructor.
 * Original polyline vertices keep their independent one-sided jets. */
export function reconstructCertifiedMiterStations(source:CertifiedMiterBoundaryOwner,options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}):NativeSmoothedMiterBody {return callBodyRust('brep_reconstruct_certified_miter',{source,owner:nativeOwners.get(source)??null,options})}
export interface ExactAffineLatticePlacement {model:NurbsBrep|null;operatorNormUpper:number|null;arithmeticErrorUpper:number|null;work:number;reason:string}
export const placeNurbsBrepOnExactAffineLattice=(model:NurbsBrep,matrix:number[][],quantum:number,maxWork:number):ExactAffineLatticePlacement=>callGeometryRust('brep_nurbs_affine_lattice',{model,matrix,quantum,maxWork})
/** Exact placement scales the complete wall/cap Hausdorff bound; actual G2,
 * regularity, material nesting and orientation are audited on the new body. */
export function transformCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,matrix:number[][],options:{quantum:number;maxWork:number;maxDeviation:number}):NativeTransformedMiterBody {return callBodyRust('brep_transform_certified_miter',{source,owner:nativeOwners.get(source)??null,matrix,options})}
/** Compose a bounded wall reconstruction with constructor-owned full boundary
 * evidence. Endpoint sections/caps stay identical; every material audit is new. */
export function smoothCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,sections:NurbsCurve[][][],sharp:number[],options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}):NativeSmoothedMiterBody {return callBodyRust('brep_smooth_certified_miter',{source,owner:nativeOwners.get(source)??null,sections,sharp,options})}

export const createProgressiveMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions):ProgressiveMiterBrepBody=>callBodyRust('brep_progressive_miter_body',{loops,points,scale,twist,options})
export interface MiterBrepBody {model:NurbsBrep;report:{profileSmoothness:MiterProfileSmoothness;method:'polyline-miter-sections';sections:number;closedPath:boolean;globalEmbeddingCertified:false;roundingCertified:false;continuousBound:false;profileRegularityCertified:boolean;profileRegularity:SweepProfileRegularityAudit;wallRegularityCertified:boolean;retainedWallCharts:SweepRetainedChartEvidence;volume:SweepVolumeAudit;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;retainedCorrespondence?:SweepRetainedCorrespondence}}
export const createMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],normal:[number,number,number],miterLimit=4,closed=false,capCorrection?:{quantum:number;tolerance:number;maxWork:number}):MiterBrepBody=>callBodyRust('brep_miter_body',{loops,points,normal,miterLimit,closed,capCorrection})
export const createRationalBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_rational_section_loft',{sections})

export interface ProgressiveBrepBody {model:NurbsBrep;approximation:ProgressiveMultiSweepResult;globalEmbeddingCertified:false}
/** Open-path caps or closed-path periodic shells, constrained by the shared B-rep face budget. Twist values are degrees. */
export const createProgressiveBrepProfileBody=(loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions):ProgressiveBrepBody=>callBodyRust('brep_progressive_profile_body',{loops,path,scale,twist,options})

/** Closed contour shells with an identical repeated endpoint section; no caps. */
export const createPeriodicBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_periodic_section_loft',{sections})

/** Streams side-wall previews on the body face budget, then constructs audited
 * caps/seams. A preview level never contains an authoritative B-rep body.
 */
export async function* streamProgressiveBrepProfileBody(loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,control:import('../../nurbsConstructors').ProgressiveSweepStreamOptions={}):AsyncGenerator<import('../../nurbsConstructors').ProgressiveSweepPreview,ProgressiveBrepBody,void>{return yield* nativeBodyStream({kind:'profile',loops,path,scale,twist,options},control)}

/** Stream retained miter walls; build topology only after acceptance and a cancellation boundary. */
export async function* streamProgressiveMiterBrepProfileBody(loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions,control:import('../../nurbsConstructors').ProgressiveSweepStreamOptions={}):AsyncGenerator<import('../../nurbsConstructors').ProgressiveSweepPreview,ProgressiveMiterBrepBody,void>{return yield* nativeBodyStream({kind:'miter',loops,points,scale,twist,options},control)}

export interface NurbsLoftCap { surface:NurbsSurface; trims:NurbsCurve[][] }
export interface LoftEmbeddingLimits {
 exactWork:number; trimPairs:number; trimCells:number; trimDomainCells:number; spans:number
 facePairs:number; faceCells:number; faceDomainCells:number; faceCellsPerPair:number; faceDomainCellsPerPair:number
}
const defaultLoftEmbeddingLimits:LoftEmbeddingLimits={exactWork:1000000,trimPairs:10000,trimCells:100000,
 trimDomainCells:1000000,spans:10000,facePairs:10000,faceCells:1000000,faceDomainCells:1000000,
 faceCellsPerPair:10000,faceDomainCellsPerPair:10000}
/** Authored curved caps; refuses unless whole-boundary embedding is proven. */
export const createCappedBrepLoftWithCaps=(start:NurbsCurve[][],end:NurbsCurve[][],sides:NurbsSurface[][],
 caps:[NurbsLoftCap,NurbsLoftCap],embeddingLimits:LoftEmbeddingLimits=defaultLoftEmbeddingLimits,toleranceUv=1e-9):NurbsBrep=>
 callGeometryRust('brep_nurbs_capped_loft_with_caps',{start,end,sides,caps,embeddingLimits,toleranceUv})

/** Opaque native capabilities are transport metadata, never serialized evidence. */
const nativeOwners=new WeakMap<object,string>()
const ownerCleanup=new FinalizationRegistry<string>(owner=>{
 try{callGeometryRust('brep_sweep_release_owner',{owner})}catch{/* Kernel disposal also releases its native registry. */}
})
function captureBody<T>(result:T):T {
 if(result&&typeof result==='object'){
  const record=result as T & {_nativeOwner?:string}
  if(typeof record._nativeOwner==='string'){
   nativeOwners.set(record,record._nativeOwner)
   ownerCleanup.register(record,record._nativeOwner,record)
   delete record._nativeOwner
  }
 }
 return result
}
function releaseBody(result:unknown):void{
 if(!result||typeof result!=='object')return
 const owner=nativeOwners.get(result)
 if(owner){nativeOwners.delete(result);ownerCleanup.unregister(result);try{callGeometryRust('brep_sweep_release_owner',{owner})}catch{/* Preserve cancellation if the kernel has closed. */}}
}
function callBodyRust<T>(op:string,request:Record<string,unknown>):T{return captureBody(callGeometryRust<T>(op,request))}
async function* nativeBodyStream<P,B>(request:Record<string,unknown>,control:import('../../nurbsConstructors').ProgressiveSweepStreamOptions):AsyncGenerator<P,B,void>{
 const checkAbort=()=>{control.signal?.throwIfAborted();if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 checkAbort()
 const {stream}=callGeometryRust<{stream:string}>('brep_sweep_stream_start',request)
 try {
  for(;;){
   await new Promise<void>(resolve=>setTimeout(resolve,0));checkAbort()
   const next=callGeometryRust<{done:false;value:P}|{done:true;value:B}>('brep_sweep_stream_next',{stream})
   if(next.done){const body=captureBody(next.value);try{checkAbort()}catch(error){releaseBody(body);throw error}return body}
   checkAbort()
   yield next.value
  }
 }finally{callGeometryRust('brep_sweep_stream_release',{stream})}
}
export interface NativeTransformedMiterBody extends CertifiedMiterBoundaryOwner {
 placement:ExactAffineLatticePlacement;profileSmoothness:MiterProfileSmoothness;retainedWallCharts:SweepRetainedChartEvidence;volume:SweepVolumeAudit
 continuousBound:boolean;boundaryErrorUpper:number|null;boundaryErrorWithinBudget:boolean|null;budget:number;filledCapErrorUpper:[number,number]|null;retainedWallErrorUpper:number|null
 wallRegularityCertified:boolean;profileRegularityCertified:boolean;globalEmbeddingCertified:false
}
export interface NativeSmoothedMiterBody extends Omit<NativeTransformedMiterBody,'placement'> {
 method:'bounded-miter-station-reconstruction';candidate:SmoothStationWallCandidate;sharpStationIndices:number[]
 authoredFramesApplied?:boolean;orientationGuideApplied?:boolean;affineLawsApplied?:boolean
}
