import {inspectMiterStationSmoothness,type MiterStationSmoothness} from './miterStationSmoothness'
import type {NurbsBrep} from './geometry/brep'
import type {NurbsSurface} from './nurbsSurface'
import type {NurbsCurve} from './nurbsCurve'
import {inspectSweepBoundaryCoverage,MAX_SWEEP_SEAM_WORK,inspectSweepProjectiveSeams,type SweepProjectiveSeamAudit,type SweepSeamDeclaration} from './nurbsSweepAudit'
export interface MiterProfileSmoothness {
 method:'retained-miter-profile-joins'
 maxWork:number
 scope:'wall-profile-seams'
 extractionComplete:boolean
 unclassifiedFaces:number[]
 unpairedEdges:number[]
 edgeIds:number[]
 profile:SweepProjectiveSeamAudit
 g1Audit:SweepProjectiveSeamAudit|null
 profileG1Certified:boolean
 g1Method:'implied-by-G2'|'exact-projective-audit'|'unproved'
 exactWork:number
 station:MiterStationSmoothness
 totalExactWork:number
 stationContinuity:'C0'|'G1'|'G2'
 capContinuity:'C0'|'absent'
 fullBoundarySmoothnessCertified:false
}
/** Natural U boundaries of retained wall charts only; arbitrary trimmed or
 * partial boundaries are not inferred to be profile seams. */
function profileBoundary(surface:NurbsSurface,pcurve:NurbsCurve):'uMin'|'uMax'|null {
 if(pcurve.degree!==1||pcurve.controlPoints.length!==2||pcurve.weights.length!==2||pcurve.weights[0]!==pcurve.weights[1]||!(pcurve.weights[0]!>0))return null
 const [a,b]=pcurve.controlPoints
 if(!a||!b||a[0]!==b[0])return null
 const u0=surface.knotsU[surface.degreeU]!,u1=surface.knotsU[surface.controlPoints.length]!
 const v0=surface.knotsV[surface.degreeV]!,v1=surface.knotsV[surface.controlPoints[0]!.length]!
 if(!((a[1]===v0&&b[1]===v1)||(a[1]===v1&&b[1]===v0)))return null
 const boundary=a[0]===u0?'uMin':a[0]===u1?'uMax':null
 if(!boundary)return null
 try{return inspectSweepBoundaryCoverage(surface,pcurve,boundary).wholeBoundaryCovered?boundary:null}
 catch{return null}
}
/** Uses retained topology to declare wall profile joins, then independently
 * proves their represented geometry with a single exact-work budget. */
export function inspectMiterProfileSmoothness(model:NurbsBrep,capFaces:number[],maxWork=MAX_SWEEP_SEAM_WORK,checkAbort?:()=>void):MiterProfileSmoothness {
 const unclassifiedFaces:number[]=[],unpairedEdges:number[]=[]
 const caps=new Set(capFaces),uses=new Map<number,{face:number;boundary:'uMin'|'uMax'}[]>()
 model.faces.forEach((face,index)=>{
  checkAbort?.()
  if(caps.has(index))return
  const boundaries=new Set<string>()
  for(const wire of [face.outer,...face.holes])for(const coedge of model.loops[wire]!.coedges){
   const boundary=profileBoundary(face.surface,coedge.pcurve)
   if(boundary){boundaries.add(boundary);const list=uses.get(coedge.edge)??[];list.push({face:index,boundary});uses.set(coedge.edge,list)}
  }
  if(!boundaries.has('uMin')||!boundaries.has('uMax'))unclassifiedFaces.push(index)
 })
 const seams:SweepSeamDeclaration[]=[],edgeIds:number[]=[]
 for(const [edge,list] of uses){
  if(list.length!==2){unpairedEdges.push(edge);continue}
  const [a,b]=list as [{face:number;boundary:'uMin'|'uMax'},{face:number;boundary:'uMin'|'uMax'}]
  seams.push({patches:[a.face,b.face],boundaries:[a.boundary,b.boundary],order:2,normalScale:1,jetTolerance:0});edgeIds.push(edge)
 }
 const profile=inspectSweepProjectiveSeams(model.faces.map(face=>face.surface),seams,maxWork,checkAbort)
 const extractionComplete=unclassifiedFaces.length===0&&unpairedEdges.length===0
 if(!extractionComplete){profile.exactG1G2Certified=false;profile.certifiedOrder=null}
 // G2 implies G1; otherwise audit G1 independently with only the remaining
 // exact-work budget. Failure of this sufficient G2 criterion is not G1 failure.
 const g1Audit=profile.exactG1G2Certified?null:inspectSweepProjectiveSeams(model.faces.map(face=>face.surface),seams.map(seam=>({...seam,order:1 as const})),maxWork-profile.exactWork,checkAbort)
 if(g1Audit&&!extractionComplete){g1Audit.exactG1G2Certified=false;g1Audit.certifiedOrder=null}
 const profileG1Certified=extractionComplete&&(profile.exactG1G2Certified||g1Audit?.exactG1G2Certified===true)
 const exactWork=profile.exactWork+(g1Audit?.exactWork??0)
 const station=inspectMiterStationSmoothness(model,capFaces,maxWork-exactWork,checkAbort)
 const totalExactWork=exactWork+station.exactWork
 return {method:'retained-miter-profile-joins',maxWork,scope:'wall-profile-seams',edgeIds,extractionComplete,unclassifiedFaces,unpairedEdges,profile,g1Audit,profileG1Certified,exactWork,g1Method:profileG1Certified?(profile.exactG1G2Certified?'implied-by-G2':'exact-projective-audit'):'unproved',
  station,totalExactWork,stationContinuity:station.stationG2Certified?'G2':station.stationG1Certified?'G1':'C0',capContinuity:capFaces.length?'C0':'absent',fullBoundarySmoothnessCertified:false}
}
