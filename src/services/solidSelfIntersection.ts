import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
import {faceContactExpectation,validFaceContacts,type FaceContacts,type FaceContactLimits} from './solidFaceContacts'
export interface FaceInjectivity {
 proven:boolean;projection:[number,number]|null;contractionUpper:number|null;spans:number
 reason:'global-projection-contraction'|'projection-not-proven'|'work-limit'|'periodic-domain'
}
export interface SelfIntersection extends Omit<FaceContacts,'scope'> {
 scope:'within-face-and-distinct-face-pairs';absenceProven:boolean;allFacesInjective:boolean
 spans:number;maxSpans:number;faces:Array<{face:number;result:FaceInjectivity|null}>
}
export function selfIntersectionExpectation(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits,maxSpans:number){
 const spans=(knots:number[],degree:number,count:number)=>knots.slice(degree,count).filter((k,i)=>k<knots[degree+i+1]).length
 return {...faceContactExpectation(model,toleranceUv,limits),maxSpans,faceSpans:model.faces.map(({surface:s})=>spans(s.knotsU,s.degreeU,s.controlPoints.length)*spans(s.knotsV,s.degreeV,s.controlPoints[0].length))}
}
export function validSelfIntersection(e:ReturnType<typeof selfIntersectionExpectation>,value:unknown):value is SelfIntersection {
 const r=value as SelfIntersection
 if(!r||r.scope!=='within-face-and-distinct-face-pairs'||r.maxSpans!==e.maxSpans||!Array.isArray(r.faces)||r.faces.length!==e.domains.length||!validFaceContacts(e,{...r,scope:'distinct-face-pairs'}))return false
 let used=0,all=true
 for(let i=0;i<r.faces.length;i++){
  const f=r.faces[i];if(!f||f.face!==i)return false
  const x=f.result
  if(used===e.maxSpans){if(x!==null)return false;all=false;continue}
  if(!x||typeof x.proven!=='boolean'||!Number.isSafeInteger(x.spans)||x.spans<0)return false
  const periodic=e.domains[i].periodic,expected=periodic?0:Math.min(e.faceSpans[i],e.maxSpans-used)
  if(x.spans!==expected)return false
  used+=x.spans
  if(x.proven){
   if(periodic||x.spans!==e.faceSpans[i]||x.reason!=='global-projection-contraction'||!Array.isArray(x.projection)||x.projection.length!==2||![[0,1],[0,2],[1,2]].some(p=>p[0]===x.projection![0]&&p[1]===x.projection![1])||typeof x.contractionUpper!=='number'||!Number.isFinite(x.contractionUpper)||x.contractionUpper<0||x.contractionUpper>=1)return false
  }else{
   all=false
   if(x.projection!==null||x.contractionUpper!==null||x.reason!==(periodic?'periodic-domain':expected<e.faceSpans[i]?'work-limit':'projection-not-proven'))return false
  }
 }
 return r.spans===used&&r.allFacesInjective===all&&r.absenceProven===(all&&r.allPairsClassified)
}
export function inspectSelfIntersection(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits,maxSpans:number):SelfIntersection {
 return callGeometryRust<SelfIntersection>('cad_self_intersection',{model,toleranceUv,...limits,maxSpans})
}
