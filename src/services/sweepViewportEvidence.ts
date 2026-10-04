import type {NativeGeometryArtifact} from '../core/nativeGeometry'
import {MAX_SWEEP_SEAM_WORK} from './nurbsSweepAudit'
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
