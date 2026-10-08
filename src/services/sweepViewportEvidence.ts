import type {NativeGeometryArtifact} from '../core/nativeGeometry'
import {MAX_SWEEP_SEAM_WORK} from './nurbsSweepAudit'
/** Presentation of native constructor-owned body error; no Solid admission. */
export function readSweepBodyBoundaryViewportEvidence(artifact:NativeGeometryArtifact|undefined){
 if(!artifact||artifact.kind!=='brep')return null
 try {
  const report=JSON.parse(artifact.geometryJson)?.sweepBodyBoundaryEvidence
  if(!report||report.globalEmbeddingCertified!==false||report.boundaryErrorScope!=='constructor-owned-retained-wall-and-cap-union')return null
  const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)&&v>=0
  const upper=finite(report.boundaryErrorUpper)?report.boundaryErrorUpper:null
  const budget=finite(report.budget)?report.budget:null
  const walls=report.retainedWalls
  const wallsProved=walls?.certified===true&&walls.faceCoverageCertified===true&&walls.coefficientFamilyCertified===true&&
   Number.isSafeInteger(walls.inspectedFaces)&&walls.inspectedFaces>0&&walls.inspectedFaces<=1024&&
   Number.isSafeInteger(walls.exactWork)&&walls.exactWork>=0&&walls.exactWork<=1000000
  const capsProved=report.closedPath===true?report.retainedCaps===null&&report.filledCapErrorUpper===null:
   report.closedPath===false&&report.retainedCaps?.exact===true&&report.capProjection?.idealCapDomainsCertified===true&&
   Array.isArray(report.filledCapErrorUpper)&&report.filledCapErrorUpper.length===2&&report.filledCapErrorUpper.every(finite)&&
   Array.isArray(report.capProjection.normalDots)&&report.capProjection.normalDots.length===2&&
   report.capProjection.normalDots.every((dot:unknown)=>Array.isArray(dot)&&dot.length===2&&
    dot.every((v:unknown)=>typeof v==='number'&&Number.isFinite(v))&&dot[0]<=dot[1]&&(dot[0]>0||dot[1]<0))
  const complete=report.boundaryContinuousBound===true&&wallsProved&&capsProved&&upper!==null&&budget!==null&&
   typeof report.boundaryErrorWithinBudget==='boolean'&&report.boundaryErrorWithinBudget===(upper<=budget)
  return {nodeId:artifact.nodeId,continuousBound:complete,errorUpper:complete?upper:null,budget,
   withinBudget:complete?report.boundaryErrorWithinBudget as boolean:null}
 }catch{return null}
}
export interface SweepViewportEvidence {
 nodeId:string
 solidGeometryCertified:boolean
 continuousBound:boolean
 profileRegularityCertified:boolean
 wallRegularityCertified:boolean
 profileG2Certified?:boolean
 profileG1Certified?:boolean
 profileSeamCount?:number
 stationG1Certified?:boolean
 stationG2Certified?:boolean
 stationSeamCount?:number
 stationContinuity?:'C0'|'G1'|'G2'
 capContinuity?:'C0'|'absent'
 boundaryErrorUpper:number|null
 boundaryErrorBudget:number|null
 boundaryErrorWithinBudget:boolean|null
}
/** Presentation only. Geometry admission must independently revalidate B-rep. */
export function readSweepViewportEvidence(artifact:NativeGeometryArtifact|undefined):SweepViewportEvidence|null {
 if(!artifact||artifact.kind!=='brep')return null
 try {
  const evidence=JSON.parse(artifact.geometryJson)?.sweepEvidence
  if(!evidence||typeof evidence!=='object'||!evidence.volume||
   typeof evidence.volume.solidGeometryCertified!=='boolean')return null
  const validBound=(x:unknown):x is number=>typeof x==='number'&&Number.isFinite(x)&&x>=0
  const upper=validBound(evidence.boundaryErrorUpper)?evidence.boundaryErrorUpper:null
  const budget=validBound(evidence.boundaryErrorBudget)?evidence.boundaryErrorBudget:null
  const within=upper!==null&&budget!==null&&typeof evidence.boundaryErrorWithinBudget==='boolean'&&evidence.boundaryErrorWithinBudget===(upper<=budget)?evidence.boundaryErrorWithinBudget:null
  const certificate=evidence.boundaryCertificate
  const completeBound=evidence.continuousBound===true&&certificate?.method==='retained-sweep-boundary-union'&&certificate.scope==='boundary-set-hausdorff'&&certificate.continuousBound===true&&upper!==null&&budget!==null&&certificate.errorUpper===upper&&certificate.budget===budget&&certificate.withinBudget===within&&within!==null
  const smooth=evidence.profileSmoothness
  const profile=smooth?.profile
  const smoothnessDeclared=smooth?.method==='retained-miter-profile-joins'&&smooth.scope==='wall-profile-seams'&&Array.isArray(smooth.edgeIds)&&Array.isArray(profile?.seams)
  const completeExtraction=smoothnessDeclared&&smooth.extractionComplete===true&&Array.isArray(smooth.unclassifiedFaces)&&smooth.unclassifiedFaces.length===0&&Array.isArray(smooth.unpairedEdges)&&smooth.unpairedEdges.length===0&&smooth.edgeIds.length>0&&new Set(smooth.edgeIds).size===smooth.edgeIds.length&&smooth.edgeIds.every((edge:number)=>Number.isSafeInteger(edge)&&edge>=0)
  const auditCertified=(audit:any,order:1|2,availableWork:number)=>completeExtraction&&audit?.method==='constant-projective-strip-jets'&&audit.exactG1G2Certified===true&&audit.certifiedOrder===order&&Array.isArray(audit.seams)&&smooth.edgeIds.length===audit.seams.length&&Number.isSafeInteger(audit.exactWork)&&audit.exactWork>=0&&audit.exactWork<=availableWork&&audit.seams.every((seam:any)=>seam&&seam.certified===true&&seam.exactIdentity===true&&seam.regularityCertified===true&&(seam.certifiedOrder===order||order===1&&seam.certifiedOrder===2))&&Array.isArray(audit.unresolvedSeams)&&audit.unresolvedSeams.length===0
  const smoothnessBudget=smooth?.maxWork===undefined?1000000:smooth.maxWork
  const validSmoothnessBudget=Number.isSafeInteger(smoothnessBudget)&&smoothnessBudget>=0&&smoothnessBudget<=MAX_SWEEP_SEAM_WORK
  const profileG2Certified=validSmoothnessBudget&&auditCertified(profile,2,smoothnessBudget)
  const profileG1Certified=profileG2Certified||validSmoothnessBudget&&smoothnessDeclared&&smooth.profileG1Certified===true&&smooth.g1Method==='exact-projective-audit'&&Number.isSafeInteger(profile.exactWork)&&profile.exactWork>=0&&profile.exactWork<=smoothnessBudget&&smooth.exactWork===profile.exactWork+smooth.g1Audit?.exactWork&&auditCertified(smooth.g1Audit,1,smoothnessBudget-profile.exactWork)
  const station=smooth?.station
  const stationEdges=station?.edgeIds
  const stationDeclared=station?.method==='retained-miter-station-joins'&&station.scope==='wall-station-seams'&&Array.isArray(stationEdges)
  const stationComplete=stationDeclared&&station.extractionComplete===true&&Array.isArray(station.unclassifiedFaces)&&station.unclassifiedFaces.length===0&&Array.isArray(station.unpairedEdges)&&station.unpairedEdges.length===0&&stationEdges.length>0&&new Set(stationEdges).size===stationEdges.length&&stationEdges.every((edge:number)=>Number.isSafeInteger(edge)&&edge>=0)
  const stationBudgetValid=stationDeclared&&validSmoothnessBudget&&Number.isSafeInteger(smooth.exactWork)&&smooth.exactWork>=0&&(!smooth.g1Audit||Number.isSafeInteger(smooth.g1Audit.exactWork)&&smooth.g1Audit.exactWork>=0)&&smooth.exactWork===profile?.exactWork+(smooth.g1Audit?.exactWork??0)&&smooth.exactWork<=smoothnessBudget&&station?.maxWork===smoothnessBudget-smooth.exactWork&&Number.isSafeInteger(station?.exactWork)&&station.exactWork>=0&&Number.isSafeInteger(station.g2?.exactWork)&&station.g2.exactWork>=0&&(!station.g1Audit||Number.isSafeInteger(station.g1Audit.exactWork)&&station.g1Audit.exactWork>=0)&&station.exactWork===station.g2?.exactWork+(station.g1Audit?.exactWork??0)&&station.exactWork<=station.maxWork&&smooth.totalExactWork===smooth.exactWork+station.exactWork
  const stationAuditCertified=(audit:any,order:1|2,available:number)=>stationComplete&&audit?.method==='constant-projective-strip-jets'&&audit.exactG1G2Certified===true&&audit.certifiedOrder===order&&Number.isSafeInteger(audit.exactWork)&&audit.exactWork>=0&&audit.exactWork<=available&&Array.isArray(audit.seams)&&audit.seams.length===stationEdges.length&&audit.seams.every((seam:any)=>seam?.certified===true&&seam.exactIdentity===true&&seam.regularityCertified===true&&seam.certifiedOrder===order)&&Array.isArray(audit.unresolvedSeams)&&audit.unresolvedSeams.length===0
  const stationG2Certified=stationBudgetValid&&stationAuditCertified(station.g2,2,station.maxWork)
  const stationG1Certified=stationG2Certified||stationBudgetValid&&station.stationG1Certified===true&&stationAuditCertified(station.g1Audit,1,station.maxWork-station.g2.exactWork)
  const stationContinuity=stationG2Certified?'G2' as const:stationG1Certified?'G1' as const:'C0' as const
  return {...(smoothnessDeclared?{profileG2Certified,profileG1Certified,profileSeamCount:profile.seams.length,stationContinuity,...(stationDeclared?{stationG1Certified,stationG2Certified,stationSeamCount:stationEdges.length}:{}),...(smooth.capContinuity==='C0'||smooth.capContinuity==='absent'?{capContinuity:smooth.capContinuity}: {})}:{}),nodeId:artifact.nodeId,solidGeometryCertified:evidence.volume.solidGeometryCertified,
   boundaryErrorUpper:upper,boundaryErrorBudget:budget,boundaryErrorWithinBudget:within,
   continuousBound:completeBound,
   profileRegularityCertified:evidence.profileRegularityCertified===true,
   wallRegularityCertified:evidence.wallRegularityCertified===true}
 }catch{return null}
}


/** Validate transport scope, coverage and work; all jet decisions are native. */
function readRetainedDecompositionProof(decomposition:any,order:1|2){
  const decompositionDeclared=decomposition?.method==='exact-retained-decomposition-strip-jets'&&
   decomposition.scope==='within-source-profile-decomposition-only'&&decomposition.requestedOrder===order&&
   decomposition.allProfileJoinsCertified===false&&decomposition.closedProfileSeamsCertified===false&&
   decomposition.sourceFrameSmoothnessCertified===false&&decomposition.capJoinsCertified===false&&
   decomposition.continuousBound===false&&decomposition.solidCertified===false
  const decompositionJoinCount=decompositionDeclared&&Number.isSafeInteger(decomposition.expectedJoins)&&
   decomposition.expectedJoins>0&&decomposition.expectedJoins<=4096?decomposition.expectedJoins as number:null
  const certified=decomposition===undefined||decompositionDeclared&&decomposition.expectedJoins===0?null:
   decompositionDeclared&&decompositionJoinCount!==null&&decomposition.coverageComplete===true&&
   decomposition.decompositionJoinsCertified===true&&decomposition.reason===null&&
   decomposition.checkedJoins===decompositionJoinCount&&Array.isArray(decomposition.joins)&&
   decomposition.joins.length===decompositionJoinCount&&Number.isSafeInteger(decomposition.exactWork)&&
   decomposition.exactWork>0&&decomposition.exactWork<=1000000&&
   decomposition.joins.every((j:any)=>Number.isSafeInteger(j.leftPatch)&&j.leftPatch>=0&&j.leftPatch<4096&&
    Number.isSafeInteger(j.rightPatch)&&j.rightPatch>=0&&j.rightPatch<4096&&j.leftPatch!==j.rightPatch&&
    j.certified===true&&j.exactIdentity===true&&j.regularityCertified===true&&
    j.reason==='exact-regular-projective-strip-jets'&&Number.isSafeInteger(j.exactWork)&&j.exactWork>0)&&
   new Set(decomposition.joins.map((j:any)=>`${j.leftPatch}:${j.rightPatch}`)).size===decompositionJoinCount&&
   decomposition.joins.reduce((sum:number,j:any)=>sum+j.exactWork,0)===decomposition.exactWork
  return {certified,count:decompositionJoinCount}
}

/** Scoped original-transport patch error; never a body, mesh or embedding proof. */
export function readSweepPatchViewportEvidence(artifact:NativeGeometryArtifact|undefined){
 if(!artifact||artifact.kind!=='patches')return null
 try{
  const report=JSON.parse(artifact.geometryJson)?.sweepPatchEvidence
  if(report?.method!=='progressive-fourfold-section-refinement'||report.accepted!==true)return null
  const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)&&v>=0
  const budget=finite(report.budget)?report.budget:null
  const cellLimit=report.errorCertificateMaxCells??10000
  const productLimit=report.decompositionMaxProducts??1000000
  const limitsValid=Number.isSafeInteger(cellLimit)&&cellLimit>=0&&cellLimit<=100000&&
   Number.isSafeInteger(productLimit)&&productLimit>=0&&productLimit<=1000000
  const complete=limitsValid&&report.continuousBound===true&&report.roundingCertified===true&&
   report.continuousErrorScope==='retained-patches-relative-to-original-profile-transport'&&
   finite(report.continuousErrorUpper)&&report.knownProfileErrorUpper===report.continuousErrorUpper&&budget!==null&&
   report.continuousErrorUpper<=budget&&Number.isSafeInteger(report.errorCertificateCells)&&report.errorCertificateCells>=0&&report.errorCertificateCells<=cellLimit&&
   Number.isSafeInteger(report.decompositionProducts)&&report.decompositionProducts>=0&&report.decompositionProducts<=productLimit
  const regularity=report.retainedPatchRegularity
  const surfaceRegularityCertified=regularity?.method==='actual-retained-patch-shared-jacobian-regularity'&&
   regularity.surfaceRegularityCertified===true&&regularity.continuousBound===false&&
   regularity.globalEmbeddingCertified===false&&regularity.solidCertified===false&&
   Number.isSafeInteger(regularity.cells)&&regularity.cells>=0&&regularity.cells<=10000&&
   Array.isArray(regularity.unresolvedPatches)&&regularity.unresolvedPatches.length===0
  const frame=report.sourceFrameRegularity
  const sourceFrameRegularityCertified=frame===undefined?null:
   (frame?.method==='original-path-adaptive-frenet-frame-regularity'||frame?.method==='original-path-adaptive-fixed-normal-frame-regularity')&&
   frame.regularityCertified===true&&frame.continuousBound===false&&frame.surfaceRegularityCertified===false&&
   frame.globalEmbeddingCertified===false&&Number.isSafeInteger(frame.cells)&&frame.cells>=0&&frame.cells<=10000&&
   Number.isSafeInteger(frame.certifiedIntervals)&&frame.certifiedIntervals>0&&frame.certifiedIntervals<=frame.cells
  const smoothness=report.sourceFrameSmoothness
  const sourceFrameC2Certified=smoothness===undefined?null:
   smoothness?.method==='original-frame-continuity-and-nondegeneracy-cover'&&
   smoothness.scope==='open-original-frame-only'&&smoothness.requestedOrder===2&&
   smoothness.sourceFrameSmoothnessCertified===true&&smoothness.reason===null&&smoothness.retainedSeamsCertified===false&&
   smoothness.profileJoinsCertified===false&&smoothness.capJoinsCertified===false&&
   smoothness.continuousBound===false&&smoothness.solidCertified===false&&
   Number.isSafeInteger(smoothness.cells)&&smoothness.cells>0&&smoothness.cells<=10000&&
   Number.isSafeInteger(smoothness.exactWork)&&smoothness.exactWork>=0&&smoothness.exactWork<=1000000
  const closed=report.closedSourceFrameSmoothness
  const closedSourceFrameC2Certified=closed===undefined?null:
   ((closed?.method==='exact-original-authored-endpoint-jets-and-frame-cover'&&closed.scope==='closed-original-authored-frame-only')||
    (closed?.method==='exact-original-path-twist-endpoint-jets-and-frame-cover'&&closed.scope==='closed-original-path-frame-only')||
    (closed?.method==='exact-original-guided-endpoint-jets-and-joint-frame-cover'&&closed.scope==='closed-original-guided-frame-only'))&&closed.requestedOrder===2&&
   closed.closedSourceFrameSmoothnessCertified===true&&closed.reason===null&&
   closed.pathSeamCertified===false&&closed.retainedSeamsCertified===false&&
   closed.profileJoinsCertified===false&&closed.capJoinsCertified===false&&
   closed.continuousBound===false&&closed.solidCertified===false&&
   Number.isSafeInteger(closed.cells)&&closed.cells>0&&closed.cells<=10000&&
   Number.isSafeInteger(closed.exactWork)&&closed.exactWork>0&&closed.exactWork<=1000000
  const closedC1=report.closedSourceFrameSmoothnessC1
  const closedSourceFrameC1Certified=closedC1===undefined?null:
   ((closedC1?.method==='exact-original-authored-endpoint-jets-and-frame-cover'&&closedC1.scope==='closed-original-authored-frame-only')||
    (closedC1?.method==='exact-original-path-twist-endpoint-jets-and-frame-cover'&&closedC1.scope==='closed-original-path-frame-only')||
    (closedC1?.method==='exact-original-guided-endpoint-jets-and-joint-frame-cover'&&closedC1.scope==='closed-original-guided-frame-only'))&&closedC1.requestedOrder===1&&
   closedC1.closedSourceFrameSmoothnessCertified===true&&closedC1.reason===null&&
   closedC1.pathSeamCertified===false&&closedC1.retainedSeamsCertified===false&&
   closedC1.profileJoinsCertified===false&&closedC1.capJoinsCertified===false&&
   closedC1.continuousBound===false&&closedC1.solidCertified===false&&
   Number.isSafeInteger(closedC1.cells)&&closedC1.cells>0&&closedC1.cells<=10000&&
   Number.isSafeInteger(closedC1.exactWork)&&closedC1.exactWork>0&&closedC1.exactWork<=1000000
  const decomposition=readRetainedDecompositionProof(report.retainedDecompositionSmoothness,2)
  const decompositionG2Certified=decomposition.certified
  const decompositionJoinCount=decomposition.count
  const fallback=report.retainedDecompositionG1Fallback
  const g1=readRetainedDecompositionProof(fallback?.g1??undefined,1)
  const g2=readRetainedDecompositionProof(fallback?.g2,2)
  const fallbackDeclared=fallback?.method==='exact-retained-decomposition-smoothness'&&
   fallback.scope==='within-source-profile-decomposition-only'&&fallback.allProfileJoinsCertified===false&&
   fallback.closedProfileSeamsCertified===false&&fallback.sourceFrameSmoothnessCertified===false&&
   fallback.capJoinsCertified===false&&fallback.continuousBound===false&&fallback.solidCertified===false&&
   Number.isSafeInteger(fallback.maxExactWork)&&fallback.maxExactWork>=0&&fallback.maxExactWork<=1000000&&
   Number.isSafeInteger(fallback.exactWork)&&fallback.exactWork>=0&&fallback.exactWork<=fallback.maxExactWork&&
   Number.isSafeInteger(fallback.g2?.exactWork)&&fallback.g2.exactWork>=0&&
   fallback.g2.exactWork<=Math.floor(fallback.maxExactWork/2)&&
   fallback.exactWork===fallback.g2.exactWork+(fallback.g1?.exactWork??0)
  const decompositionG1Certified=fallback===undefined||g2.count===null&&g1.count===null?null:
   fallbackDeclared&&fallback.decompositionG1Certified===true&&
   (fallback.g1===null?g2.certified===true:g1.certified===true&&g1.count===g2.count)
  return {nodeId:artifact.nodeId,decompositionG1Certified,decompositionG2Certified,decompositionJoinCount,closedSourceFrameC1Certified,closedSourceFrameC2Certified,sourceFrameC2Certified,sourceFrameRegularityCertified,surfaceRegularityCertified,continuousBound:complete,errorUpper:complete?report.continuousErrorUpper as number:null,budget,
   reason:complete?null:typeof report.errorCertificateReason==='string'?report.errorCertificateReason:null}
 }catch{return null}
}
