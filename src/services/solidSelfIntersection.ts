import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
import {faceContactExpectation,validFaceContacts,type FaceContacts,type FaceContactLimits} from './solidFaceContacts'
export interface FaceInjectivity {
 proven:boolean;projection:[number,number]|null;linearProjection?:[[number,number,number],[number,number,number]]|null;projectiveProjection?:number[][]|null;contractionUpper:number|null;spans:number
 reason:'global-projection-contraction'|'global-linear-projection-contraction'|'global-projective-projection-contraction'|'projection-not-proven'|'work-limit'|'periodic-domain'
}
export interface SelfIntersection extends Omit<FaceContacts,'scope'> {
 scope:'within-face-and-distinct-face-pairs';absenceProven:boolean;allFacesInjective:boolean
 spans:number;maxSpans:number;faces:Array<{face:number;result:FaceInjectivity|null}>
}
export function selfIntersectionExpectation(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits,maxSpans:number){
 const spans=(knots:number[],degree:number,count:number)=>knots.slice(degree,count).filter((k,i)=>k<knots[degree+i+1]).length
 const projectiveProjections=model.faces.map(({surface:s})=>[2,2,0,0,1,1].map((axis,i)=>{
  const free=axis===2?[0,1]:axis===0?[1,2]:[0,2],rows=Array.from({length:3},()=>[0,0,0,0])
  rows[0][free[0]]=1;rows[1][free[1]]=1;rows[2][axis]=i%2===0?1:-1
  rows[2][3]=s.controlPoints.reduce((n,row)=>row.reduce((m,p)=>Math.max(m,Math.abs(p[axis])),n),0)
  return rows
 }))
 return {...faceContactExpectation(model,toleranceUv,limits),maxSpans,projectiveProjections,faceSpans:model.faces.map(({surface:s})=>spans(s.knotsU,s.degreeU,s.controlPoints.length)*spans(s.knotsV,s.degreeV,s.controlPoints[0].length))}
}
const linearProjections=[[[1,1,0],[0,0,1]],[[1,-1,0],[0,0,1]],[[1,0,1],[0,1,0]],[[1,0,-1],[0,1,0]],[[0,1,1],[1,0,0]],[[0,1,-1],[1,0,0]]]
export function validSelfIntersection(e:ReturnType<typeof selfIntersectionExpectation>,value:unknown):value is SelfIntersection {
 const r=value as SelfIntersection
 if(!r||r.scope!=='within-face-and-distinct-face-pairs'||r.maxSpans!==e.maxSpans||!Array.isArray(r.faces)||r.faces.length!==e.domains.length||!validFaceContacts(e,{...r,scope:'distinct-face-pairs'}))return false
 let used=0,all=true
 for(let i=0;i<r.faces.length;i++){
  const f=r.faces[i];if(!f||f.face!==i)return false
  const x=f.result
  if(used===e.maxSpans){if(x!==null)return false;all=false;continue}
  if(!x||typeof x.proven!=='boolean'||!Number.isSafeInteger(x.spans)||x.spans<0)return false
  const periodic=e.domains[i].periodic,remaining=e.maxSpans-used,base=periodic?0:Math.min(e.faceSpans[i],remaining)
  if(x.spans<base||x.spans>remaining)return false
  used+=x.spans
  if(x.proven){
   if(periodic||typeof x.contractionUpper!=='number'||!Number.isFinite(x.contractionUpper)||x.contractionUpper<0||x.contractionUpper>=1)return false
   if(x.reason==='global-projection-contraction'){
    if(x.projectiveProjection!=null)return false
    if(x.spans!==e.faceSpans[i]||x.linearProjection!=null||!Array.isArray(x.projection)||x.projection.length!==2||![[0,1],[0,2],[1,2]].some(p=>p[0]===x.projection![0]&&p[1]===x.projection![1]))return false
   }else if(x.reason==='global-linear-projection-contraction'){
    if(x.projectiveProjection!=null)return false
    const basis=x.linearProjection
    if(x.projection!==null||!Array.isArray(basis)||basis.length!==2||!basis.every(row=>Array.isArray(row)&&row.length===3&&row.every(Number.isFinite)))return false
    const candidate=linearProjections.findIndex(p=>p.every((row,j)=>row.every((n,k)=>n===basis[j][k])))
    if(candidate<0||x.spans!==e.faceSpans[i]*(1+16*(candidate+1)))return false
   }else if(x.reason==='global-projective-projection-contraction'){
    const basis=x.projectiveProjection
    if(x.projection!==null||x.linearProjection!==null||!Array.isArray(basis)||basis.length!==3||!basis.every(row=>Array.isArray(row)&&row.length===4&&row.every(Number.isFinite)))return false
    const candidate=e.projectiveProjections[i].findIndex(p=>p.every((row,j)=>row.every((n,k)=>n===basis[j][k])))
    if(candidate<0||x.spans!==e.faceSpans[i]*(97+16*(candidate+1)))return false
   }else return false
  }else{
   all=false
   if(x.projection!==null||x.linearProjection!=null||x.projectiveProjection!=null||x.contractionUpper!==null)return false
   if(periodic){if(x.spans!==0||x.reason!=='periodic-domain')return false}
   else if(x.reason==='work-limit'){if(x.spans!==remaining)return false}
   else if(x.reason==='projection-not-proven'){
    const expected=e.faceSpans[i]*(x.projectiveProjection!==undefined?193:x.linearProjection===undefined?1:97)
    if(x.spans!==expected)return false
   }else return false
  }
 }
 return r.spans===used&&r.allFacesInjective===all&&r.absenceProven===(all&&r.allPairsClassified)
}
export function inspectSelfIntersection(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits,maxSpans:number):SelfIntersection {
 return callGeometryRust<SelfIntersection>('cad_self_intersection',{model,toleranceUv,...limits,maxSpans})
}
