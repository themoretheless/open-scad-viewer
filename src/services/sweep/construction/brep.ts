import {inspectMiterProfileSmoothness,type MiterProfileSmoothness} from '../certificates/miterProfileSmoothness'
import {composeSweepBoundaryCertificate,type SweepBoundaryCertificate} from '../certificates/sweepBoundaryCertificate'
import {certifiedSweepBoundaryErrorUpper,filledMiterCapErrorUpper} from '../certificates/nurbsFilledCapError'
import {addCertifiedErrorUpper,multiplyCertifiedErrorUpper} from '../certificates/nurbsErrorComposition'
import {type SweepSectionCorrection} from './nurbsSectionProjection'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedDecomposition,inspectSweepRetainedCaps,inspectSweepRetainedCapDecomposition,type SweepRetainedCaps,type SweepRetainedCorrespondence} from '../certificates/sweepRetainedCorrespondence'
import {inspectSweepEmbedding,DEFAULT_SWEEP_EMBEDDING_BUDGETS,type SweepEmbeddingAudit,inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS,type SweepVolumeAudit} from '../certificates/nurbsSweepEmbedding'
import {callGeometryRust} from '../../geometry/kernel'
import type {NurbsCurve} from '../../nurbsCurve'
import type {NurbsSurface} from '../../nurbsSurface'
import type {NurbsBrep} from '../../geometry/brep/core'
import {miterNurbsProfileSections} from '../../nurbsConstructors'
import {inspectProgressiveMiterIdealCapDomains,inspectProgressiveMiterCapProjection,inspectProgressiveMiterCapParallelism,inspectProgressiveMiterWalls,streamProgressiveMiterNurbsProfiles,progressiveMiterNurbsProfiles,type ProgressiveMiterOptions,type ProgressiveMiterResult} from '../../nurbsConstructors'
import {inspectSweepContours,inspectSweepProfileRegularity,type SweepProfileRegularityAudit,type SweepContourAudit} from '../certificates/nurbsSweepAudit'
import {inspectSweepRetainedWallCharts,type SweepRetainedChartEvidence} from '../certificates/nurbsSweepRetainedCharts'
import {inspectSweepCapContacts,inspectSweepCapPairs,type SweepCapPairEvidence,type SweepCapContactEvidence} from '../certificates/nurbsSweepCapContacts'
import {correctMiterSections,correctMiterCaps} from './correction'
import type {NurbsScaleLaw,ProgressiveGuidedSurfaceSweepOptions,ProgressiveMultiSweepResult} from '../../nurbsConstructors'
import {sweepAffineLawPayload,sweepFrameLawPayload,sweepGuidePayload} from '../../nurbsConstructors'
import {decomposeNurbsCurve} from '../../nurbsCurve'
import {previewProgressiveNurbsProfiles,streamProgressiveNurbsProfiles} from '../../nurbsConstructors'

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
const miterProofOwners=new WeakMap<CertifiedMiterBoundaryOwner,{model:string;certificate:string;sections?:string;sharp?:number[];authoring?:{authoredFramesApplied:boolean;orientationGuideApplied:boolean;affineLawsApplied:boolean}}>()
const ownMiterProof=(body:ProgressiveMiterBrepBody,sections:NurbsCurve[][][],edges:number):ProgressiveMiterBrepBody=>{
 const steps=body.approximation.report.steps
 if(sections.length!==edges*steps+1)throw new Error('Miter station ownership requires complete uniform span coverage')
 const sharp=Array.from({length:body.boundaryCertificate.closed?edges:Math.max(0,edges-1)},(_,i)=>(i+(body.boundaryCertificate.closed?0:1))*steps)
 const {authoredFramesApplied,orientationGuideApplied,affineLawsApplied}=body.approximation.report
 miterProofOwners.set(body,{model:JSON.stringify(body.model),certificate:JSON.stringify(body.boundaryCertificate),sections:JSON.stringify(sections),sharp,authoring:{authoredFramesApplied,orientationGuideApplied,affineLawsApplied}})
 return body
}
/** Reconstruct final corrected sections owned by the source constructor.
 * Original polyline vertices keep their independent one-sided jets. */
export function reconstructCertifiedMiterStations(source:CertifiedMiterBoundaryOwner,options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner?.sections||!owner.sharp)throw new Error('Station reconstruction requires constructor-owned retained sections')
 return smoothCertifiedMiterBody(source,JSON.parse(owner.sections) as NurbsCurve[][][],owner.sharp,options)
}
export interface ExactAffineLatticePlacement {model:NurbsBrep|null;operatorNormUpper:number|null;arithmeticErrorUpper:number|null;work:number;reason:string}
export const placeNurbsBrepOnExactAffineLattice=(model:NurbsBrep,matrix:number[][],quantum:number,maxWork:number):ExactAffineLatticePlacement=>callGeometryRust('brep_nurbs_affine_lattice',{model,matrix,quantum,maxWork})
/** Exact placement scales the complete wall/cap Hausdorff bound; actual G2,
 * regularity, material nesting and orientation are audited on the new body. */
export function transformCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,matrix:number[][],options:{quantum:number;maxWork:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner||owner.model!==JSON.stringify(source.model)||owner.certificate!==JSON.stringify(source.boundaryCertificate))throw new Error('Affine placement requires unchanged constructor-owned boundary evidence')
 if(!source.boundaryCertificate.continuousBound||!source.boundaryCertificate.withinBudget)throw new Error('Affine source complete boundary bound unproved')
 const placement=placeNurbsBrepOnExactAffineLattice(source.model,matrix,options.quantum,options.maxWork)
 if(!placement.model||placement.operatorNormUpper===null||placement.arithmeticErrorUpper!==0)throw new Error(`Exact affine placement unproved: ${placement.reason}`)
 const norm=placement.operatorNormUpper
 const scale=(upper:number|null)=>upper===null?null:multiplyCertifiedErrorUpper(norm,upper)
 const caps=source.boundaryCertificate.filledCapErrorUpper?.map(scale)
 const closed=source.boundaryCertificate.closed
 const boundaryCertificate=composeSweepBoundaryCertificate(scale(source.boundaryCertificate.wallErrorUpper),caps&&caps.every((x):x is number=>x!==null)?caps as [number,number]:null,closed,options.maxDeviation)
 if(!boundaryCertificate.continuousBound||!boundaryCertificate.withinBudget)throw new Error('Affine complete boundary error exceeds max_deviation or is unproved')
 const model=placement.model,capFaces=closed?[]:[model.faces.length-2,model.faces.length-1]
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,capFaces,100000)
 const volume=inspectSweepVolume(model,capFaces,DEFAULT_SWEEP_VOLUME_BUDGETS)
 if(!retainedWallCharts.allChartsCertified||!volume.solidGeometryCertified)throw new Error('Affine transformed Solid geometry unproved')
 const transformed={model,placement,boundaryCertificate,profileSmoothness,retainedWallCharts,volume,continuousBound:boundaryCertificate.continuousBound,boundaryErrorUpper:boundaryCertificate.errorUpper,boundaryErrorWithinBudget:boundaryCertificate.withinBudget,budget:boundaryCertificate.budget,filledCapErrorUpper:boundaryCertificate.filledCapErrorUpper,retainedWallErrorUpper:boundaryCertificate.wallErrorUpper,wallRegularityCertified:retainedWallCharts.allChartsCertified,profileRegularityCertified:retainedWallCharts.allChartsCertified}
 miterProofOwners.set(transformed,{model:JSON.stringify(model),certificate:JSON.stringify(boundaryCertificate)})
 return transformed
}
/** Compose a bounded wall reconstruction with constructor-owned full boundary
 * evidence. Endpoint sections/caps stay identical; every material audit is new. */
export function smoothCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,sections:NurbsCurve[][][],sharp:number[],options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner||owner.model!==JSON.stringify(source.model)||owner.certificate!==JSON.stringify(source.boundaryCertificate))throw new Error('Station smoothing requires unchanged constructor-owned boundary evidence')
 const certificate=source.boundaryCertificate
 if(!certificate.continuousBound||!certificate.withinBudget||certificate.wallErrorUpper===null)throw new Error('Station smoothing source complete boundary bound unproved')
 const identity=callGeometryRust<{geometryAndTopologyIdentical:boolean}>('brep_nurbs_section_loft_source_audit',{model:source.model,sections,closed:certificate.closed})
 if(!identity.geometryAndTopologyIdentical)throw new Error('Station smoothing sections do not reproduce the certified source body')
 const baseline=source.model
 const candidate=proposeSmoothStationWalls(sections,sharp,certificate.closed,options.quantum,options.wallTolerance,options.maxWork)
 if(!candidate.sides||candidate.wallDisplacementUpper===null)throw new Error(`Station smoothing candidate unproved: ${candidate.reason}`)
 const wallUpper=addCertifiedErrorUpper(certificate.wallErrorUpper,candidate.wallDisplacementUpper)
 const boundaryCertificate=composeSweepBoundaryCertificate(wallUpper,certificate.filledCapErrorUpper,certificate.closed,options.maxDeviation)
 if(!boundaryCertificate.continuousBound||!boundaryCertificate.withinBudget)throw new Error('Smoothed complete boundary error exceeds max_deviation or is unproved')
 const model=createBrepSectionLoftSurfaces(sections,candidate.sides,certificate.closed)
 const capFaces=certificate.closed?[]:[model.faces.length-2,model.faces.length-1]
 for(const face of capFaces) {
  if(JSON.stringify(model.faces[face])!==JSON.stringify(baseline.faces[face]))throw new Error('Station smoothing changed a filled cap')
  for(const loop of [model.faces[face]!.outer,...model.faces[face]!.holes]) {
   if(JSON.stringify(model.loops[loop])!==JSON.stringify(baseline.loops[loop]))throw new Error('Station smoothing changed cap trims')
   for(const use of model.loops[loop]!.coedges) {
    const edge=model.edges[use.edge]!
    if(JSON.stringify(edge)!==JSON.stringify(baseline.edges[use.edge]))throw new Error('Station smoothing changed a cap boundary edge')
    for(const vertex of edge.vertices)if(JSON.stringify(model.vertices[vertex])!==JSON.stringify(baseline.vertices[vertex]))throw new Error('Station smoothing changed a cap boundary vertex')
   }
  }
 }
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,capFaces,100000)
 const volume=inspectSweepVolume(model,capFaces,DEFAULT_SWEEP_VOLUME_BUDGETS)
 if(!retainedWallCharts.allChartsCertified||!volume.solidGeometryCertified)throw new Error('Smoothed Solid geometry unproved')
 const smoothed={...owner.authoring,method:'bounded-miter-station-reconstruction' as const,model,candidate,sharpStationIndices:[...sharp],boundaryCertificate,profileSmoothness,retainedWallCharts,volume,
  continuousBound:boundaryCertificate.continuousBound,boundaryErrorUpper:boundaryCertificate.errorUpper,
  boundaryErrorWithinBudget:boundaryCertificate.withinBudget,filledCapErrorUpper:boundaryCertificate.filledCapErrorUpper,
  retainedWallErrorUpper:boundaryCertificate.wallErrorUpper,budget:boundaryCertificate.budget,
  wallRegularityCertified:retainedWallCharts.allChartsCertified,profileRegularityCertified:retainedWallCharts.allChartsCertified,globalEmbeddingCertified:false as const}
 miterProofOwners.set(smoothed,{model:JSON.stringify(model),certificate:JSON.stringify(boundaryCertificate)})
 return smoothed
}
const auditMiterCapDomains=(sections:NurbsCurve[][][],options:ProgressiveMiterOptions,checkAbort=()=>{}):[SweepContourAudit,SweepContourAudit]|null=>{
 if(options.closed)return null
 const budgets=options.contourAuditBudgets??{tolerance:.001,maxPairs:1000,maxCells:1000}
 checkAbort();const start=inspectSweepContours(sections[0]!,budgets)
 checkAbort();const end=inspectSweepContours(sections.at(-1)!,budgets)
 checkAbort();return [start,end]
}
const defaultMiterWallAuditBudgets={clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000}

export const createProgressiveMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions):ProgressiveMiterBrepBody=>{
 if(loops.length<1||loops.length>16||loops.some(loop=>!loop.length))throw new Error('Miter body needs 1..16 nonempty loops')
 const profiles=loops.flat(),edges=points.length-(options.closed?0:1)
 const spans=profiles.reduce((count,curve)=>count+decomposeNurbsCurve(curve).length,0)
 if(edges<1||spans<1||spans>64)throw new Error('Progressive miter body exceeds its site/span budget')
 const maximum=Math.min(options.maxSteps??64,Math.floor(1024/edges),Math.floor((1024-(options.closed?0:2))/(edges*spans)))
 if(maximum<(options.initialSteps??1))throw new Error('Progressive miter initial steps exceed face budget')
 const approximation=progressiveMiterNurbsProfiles(profiles,points,scale,twist,{...options,maxSteps:maximum})
 if(approximation.report.frameTransportCertified===false)throw new Error('Progressive miter frame transport could not be proved; review the path, normal and miter limit')
    if(approximation.report.certifiedErrorUpper===null)throw new Error(`Progressive miter wall error bound could not be proved: ${approximation.report.errorCertificateReason??'unresolved certificate'}`)
    if(approximation.report.profileRegularityCertified===false)throw new Error('Progressive miter profile tangent regularity could not be proved')
    if(approximation.report.wallRegularityCertified===false)throw new Error('Progressive miter retained wall Jacobian regularity could not be proved')
 if(!approximation.report.accepted||!approximation.sections)throw new Error('Progressive miter refinement/phase budget not met within the body face budget')
 const correction=correctMiterSections(approximation.sections,points,options)
 const retainedSections=correction?.sections??approximation.sections
 const sectionCorrection=correction?(({sections,...evidence})=>evidence)(correction):undefined
 const wallAudit=inspectProgressiveMiterWalls(profiles,points,scale,twist,{...options,maxSteps:maximum},retainedSections,options.wallAuditBudgets??defaultMiterWallAuditBudgets,loops.map(loop=>loop.length))
 const sections=retainedSections.map(section=>{let offset=0;return loops.map(loop=>{const wire=section.slice(offset,offset+loop.length);offset+=loop.length;return wire})})
 const capDomains=auditMiterCapDomains(sections,options)
 const model=options.closed?createPeriodicBrepSectionLoft(sections):createRationalBrepSectionLoft(sections)
 const retainedCorrespondence=inspectSweepRetainedCorrespondence(model,sections,options.closed??false)
 const retainedDecomposition=retainedCorrespondence.exact?null:inspectSweepRetainedDecomposition(model,sections,options.closed??false,options.retainedDecompositionBudgets?.maxProducts??100000,options.retainedDecompositionBudgets?.maxFaces??1024)
 const decompositionError=retainedCorrespondence.exact?0:retainedDecomposition?.wallErrorUpper??null
 if(sectionCorrection&&decompositionError===null)throw new Error('Corrected progressive miter retained wall correspondence unproved')
 const baseWallError=addCertifiedErrorUpper(approximation.report.certifiedErrorUpper!,sectionCorrection?.wallDisplacementUpper??0)
 const retainedWallErrorUpper=baseWallError===null||decompositionError===null?null:addCertifiedErrorUpper(baseWallError,decompositionError)
 if(sectionCorrection&&(retainedWallErrorUpper===null||retainedWallErrorUpper>options.maxDeviation))throw new Error('Corrected progressive miter retained wall error exceeds max_deviation or is unproved')
 const retainedCaps=options.closed?null:inspectSweepRetainedCaps(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets)
 const retainedCapDecomposition=options.closed||retainedCaps?.exact?null:inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets,options.retainedDecompositionBudgets?.maxProducts??100000)
 if(sectionCorrection&&!options.closed&&!retainedCaps?.exact&&!retainedCapDecomposition?.certified)throw new Error('Corrected progressive miter filled retained cap regions unproved')
 const idealCapDomains=options.closed?null:inspectProgressiveMiterIdealCapDomains(profiles,points,scale,twist,options,loops.map(loop=>loop.length),options.capDomainBudgets)
 const capProjection=options.closed?null:inspectProgressiveMiterCapProjection(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const capParallelism=options.closed?null:inspectProgressiveMiterCapParallelism(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const filledCapErrorUpper=options.closed?null:filledMiterCapErrorUpper({parallelPlanesCertified:capParallelism?.parallel??null,idealCapDomainsCertified:idealCapDomains?.idealCapDomainsCertified===true,retainedCapRegionsExact:retainedCaps?.exact===true||retainedCapDecomposition?.certified===true,decompositionErrorUpper:retainedCaps?.exact?[0,0]:retainedCapDecomposition?.capErrorUpper??null,projectionNormalDots:capProjection?.normalDots??null,endpointContourErrorUpper:approximation.report.endpointContourErrorUpper,correctionDisplacementUpper:sectionCorrection?sectionCorrection.wallDisplacementUpper:0})
 const boundaryErrorUpper=certifiedSweepBoundaryErrorUpper(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false)
 const boundaryErrorWithinBudget=boundaryErrorUpper===null?null:boundaryErrorUpper<=options.maxDeviation
 const boundaryCertificate=composeSweepBoundaryCertificate(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false,options.maxDeviation)
 if(boundaryCertificate.withinBudget===false)throw new Error('Progressive miter complete boundary error exceeds max_deviation')
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.retainedWallMaxInjectivityCells??1000)
 if(sectionCorrection&&!retainedWallCharts.allChartsCertified)throw new Error('Corrected progressive miter retained wall regularity unproved')
 const capPairs=options.closed?null:inspectSweepCapPairs(model,[model.faces.length-2,model.faces.length-1],options.capPairAuditBudgets??{clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000})
 const capContacts=options.closed?null:inspectSweepCapContacts(model,[model.faces.length-2,model.faces.length-1],options.capWallMaxWalls??1024)
 const embedding=options.closed?null:inspectSweepEmbedding(model,[model.faces.length-2,model.faces.length-1],options.embeddingBudgets??DEFAULT_SWEEP_EMBEDDING_BUDGETS)
 const volume=inspectSweepVolume(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS)
 return ownMiterProof({model,approximation,profileSmoothness:inspectMiterProfileSmoothness(model,options.closed?[]:[model.faces.length-2,model.faces.length-1]),boundaryCertificate,retainedCapDecomposition,retainedDecomposition,capParallelism,boundaryErrorWithinBudget,boundaryErrorUpper,filledCapErrorUpper,idealCapDomains,capProjection,sectionCorrection,retainedWallErrorUpper,wallAudit,retainedCorrespondence,retainedCaps,retainedWallCharts,capDomains,capContacts,capPairs,embedding,volume,globalEmbeddingCertified:false},sections,edges)
}
export interface MiterBrepBody {model:NurbsBrep;report:{profileSmoothness:MiterProfileSmoothness;method:'polyline-miter-sections';sections:number;closedPath:boolean;globalEmbeddingCertified:false;roundingCertified:false;continuousBound:false;profileRegularityCertified:boolean;profileRegularity:SweepProfileRegularityAudit;wallRegularityCertified:boolean;retainedWallCharts:SweepRetainedChartEvidence;volume:SweepVolumeAudit;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;retainedCorrespondence?:SweepRetainedCorrespondence}}
export const createMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],normal:[number,number,number],miterLimit=4,closed=false,capCorrection?:{quantum:number;tolerance:number;maxWork:number}):MiterBrepBody=>{
 if(loops.length<1||loops.length>16||loops.some(loop=>loop.length===0))throw new Error('Miter body needs 1..16 nonempty loops')
 let sections=miterNurbsProfileSections(loops.flat(),points,normal,miterLimit,closed)
 let sectionCorrection:Omit<SweepSectionCorrection,'sections'>|undefined
 if(capCorrection){
  if(closed)throw new Error('Closed miter has no caps to correct')
  const result=correctMiterCaps(sections,points,closed,capCorrection)
  const {sections:corrected,...evidence}=result
  sections=corrected;sectionCorrection=evidence
 }
 const nested=sections.map(section=>{let offset=0;return loops.map(loop=>{const wire=section.slice(offset,offset+loop.length);offset+=loop.length;return wire})})
 const model=closed?createPeriodicBrepSectionLoft(nested):createRationalBrepSectionLoft(nested)
 const retainedCorrespondence=sectionCorrection?inspectSweepRetainedCorrespondence(model,nested,closed):undefined
 if(sectionCorrection&&!retainedCorrespondence?.exact)throw new Error('Corrected miter retained wall correspondence unproved')
 const volume=inspectSweepVolume(model,closed?[]:[model.faces.length-2,model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS)
 const profileRegularity=inspectSweepProfileRegularity(loops.flat(),10000)
 // A positive projected symmetric Jacobian has rank two everywhere on each
 // actual retained chart, and therefore proves wall regularity independently
 // of cap contacts, material orientation and authored approximation error.
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,closed?[]:[model.faces.length-2,model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS.maxLinearCells)
 return {model,report:{profileSmoothness:inspectMiterProfileSmoothness(model,closed?[]:[model.faces.length-2,model.faces.length-1]),method:'polyline-miter-sections',sections:sections.length,closedPath:closed,globalEmbeddingCertified:false,roundingCertified:false,continuousBound:false,profileRegularityCertified:profileRegularity.spanwiseRegular,profileRegularity,wallRegularityCertified:retainedWallCharts.allChartsCertified,retainedWallCharts,volume,...(sectionCorrection?{sectionCorrection,retainedCorrespondence}:{})}}
}
export const createRationalBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_rational_section_loft',{sections})

export interface ProgressiveBrepBody {model:NurbsBrep;approximation:ProgressiveMultiSweepResult;globalEmbeddingCertified:false}
/** Open-path caps or closed-path periodic shells, constrained by the shared B-rep face budget. Twist values are degrees. */
export const createProgressiveBrepProfileBody=(loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions):ProgressiveBrepBody=>
 callGeometryRust('brep_nurbs_progressive_profile_body',{loops,path,...sweepAffineLawPayload(options),...sweepFrameLawPayload(options),...sweepGuidePayload(options),
  scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false},
  twist:{degree:twist.degree,knots:twist.knots,controlPoints:twist.values.map(a=>[a*Math.PI/180,0,0]),weights:twist.weights,periodic:false},
  normal:options.normal,orientation:options.orientation??'rmf',spacing:options.spacing??'parameter',initial_sections:options.initialSections??5,max_sections:options.maxSections??257,max_deviation:options.maxDeviation,length_tolerance:options.lengthTolerance??0.001,length_max_cells:options.lengthMaxCells??100000})

/** Closed contour shells with an identical repeated endpoint section; no caps. */
export const createPeriodicBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_periodic_section_loft',{sections})

/** Streams side-wall previews on the body face budget, then constructs audited
 * caps/seams. A preview level never contains an authoritative B-rep body.
 */
export async function* streamProgressiveBrepProfileBody(
 loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,control:import('../../nurbsConstructors').ProgressiveSweepStreamOptions={},
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
 // brep-core MAX_FACES is 1024; an open path reserves its two cap faces.
 const maximum=Math.min(options.maxSections??257,Math.floor((1024-(first.report.closedPath?0:2))/spans)+1)
 const bounded={...options,maxSections:maximum}
 if((options.initialSections??5)>maximum)throw new Error('Progressive body initial sections exceed face budget')
 const stream=streamProgressiveNurbsProfiles(profiles,path,scale,twist,bounded,control)
 try{
  for(;;){
   const level=await stream.next()
   checkAbort()
   if(level.done){
    if(!level.value.report.accepted)throw new Error('Progressive body sampled refinement exceeds budget')
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

/** Stream retained miter walls; build topology only after acceptance and a cancellation boundary. */
export async function* streamProgressiveMiterBrepProfileBody(
 loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,control:import('../../nurbsConstructors').ProgressiveSweepStreamOptions={},
):AsyncGenerator<import('../../nurbsConstructors').ProgressiveSweepPreview,ProgressiveMiterBrepBody,void>{
 const checkAbort=()=>{control.signal?.throwIfAborted();if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 checkAbort()
 // Retain the same original request through all post-preview certificates.
 ;({loops,points,scale,twist,options}=structuredClone({loops,points,scale,twist,options}))
 if(!loops.length||loops.length>16||loops.some(loop=>!loop.length))throw new Error('Miter body needs 1..16 nonempty loops')
 const profiles=loops.flat(),edges=points.length-(options.closed?0:1)
 const spans=profiles.reduce((n,c)=>n+decomposeNurbsCurve(c).length,0)
 if(edges<1||!spans||spans>64)throw new Error('Progressive miter body exceeds its site/span budget')
 const maximum=Math.min(options.maxSteps??64,Math.floor(1024/edges),Math.floor((1024-(options.closed?0:2))/(edges*spans)))
 if(maximum<(options.initialSteps??1))throw new Error('Progressive miter initial steps exceed face budget')
 const stream=streamProgressiveMiterNurbsProfiles(profiles,points,scale,twist,{...options,maxSteps:maximum},control)
 try{
  for(;;){
   const level=await stream.next();checkAbort()
   if(level.done){
    const approximation=level.value
    if(approximation.report.frameTransportCertified===false)throw new Error('Progressive miter frame transport could not be proved; review the path, normal and miter limit')
    if(approximation.report.certifiedErrorUpper===null)throw new Error(`Progressive miter wall error bound could not be proved: ${approximation.report.errorCertificateReason??'unresolved certificate'}`)
    if(approximation.report.profileRegularityCertified===false)throw new Error('Progressive miter profile tangent regularity could not be proved')
    if(approximation.report.wallRegularityCertified===false)throw new Error('Progressive miter retained wall Jacobian regularity could not be proved')
    if(!approximation.report.accepted||!approximation.sections)throw new Error('Progressive miter refinement/phase budget not met within the body face budget')
    await new Promise<void>(resolve=>setTimeout(resolve,0));checkAbort()
    const correction=correctMiterSections(approximation.sections,points,options,checkAbort)
    const retainedSections=correction?.sections??approximation.sections
    const sectionCorrection=correction?(({sections,...evidence})=>evidence)(correction):undefined
    const wallAudit=inspectProgressiveMiterWalls(profiles,points,scale,twist,{...options,maxSteps:maximum},retainedSections,options.wallAuditBudgets??defaultMiterWallAuditBudgets,loops.map(loop=>loop.length))
    checkAbort()
    const sections=retainedSections.map(row=>{let offset=0;return loops.map(loop=>{const wire=row.slice(offset,offset+loop.length);offset+=loop.length;return wire})})
    const capDomains=auditMiterCapDomains(sections,options,checkAbort)
    const model=options.closed?createPeriodicBrepSectionLoft(sections):createRationalBrepSectionLoft(sections)
    checkAbort()
    const retainedCorrespondence=inspectSweepRetainedCorrespondence(model,sections,options.closed??false)
 const retainedDecomposition=retainedCorrespondence.exact?null:inspectSweepRetainedDecomposition(model,sections,options.closed??false,options.retainedDecompositionBudgets?.maxProducts??100000,options.retainedDecompositionBudgets?.maxFaces??1024)
 const decompositionError=retainedCorrespondence.exact?0:retainedDecomposition?.wallErrorUpper??null
 if(sectionCorrection&&decompositionError===null)throw new Error('Corrected progressive miter retained wall correspondence unproved')
 const baseWallError=addCertifiedErrorUpper(approximation.report.certifiedErrorUpper!,sectionCorrection?.wallDisplacementUpper??0)
 const retainedWallErrorUpper=baseWallError===null||decompositionError===null?null:addCertifiedErrorUpper(baseWallError,decompositionError)
 if(sectionCorrection&&(retainedWallErrorUpper===null||retainedWallErrorUpper>options.maxDeviation))throw new Error('Corrected progressive miter retained wall error exceeds max_deviation or is unproved')
 const retainedCaps=options.closed?null:inspectSweepRetainedCaps(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets)
 const retainedCapDecomposition=options.closed||retainedCaps?.exact?null:inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets,options.retainedDecompositionBudgets?.maxProducts??100000)
 if(sectionCorrection&&!options.closed&&!retainedCaps?.exact&&!retainedCapDecomposition?.certified)throw new Error('Corrected progressive miter filled retained cap regions unproved')
 const idealCapDomains=options.closed?null:inspectProgressiveMiterIdealCapDomains(profiles,points,scale,twist,options,loops.map(loop=>loop.length),options.capDomainBudgets)
 const capProjection=options.closed?null:inspectProgressiveMiterCapProjection(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const capParallelism=options.closed?null:inspectProgressiveMiterCapParallelism(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const filledCapErrorUpper=options.closed?null:filledMiterCapErrorUpper({parallelPlanesCertified:capParallelism?.parallel??null,idealCapDomainsCertified:idealCapDomains?.idealCapDomainsCertified===true,retainedCapRegionsExact:retainedCaps?.exact===true||retainedCapDecomposition?.certified===true,decompositionErrorUpper:retainedCaps?.exact?[0,0]:retainedCapDecomposition?.capErrorUpper??null,projectionNormalDots:capProjection?.normalDots??null,endpointContourErrorUpper:approximation.report.endpointContourErrorUpper,correctionDisplacementUpper:sectionCorrection?sectionCorrection.wallDisplacementUpper:0})
 const boundaryErrorUpper=certifiedSweepBoundaryErrorUpper(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false)
 const boundaryErrorWithinBudget=boundaryErrorUpper===null?null:boundaryErrorUpper<=options.maxDeviation
 const boundaryCertificate=composeSweepBoundaryCertificate(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false,options.maxDeviation)
 if(boundaryCertificate.withinBudget===false)throw new Error('Progressive miter complete boundary error exceeds max_deviation')
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.retainedWallMaxInjectivityCells??1000,checkAbort)
    if(sectionCorrection&&!retainedWallCharts.allChartsCertified)throw new Error('Corrected progressive miter retained wall regularity unproved')
 const capPairs=options.closed?null:inspectSweepCapPairs(model,[model.faces.length-2,model.faces.length-1],options.capPairAuditBudgets??{clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000},checkAbort)
    const capContacts=options.closed?null:inspectSweepCapContacts(model,[model.faces.length-2,model.faces.length-1],options.capWallMaxWalls??1024,checkAbort)
    checkAbort()
    const embedding=options.closed?null:inspectSweepEmbedding(model,[model.faces.length-2,model.faces.length-1],options.embeddingBudgets??DEFAULT_SWEEP_EMBEDDING_BUDGETS)
    checkAbort()
    const volume=inspectSweepVolume(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS)
    checkAbort();return ownMiterProof({model,approximation,profileSmoothness:inspectMiterProfileSmoothness(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],undefined,checkAbort),boundaryCertificate,retainedCapDecomposition,retainedDecomposition,capParallelism,boundaryErrorWithinBudget,boundaryErrorUpper,filledCapErrorUpper,idealCapDomains,capProjection,sectionCorrection,retainedWallErrorUpper,wallAudit,retainedCorrespondence,retainedCaps,retainedWallCharts,capDomains,capContacts,capPairs,embedding,volume,globalEmbeddingCertified:false},sections,edges)
   }
   const rows=level.value.sections.map(row=>row.map(c=>decomposeNurbsCurve(c).map(span=>span.curve)))
   const patches:NurbsSurface[]=[],profilePatchRanges:[number,number][]=[]
   for(let p=0;p<profiles.length;p++){
    const start=patches.length
    for(let station=0;station<rows.length-1;station++)for(let span=0;span<rows[station]![p]!.length;span++){
     const a=rows[station]![p]![span]!,b=rows[station+1]![p]![span]!
     patches.push({degreeU:a.degree,degreeV:1,knotsU:a.knots,knotsV:[0,0,1,1],
      controlPoints:a.controlPoints.map((point,i)=>[point,b.controlPoints[i]!]),weights:a.weights.map((w,i)=>[w,b.weights[i]!]),periodicU:false,periodicV:false})
    }
    profilePatchRanges.push([start,patches.length])
   }
   checkAbort();yield {preview:true,patches,profilePatchRanges,report:level.value.report};checkAbort()
  }
 }finally{await stream.return(undefined as never)}
}

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
