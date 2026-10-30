import type {ExactAffineLatticePlacement} from './brep'
// Original-request native candidates and their explicit transport ownership.
import type {NurbsBrep} from '../../geometry/brep/core'
import {constructMiterOwnedBoundary,constructProgressiveMiterOwnedBoundary,constructUnbudgetedDirectMiterBoundary,progressiveMiterOwnedRequest,reconstructMiterOwnedBoundary,type MiterOwnedBoundaryReport,type DirectMiterOwnedSourceRequest,type UnbudgetedAnnularMiterBoundaryReport} from '../../nurbsMiterOwnedBoundary'

import {inspectMiterProfileSmoothness,type MiterProfileSmoothness} from '../../miterProfileSmoothness'

import {type SweepBoundaryCertificate} from '../../sweepBoundaryCertificate'

import {planMiterBody,ownedMiterSharpStations,previewMiterWalls} from '../../miterBodyLayout'

import {correctMiterSweepSections,type SweepSectionCorrection} from '../../nurbsSectionProjection'

import {inspectSweepRetainedCorrespondence,inspectSweepRetainedDecomposition,inspectSweepRetainedCaps,inspectSweepRetainedCapDecomposition,type SweepRetainedCaps,type SweepRetainedCorrespondence} from '../../sweepRetainedCorrespondence'

import {inspectSweepEmbedding,DEFAULT_SWEEP_EMBEDDING_BUDGETS,type SweepEmbeddingAudit,inspectSweepVolume,inspectSweepVolumeAndCharts,inspectSweepVolumeAndChartsAsync,DEFAULT_SWEEP_VOLUME_BUDGETS,type SweepVolumeAudit} from '../../nurbsSweepEmbedding'

import type {DirectSketch} from '../../directModeling'

import {callGeometryRust} from '../../geometry/kernel'

import type {NurbsCurve} from '../../nurbsCurve'

import type {NurbsSurface} from '../../nurbsSurface'

import {normalizePolygonMesh} from '../../geometry/polygon'

import type {PolygonMesh,PolygonBuild} from '../../geometry/polygon'

import type {RustChangeSet,TopoId} from '../../../core/topologyLineage'

/** Affine edit of authored carriers. Reflections also reverse shell face uses. */
// These are native-derived optimization selections, never positive proof
// premises. Native affine transforms retain face indices; receiving workers
// still audit the actual transformed geometry and every face/pair.
import {nativeSweepPlanarCapSelections,readNativeSweepPlanarCapFaces} from './nativeCapSelections'
export {readNativeSweepPlanarCapFaces} from './nativeCapSelections'

import {miterNurbsProfileSections} from '../../nurbsConstructors'

import {inspectProgressiveMiterIdealCapDomains,inspectProgressiveMiterCapProjection,inspectProgressiveMiterCapParallelism,inspectProgressiveMiterWalls,streamProgressiveMiterNurbsProfiles,type ProgressiveMiterOptions,type ProgressiveMiterResult} from '../../nurbsConstructors'

import {inspectSweepContours,inspectSweepProfileRegularity,type SweepProfileRegularityAudit,type SweepContourAudit} from '../../nurbsSweepAudit'

import {inspectSweepRetainedWallCharts,type SweepRetainedChartEvidence} from '../../nurbsSweepRetainedCharts'

import {inspectSweepCapContacts,inspectSweepCapPairs,type SweepCapPairEvidence,type SweepCapContactEvidence} from '../../nurbsSweepCapContacts'

// Route original requests and station metadata by object identity. Rust replays
// and compares actual geometry/certificates before deriving modifier bounds.
const sweepCapFaces=(model:NurbsBrep,closed:boolean)=>closed?[]:[model.faces.length-2,model.faces.length-1]

/** Copy native receipt fields without recomputing or promoting any proof. */
const boundaryReceiptFields=(certificate:SweepBoundaryCertificate)=>({
 continuousBound:certificate.continuousBound,boundaryErrorUpper:certificate.errorUpper,
 boundaryErrorWithinBudget:certificate.withinBudget,budget:certificate.budget,
 filledCapErrorUpper:certificate.filledCapErrorUpper,retainedWallErrorUpper:certificate.wallErrorUpper,
})

const chartReceiptFields=(charts:SweepRetainedChartEvidence)=>({wallRegularityCertified:charts.allChartsCertified,profileRegularityCertified:charts.allChartsCertified})

export interface CertifiedMiterBoundaryOwner {model:NurbsBrep;boundaryCertificate:SweepBoundaryCertificate}

const miterOriginalRequests=new WeakMap<CertifiedMiterBoundaryOwner,string>()

const miterAffineHistory=new WeakMap<CertifiedMiterBoundaryOwner,string>()

const miterReconstructionReplay=new WeakMap<CertifiedMiterBoundaryOwner,string>()

const miterProofOwners=new WeakMap<CertifiedMiterBoundaryOwner,{sections?:string;sharp?:number[];authoring?:{authoredFramesApplied:boolean;orientationGuideApplied:boolean;affineLawsApplied:boolean}}>()

/** Explicit original direct request; all geometry and certificate audits are native. */
export function createDirectMiterOwnedBody(source:DirectMiterOwnedSourceRequest) {
 const owned=constructMiterOwnedBoundary(source),model=owned.model
 const capFaces=sweepCapFaces(model,source.closed)
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const profileRegularity=inspectSweepProfileRegularity(source.loops.flat(),10000)
 const volume=inspectSweepVolume(model,capFaces,DEFAULT_SWEEP_VOLUME_BUDGETS)
 const certificate=owned.boundaryCertificate
 const report={method:certificate.method,retentionMode:owned.retentionMode,
  boundaryCertificate:certificate,...boundaryReceiptFields(certificate),
  profileSmoothness,profileRegularity,profileRegularityCertified:profileRegularity.spanwiseRegular,
  retainedWallCharts:owned.retainedWallCharts,wallRegularityCertified:owned.retainedWallCharts.allChartsCertified,
  sectionCorrection:owned.sectionCorrection,volume,globalEmbeddingCertified:false as const,
  closedPath:source.closed,levels:owned.levels}
 const body={...owned,profileSmoothness,volume,report}
 miterOriginalRequests.set(body,JSON.stringify(source))
 miterProofOwners.set(body,{sections:JSON.stringify(owned.sections),sharp:owned.sharpStationIndices})
 return body
}

/** Replayable inputs only; no positive geometry certificate is transported. */
export function readMiterAffineReplayRouting(owner:CertifiedMiterBoundaryOwner) {
 const original=miterOriginalRequests.get(owner),encoded=miterAffineHistory.get(owner)
 if(!original)return null
 const reconstruction=miterReconstructionReplay.get(owner)
 if(!encoded&&!reconstruction)return null
 // Unplaced reconstructions still need original-request replay in a new
 // worker. Identity placement uses the same native final-model binding gate.
 const steps=encoded?JSON.parse(encoded):[{
  matrix:[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],
  quantum:JSON.parse(reconstruction!).quantum,maxWork:JSON.parse(reconstruction!).maxWork,budget:owner.boundaryCertificate.budget,
 }]
 const [first,...following]=steps as {matrix:number[][];quantum:number;maxWork:number;budget:number|null}[]
 if(!first)return null
 return {source:JSON.parse(original),...first,followingPlacements:following,
  ...(miterReconstructionReplay.has(owner)?{beforeReconstruction:JSON.parse(miterReconstructionReplay.get(owner)!)}:{})}
}

export interface MiterBrepBody {model:NurbsBrep;report:{profileSmoothness:MiterProfileSmoothness;method:'polyline-miter-sections';sections:number;closedPath:boolean;globalEmbeddingCertified:false;roundingCertified:false;continuousBound:boolean;profileRegularityCertified:boolean;profileRegularity:SweepProfileRegularityAudit;wallRegularityCertified:boolean;retainedWallCharts:SweepRetainedChartEvidence;volume:SweepVolumeAudit;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;retainedCorrespondence?:SweepRetainedCorrespondence}}

export interface NativeDirectMiterGeometry extends MiterBrepBody {
 report:MiterBrepBody['report'] & {
  sweepMiterReplay:{sourceKind:'direct-annular';source:import('../../nurbsMiterOwnedBoundary').UnbudgetedDirectMiterSourceRequest}
  budget:null;boundaryErrorWithinBudget:null
  boundaryErrorUpper:number|null;filledCapErrorUpper:[number,number]|null
  boundaryCertificate:UnbudgetedAnnularMiterBoundaryReport['boundaryCertificate']
  capRepresentation:UnbudgetedAnnularMiterBoundaryReport['capRepresentation']
  capFaceGroups:[number[],number[]];planarCapFaces:number[]
  retainedAnnularFilling:UnbudgetedAnnularMiterBoundaryReport['retainedAnnularFilling']
  retainedSmoothness:UnbudgetedAnnularMiterBoundaryReport['retainedSmoothness']
 }
}

/** Prepared transport for the matching native unbudgeted API. Geometry,
 * correction, cap selection and the complete numerical bound are Rust-owned.
 * The live Rush route must switch only with a matching qualified WASM. */
export const createNativeDirectMiterGeometry=(loops:NurbsCurve[][],points:[number,number,number][],normal:[number,number,number],miterLimit=4,closed=false,capCorrection?:{quantum:number;tolerance:number;maxWork:number}):NativeDirectMiterGeometry=>{
 const source={direct:true as const,loops,points,normal,closed,miterLimit,capCorrection}
 const owned=constructUnbudgetedDirectMiterBoundary(source,{smoothnessMaxWork:2000000})
 if(owned.retainedSmoothness===null)throw new Error('Native direct miter retained smoothness report missing')
 const model=owned.model,retainedWallCharts=owned.retainedWallCharts
 const volume=inspectSweepVolume(model,owned.planarCapFaces,DEFAULT_SWEEP_VOLUME_BUDGETS)
 const profileRegularity=inspectSweepProfileRegularity(loops.flat(),10000)
 nativeSweepPlanarCapSelections.set(model,owned.planarCapFaces.slice())
 return {model,report:{sweepMiterReplay:{sourceKind:'direct-annular',source:structuredClone(source)},method:'polyline-miter-sections',sections:owned.sections.length,closedPath:closed,
  profileSmoothness:owned.retainedSmoothness.walls,globalEmbeddingCertified:false,roundingCertified:false,
  continuousBound:owned.boundaryCertificate.continuousBound,profileRegularityCertified:profileRegularity.spanwiseRegular,
  profileRegularity,wallRegularityCertified:retainedWallCharts.allChartsCertified,retainedWallCharts,volume,
  ...(owned.sectionCorrection?{sectionCorrection:owned.sectionCorrection}:{}),budget:null,boundaryErrorWithinBudget:null,
  boundaryErrorUpper:owned.boundaryCertificate.errorUpper,filledCapErrorUpper:owned.boundaryCertificate.filledCapErrorUpper,
  boundaryCertificate:owned.boundaryCertificate,capRepresentation:owned.capRepresentation,capFaceGroups:owned.capFaceGroups,
  planarCapFaces:owned.planarCapFaces,retainedAnnularFilling:owned.retainedAnnularFilling,retainedSmoothness:owned.retainedSmoothness}}
}

import type {NurbsScaleLaw,ProgressiveGuidedSurfaceSweepOptions,ProgressiveMultiSweepResult} from '../../nurbsConstructors'

export type ProgressiveBrepSweepOptions=ProgressiveGuidedSurfaceSweepOptions & {capCorrection?:{quantum:number;tolerance:number;maxWork:number};circleCorrection?:{quantum:number;tolerance:number;maxWork:number};
 /** Explicit native radial caps; requires a kernel with this operation. */
 annularCaps?:{automatic?:boolean;maxWork?:number;maxProducts?:number;maxPairs?:number;maxCells?:number;smoothnessMaxWork?:number;geometricMaxCells?:number}}

export interface ProgressiveAnnularFilling {
 method:'authored-smooth-endpoint-annular-filling';sourceFillingCovered:boolean;filledCapErrorUpper:[number,number]|null
 withinBudget:boolean;exactWork:number;constructionExactWork:number;fillingExactWork:number;selectionExactWork?:number
 products:number;endpointCells:number;sourceUvFacesInjective:boolean;sourceUvPairsClassified:boolean
 reason:string;solidGeometryCertified:false
}

export interface ProgressiveBrepBody {model:NurbsBrep;approximation:ProgressiveMultiSweepResult;globalEmbeddingCertified:false;
 capRepresentation?:'radial-annular'|'closed'|'legacy';capSelectionExactWork?:number;capFaceGroups?:[number[],number[]];capFaces?:number[];planarCapFaces?:number[]
 retainedAnnularFilling?:ProgressiveAnnularFilling|null
 retainedSmoothness?:(Omit<NonNullable<UnbudgetedAnnularMiterBoundaryReport['retainedSmoothness']>,'capWallContinuity'> & {capWallContinuity:'C0'|'absent'})|null
 bodyDecompositionErrorUpper?:number|null;bodyDecompositionProducts?:number
 capProjection?:{idealCapDomainsCertified:boolean;normalDots:[[number,number],[number,number]]|null;reversesOrientation:[boolean,boolean]|null;
  cells:number;exactWork:number;reason:string|null;scope:'constructor-owned-endpoint-plane-projection'}|null
 capCorrectionErrorUpper?:number|null
 profileCorrectionErrorUpper?:number|null
 profileSmoothness?:MiterProfileSmoothness
 filledCapErrorUpper?:[number,number]|null;boundaryContinuousBound?:boolean;boundaryErrorUpper?:number|null
 boundaryErrorWithinBudget?:boolean|null
 boundaryErrorScope?:'constructor-owned-retained-wall-and-cap-union'
 retainedWalls?:{certified:boolean;faceCoverageCertified:boolean;coefficientFamilyCertified:boolean;inspectedFaces:number;exactWork:number;reason:string}
 retainedCaps?:{exact:boolean;capErrorUpper:0|null;exactWork:number;inspectedEdges:number;reason:string;
  scope:'constructor-owned-retained-endpoint-regions';continuousBound:false;globalEmbeddingCertified:false}|null}

const progressiveBodyRequests=new WeakMap<ProgressiveBrepBody,string>()

const progressiveBodyOperations=new WeakMap<ProgressiveBrepBody,string>()

const progressiveAnnularSources=new WeakSet<ProgressiveBrepBody>()

type ProgressiveCapOwnership=Pick<ProgressiveBrepBody,'capRepresentation'|'capSelectionExactWork'|'capFaceGroups'|'capFaces'|'planarCapFaces'|'retainedAnnularFilling'>

// Validate transport ownership, never infer cap geometry from face positions.
function copyProgressiveCapOwnership(result:ProgressiveCapOwnership,faces:number,closed:boolean):ProgressiveCapOwnership {
 const {capRepresentation,capFaceGroups,capFaces,planarCapFaces}=result
 if(!['radial-annular','legacy','closed'].includes(capRepresentation??'')||
  !Array.isArray(capFaceGroups)||capFaceGroups.length!==2||!capFaceGroups.every(Array.isArray)||
  !Array.isArray(capFaces)||!Array.isArray(planarCapFaces))throw new Error('Native physical cap ownership is unavailable')
 const flat=capFaceGroups.flat()
 if(flat.length!==capFaces.length||flat.some((face,index)=>face!==capFaces[index])||
  new Set(flat).size!==flat.length||flat.some(face=>!Number.isInteger(face)||face<0||face>=faces)||
  new Set(planarCapFaces).size!==planarCapFaces.length||planarCapFaces.some(face=>!flat.includes(face))||
  (closed?(capRepresentation!=='closed'||flat.length!==0||planarCapFaces.length!==0)
   :capRepresentation==='closed'||capFaceGroups.some(group=>group.length===0))||
  (capRepresentation==='radial-annular'&&planarCapFaces.length!==0)||
  (capRepresentation==='legacy'&&(capFaceGroups.some(group=>group.length!==1)||planarCapFaces.length!==2)))
  throw new Error('Invalid native physical cap ownership')
 return {capRepresentation,capSelectionExactWork:result.capSelectionExactWork,
  capFaceGroups:capFaceGroups.map(group=>group.slice()) as [number[],number[]],capFaces:capFaces.slice(),planarCapFaces:planarCapFaces.slice(),
  retainedAnnularFilling:result.retainedAnnularFilling}
}

/** Replay the owned original request in Rust; audit the actual reconstructed surfaces. */
function prepareCertifiedProgressiveStations(source:ProgressiveBrepBody,options:{quantum:number;wallTolerance:number;maxWork:number;smoothnessMaxWork:number}) {
 const encoded=progressiveBodyRequests.get(source)
 if(!encoded)throw new Error('Missing owned progressive station source')
 const annular=progressiveAnnularSources.has(source)
 const result=callGeometryRust<ProgressiveCapOwnership & {stationReconstruction:{reason:string;source:string;globalEmbeddingCertified:false;profileSmoothness:MiterProfileSmoothness|null;reconstruction:{model:NurbsBrep|null;wallDisplacementUpper:number|null;work:number;reason:string}|null;boundaryCertificate:SweepBoundaryCertificate|null}}>((progressiveBodyOperations.get(source)??'brep_nurbs_progressive_profile_body')+'_reconstruct_stations',{
  ...JSON.parse(encoded),expected_source_model:source.model,station_quantum:options.quantum,station_tolerance:options.wallTolerance,station_max_work:options.maxWork,smoothness_max_work:options.smoothnessMaxWork,
 })
 const native=result.stationReconstruction,model=native.reconstruction?.model,certificate=native.boundaryCertificate
 if(!model||!certificate||!native.profileSmoothness)throw new Error('Native station reconstruction unproved: '+native.reason)
 if(!certificate.continuousBound||certificate.withinBudget!==true)throw new Error('Station boundary unproved or over original budget')
 const capOwnership:ProgressiveCapOwnership=annular?copyProgressiveCapOwnership(result,model.faces.length,certificate.closed):{}
 if(capOwnership.planarCapFaces)nativeSweepPlanarCapSelections.set(model,capOwnership.planarCapFaces.slice())
 return {model,...capOwnership,method:certificate.method,boundaryCertificate:certificate,...boundaryReceiptFields(certificate),
  profileSmoothness:native.profileSmoothness,globalEmbeddingCertified:false as const,
  stationReconstruction:{...capOwnership,source:native.source,closedPath:certificate.closed,work:native.reconstruction!.work,wallDisplacementUpper:native.reconstruction!.wallDisplacementUpper},
 }
}

export function reconstructCertifiedProgressiveStations(...args:Parameters<typeof prepareCertifiedProgressiveStations>) {
 const prepared=prepareCertifiedProgressiveStations(...args)
 const {retainedWallCharts,...volume}=inspectSweepVolumeAndCharts(prepared.model,readNativeSweepPlanarCapFaces(prepared.model)??sweepCapFaces(prepared.model,prepared.boundaryCertificate.closed),DEFAULT_SWEEP_VOLUME_BUDGETS)
 return {...prepared,retainedWallCharts,...chartReceiptFields(retainedWallCharts),volume}
}

export async function reconstructCertifiedProgressiveStationsAsync(source:ProgressiveBrepBody,
 options:Parameters<typeof prepareCertifiedProgressiveStations>[1],control:{shouldAbort?:()=>boolean;onYield?:()=>void}={}) {
 const prepared=prepareCertifiedProgressiveStations(source,options)
 const {retainedWallCharts,...volume}=await inspectSweepVolumeAndChartsAsync(prepared.model,readNativeSweepPlanarCapFaces(prepared.model)??sweepCapFaces(prepared.model,prepared.boundaryCertificate.closed),DEFAULT_SWEEP_VOLUME_BUDGETS,control)
 return {...prepared,retainedWallCharts,...chartReceiptFields(retainedWallCharts),volume}
}

/** Open-path caps or closed-path periodic shells, constrained by the shared B-rep face budget. Twist values are degrees. */
export const createProgressiveBrepProfileBody=(loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveBrepSweepOptions):ProgressiveBrepBody=>{
 const annular=options.annularCaps
 const request={loops,path,...(annular?{annular_cap_max_work:annular.maxWork??1000000,annular_cap_max_products:annular.maxProducts??100000,annular_cap_max_pairs:annular.maxPairs??10000,annular_cap_max_cells:annular.maxCells??100000,smoothness_max_work:annular.smoothnessMaxWork??2000000,geometric_g2_max_cells:annular.geometricMaxCells??(annular.smoothnessMaxWork===0?0:128)}:{}),...(options.circleCorrection?{circle_correction_quantum:options.circleCorrection.quantum,circle_correction_tolerance:options.circleCorrection.tolerance,circle_correction_max_work:options.circleCorrection.maxWork}:{}),...(options.capCorrection?{cap_correction_quantum:options.capCorrection.quantum,cap_correction_tolerance:options.capCorrection.tolerance,cap_correction_max_work:options.capCorrection.maxWork}:{}),...sweepAffineLawPayload(options),...sweepFrameLawPayload(options),...sweepGuidePayload(options),
  scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false},
  twist:{degree:twist.degree,knots:twist.knots,controlPoints:twist.values.map(a=>[a*Math.PI/180,0,0]),weights:twist.weights,periodic:false},
  ...(options.rmfTransportSteps===undefined?{}:{rmf_transport_steps:options.rmfTransportSteps}),...(options.errorMaxCells===undefined?{}:{error_max_cells:options.errorMaxCells}),...(options.errorMaxProducts===undefined?{}:{error_max_products:options.errorMaxProducts}),normal:options.normal,orientation:options.orientation??'rmf',spacing:options.spacing??'parameter',initial_sections:options.initialSections??5,max_sections:options.maxSections??257,max_deviation:options.maxDeviation,length_tolerance:options.lengthTolerance??0.001,length_max_cells:options.lengthMaxCells??100000}
 const encoded=JSON.stringify(request)
 const operation=annular?(annular.automatic?'brep_nurbs_progressive_profile_auto_body':'brep_nurbs_progressive_profile_annular_body'):'brep_nurbs_progressive_profile_body'
 const result=callGeometryRust<ProgressiveBrepBody>(operation,JSON.parse(encoded))
 if(annular){
  if(!Array.isArray(result.planarCapFaces)||!result.retainedSmoothness)throw new Error('Native annular body evidence is unavailable')
  progressiveAnnularSources.add(result)
  nativeSweepPlanarCapSelections.set(result.model,result.planarCapFaces.slice())
  result.profileSmoothness=result.retainedSmoothness.walls
 }
 progressiveBodyRequests.set(result,encoded)
 progressiveBodyOperations.set(result,operation)
 if(options.circleCorrection&&(typeof result.profileCorrectionErrorUpper!=='number'||!Number.isFinite(result.profileCorrectionErrorUpper)||result.profileCorrectionErrorUpper<0))throw new Error('Native progressive profile circle correction is unavailable or unproved')
 if(options.circleCorrection&&!annular)result.profileSmoothness=inspectMiterProfileSmoothness(result.model,result.approximation.report.closedPath?[]:[result.model.faces.length-2,result.model.faces.length-1])
 return result
}


import {sweepAffineLawPayload,sweepFrameLawPayload,sweepGuidePayload} from '../../nurbsConstructors'



/** Streams side-wall previews on the body face budget, then constructs audited
 * caps/seams. A preview level never contains an authoritative B-rep body.
 */
export async function* streamProgressiveBrepProfileBody(
 loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveBrepSweepOptions,control:import('../../nurbsConstructors').ProgressiveSweepStreamOptions={},
):AsyncGenerator<import('../../nurbsConstructors').ProgressiveSweepPreview,ProgressiveBrepBody,void>{
 const checkAbort=()=>{
  control.signal?.throwIfAborted()
  if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')
 }
 checkAbort()
 // Retain the same original request through all post-preview certificates.
 ;({loops,path,scale,twist,options}=structuredClone({loops,path,scale,twist,options}))
 if(!loops.length||loops.length>16||loops.some(loop=>!loop.length)||(options.maxSections??257)>1025)
  throw new Error('Progressive body needs 1..16 nonempty loops and at most1025 sections')
 const profiles=loops.flat()
 const spans=profiles.reduce((count,curve)=>count+decomposeNurbsCurve(curve).length,0)
 if(!spans||spans>64)throw new Error('Progressive body exceeds64 section spans')
 const first=previewProgressiveNurbsProfiles(profiles,path,scale,twist,options,options.initialSections??5)
 // Mirror native face reservation for previews; Rust validates final topology.
 const maximum=Math.min(options.maxSections??257,Math.floor((1024-(first.report.closedPath?0:options.annularCaps&&!options.annularCaps.automatic?spans:2))/spans)+1)
 const bounded={...options,maxSections:maximum}
 if((options.initialSections??5)>maximum)throw new Error('Progressive body initial sections exceed face budget')
 const stream=streamProgressiveNurbsProfiles(profiles,path,scale,twist,bounded,control)
 try{
  for(;;){
   const level=await stream.next()
   checkAbort()
   if(level.done){
    if(!level.value.report.accepted)throw new Error(options.rmfTransportSteps===undefined
     ?'Progressive body sampled refinement exceeds budget'
     :'Progressive body continuous retained-patch error is unproved or exceeds budget')
    // Let a queued worker cancel run before final topology/cap construction.
    await new Promise<void>(resolve=>setTimeout(resolve,0))
    checkAbort()
    const body=createProgressiveBrepProfileBody(loops,path,scale,twist,bounded)
    checkAbort()
    return body
   }
   yield level.value
  }
 }finally{await stream.return(undefined as never)}
}

import {decomposeNurbsCurve} from '../../nurbsCurve'

import {previewProgressiveNurbsProfiles,streamProgressiveNurbsProfiles} from '../../nurbsConstructors'
export function transformOriginalMiterBody(source:CertifiedMiterBoundaryOwner,matrix:number[][],options:{quantum:number;maxWork:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner)throw new Error('Affine placement requires constructor-owned boundary evidence')
 const original=miterOriginalRequests.get(source)
 if(!original)throw new Error('Affine placement requires constructor-owned original request')
 const history=JSON.parse(miterAffineHistory.get(source)??'[]') as {matrix:number[][];quantum:number;maxWork:number;budget:number|null}[]
 const steps=[...history,{matrix,quantum:options.quantum,maxWork:options.maxWork,budget:Number.isFinite(options.maxDeviation)?options.maxDeviation:null}]
 const result=callGeometryRust<{placement:ExactAffineLatticePlacement|null;boundaryCertificate:SweepBoundaryCertificate|null;reason:string;retainedWallCharts:SweepRetainedChartEvidence|null;volume:SweepVolumeAudit|null}>('brep_miter_owned_place',{
  source:JSON.parse(original),...steps[0],followingPlacements:steps.slice(1),
  expectedSourceModel:source.model,expectedSourceCertificate:source.boundaryCertificate,
  ...(miterReconstructionReplay.has(source)?{beforeReconstruction:JSON.parse(miterReconstructionReplay.get(source)!)}:{}),
  requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS,
 })
 const {placement,boundaryCertificate}=result
 if(result.reason==='boundary-budget-unproved')throw new Error('Affine complete boundary error exceeds max_deviation or is unproved')
 if(!placement?.model||!boundaryCertificate)throw new Error(`Affine placement unproved: ${result.reason}`)
 const closed=boundaryCertificate.closed
 const model=placement.model,capFaces=sweepCapFaces(model,closed)
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const {retainedWallCharts,volume}=result
 if(!retainedWallCharts||!volume)throw new Error('Native affine material report missing')
 const transformed={globalEmbeddingCertified:false as const,model,placement,boundaryCertificate,profileSmoothness,retainedWallCharts,volume,...boundaryReceiptFields(boundaryCertificate),...chartReceiptFields(retainedWallCharts)}
 miterProofOwners.set(transformed,{})
 miterOriginalRequests.set(transformed,original)
 miterAffineHistory.set(transformed,JSON.stringify(steps))
 if(miterReconstructionReplay.has(source))miterReconstructionReplay.set(transformed,miterReconstructionReplay.get(source)!)
 return transformed
}

export function smoothOriginalMiterBody(source:CertifiedMiterBoundaryOwner,sections:NurbsCurve[][][],sharp:number[],options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner)throw new Error('Station smoothing requires constructor-owned boundary evidence')
 const original=miterOriginalRequests.get(source)
 if(!original)throw new Error('Station smoothing requires constructor-owned original request')
 const reconstruction=reconstructMiterOwnedBoundary(JSON.parse(original),{
  quantum:options.quantum,tolerance:options.wallTolerance,maxWork:options.maxWork,budget:options.maxDeviation,
  requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS,
  expectedSourceModel:source.model,expectedSourceCertificate:source.boundaryCertificate,expectedSections:sections,expectedSharp:sharp,
 })
 const {model,candidate,boundaryCertificate}=reconstruction
 if(reconstruction.reason==='boundary-budget-unproved')throw new Error('Smoothed complete boundary error exceeds max_deviation or is unproved')
 if(!model||!candidate?.sides||candidate.wallDisplacementUpper===null||!boundaryCertificate)throw new Error(`Station smoothing candidate unproved: ${reconstruction.reason}`)
 const capFaces=sweepCapFaces(model,boundaryCertificate.closed)
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const {retainedWallCharts,volume}=reconstruction
 if(!retainedWallCharts||!volume)throw new Error('Native reconstruction material report missing')
 const smoothed={...owner.authoring,method:'bounded-miter-station-reconstruction' as const,model,candidate,sharpStationIndices:[...sharp],boundaryCertificate,profileSmoothness,retainedWallCharts,volume,
  ...boundaryReceiptFields(boundaryCertificate),...chartReceiptFields(retainedWallCharts),globalEmbeddingCertified:false as const}
 miterProofOwners.set(smoothed,{})
 miterOriginalRequests.set(smoothed,original)
 miterReconstructionReplay.set(smoothed,JSON.stringify({quantum:options.quantum,tolerance:options.wallTolerance,maxWork:options.maxWork,budget:options.maxDeviation}))
 return smoothed
}

export function reconstructOriginalMiterStations(source:CertifiedMiterBoundaryOwner,options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner?.sections||!owner.sharp)throw new Error('Station reconstruction requires constructor-owned retained sections')
 return smoothOriginalMiterBody(source,JSON.parse(owner.sections) as NurbsCurve[][][],owner.sharp,options)
}

export const hasOriginalMiterBody=(body:CertifiedMiterBoundaryOwner)=>miterOriginalRequests.has(body)
export function rememberProgressiveSource(body:ProgressiveBrepBody,request:unknown,operation:string) {
 progressiveBodyRequests.set(body,JSON.stringify(request))
 progressiveBodyOperations.set(body,operation)
 if(body.planarCapFaces)nativeSweepPlanarCapSelections.set(body.model,body.planarCapFaces.slice())
 if(body.capRepresentation){
  progressiveAnnularSources.add(body)
  if(body.retainedSmoothness)body.profileSmoothness=body.retainedSmoothness.walls
 }
}
