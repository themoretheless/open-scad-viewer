import type {NurbsBrep} from './geometry/brep'
import type {NurbsCurve} from './nurbsCurve'
import type {NurbsSurface} from './nurbsSurface'
import {callNurbsRust} from './geometry/nurbs'
import {inspectSweepBoundaryCoverage,inspectSweepProjectiveSeams,type SweepProjectiveSeamAudit,type SweepSeamDeclaration} from './nurbsSweepAudit'
export interface MiterStationSmoothness {
 method:'retained-miter-station-joins'
 scope:'wall-station-seams'
 maxWork:number
 exactWork:number
 extractionComplete:boolean
 unclassifiedFaces:number[]
 unpairedEdges:number[]
 capEdges:number[]
 edgeIds:number[]
 g2:SweepProjectiveSeamAudit
 g1Audit:SweepProjectiveSeamAudit|null
 stationG1Certified:boolean
 stationG2Certified:boolean
}
function stationBoundary(surface:NurbsSurface,pcurve:NurbsCurve):'vMin'|'vMax'|null {
 if(pcurve.degree!==1||pcurve.controlPoints.length!==2||pcurve.weights.length!==2||pcurve.weights[0]!==pcurve.weights[1]||!(pcurve.weights[0]!>0))return null
 const [a,b]=pcurve.controlPoints
 if(!a||!b||a[1]!==b[1])return null
 const u0=surface.knotsU[surface.degreeU]!,u1=surface.knotsU[surface.controlPoints.length]!
 const v0=surface.knotsV[surface.degreeV]!,v1=surface.knotsV[surface.controlPoints[0]!.length]!
 if(!((a[0]===u0&&b[0]===u1)||(a[0]===u1&&b[0]===u0)))return null
 const boundary=a[1]===v0?'vMin':a[1]===v1?'vMax':null
 if(!boundary)return null
 try{return inspectSweepBoundaryCoverage(surface,pcurve,boundary).wholeBoundaryCovered?boundary:null}
 catch{return null}
}
/** A numerical proposal only. Exact strip identities and regularity decide
 * admission; approximate parallelism never produces a positive certificate. */
function proposedNormalScale(a:NurbsSurface,b:NurbsSurface,ab:'vMin'|'vMax',bb:'vMin'|'vMax'):number {
 return callNurbsRust('surface_station_normal_scale',{reference:a,edited:b,referenceBoundary:ab,editedBoundary:bb})
}
/** Full natural V boundaries only. Cap joins are explicitly excluded; the
 * whole retained station set shares one G2/G1 budget, including closed seams.
 * Failure of this sufficient criterion does not prove geometric nonsmoothness. */
export function inspectMiterStationSmoothness(model:NurbsBrep,capFaces:number[],maxWork=2000000,checkAbort?:()=>void):MiterStationSmoothness {
 if(new Set(capFaces).size!==capFaces.length||capFaces.some(f=>!Number.isSafeInteger(f)||f<0||f>=model.faces.length))throw new Error('Invalid station smoothness cap scope')
 const caps=new Set(capFaces),capOwned=new Set<number>(),capEdges:number[]=[],unclassifiedFaces:number[]=[],unpairedEdges:number[]=[]
 for(const id of capFaces){const face=model.faces[id]!;for(const wire of [face.outer,...face.holes])for(const c of model.loops[wire]!.coedges)capOwned.add(c.edge)}
 const uses=new Map<number,{face:number;boundary:'vMin'|'vMax'}[]>()
 model.faces.forEach((face,index)=>{
  checkAbort?.();if(caps.has(index))return
  const found=new Set<string>()
  for(const wire of [face.outer,...face.holes])for(const c of model.loops[wire]!.coedges){
   const boundary=stationBoundary(face.surface,c.pcurve)
   if(boundary){found.add(boundary);const list=uses.get(c.edge)??[];list.push({face:index,boundary});uses.set(c.edge,list)}
  }
  if(!found.has('vMin')||!found.has('vMax'))unclassifiedFaces.push(index)
 })
 const seams:SweepSeamDeclaration[]=[],edgeIds:number[]=[]
 for(const [edge,list] of uses){
  if(list.length===1&&capOwned.has(edge)){capEdges.push(edge);continue}
  if(list.length!==2||capOwned.has(edge)){unpairedEdges.push(edge);continue}
  const [a,b]=list as [{face:number;boundary:'vMin'|'vMax'},{face:number;boundary:'vMin'|'vMax'}]
  seams.push({patches:[a.face,b.face],boundaries:[a.boundary,b.boundary],order:2,normalScale:proposedNormalScale(model.faces[a.face]!.surface,model.faces[b.face]!.surface,a.boundary,b.boundary),jetTolerance:0});edgeIds.push(edge)
 }
 const patches=model.faces.map(f=>f.surface)
 const g2=inspectSweepProjectiveSeams(patches,seams,maxWork,checkAbort)
 const extractionComplete=!unclassifiedFaces.length&&!unpairedEdges.length
 if(!extractionComplete){g2.exactG1G2Certified=false;g2.certifiedOrder=null}
 const g1Audit=g2.exactG1G2Certified?null:inspectSweepProjectiveSeams(patches,seams.map(s=>({...s,order:1 as const})),maxWork-g2.exactWork,checkAbort)
 if(g1Audit&&!extractionComplete){g1Audit.exactG1G2Certified=false;g1Audit.certifiedOrder=null}
 return {method:'retained-miter-station-joins',scope:'wall-station-seams',maxWork,exactWork:g2.exactWork+(g1Audit?.exactWork??0),extractionComplete,unclassifiedFaces,unpairedEdges,capEdges,edgeIds,g2,g1Audit,stationG2Certified:g2.exactG1G2Certified,stationG1Certified:extractionComplete&&(g2.exactG1G2Certified||g1Audit?.exactG1G2Certified===true)}
}
