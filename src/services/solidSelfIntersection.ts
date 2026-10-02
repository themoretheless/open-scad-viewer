import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
import {faceContactExpectation,validFaceContacts,type FaceContacts,type FaceContactLimits} from './solidFaceContacts'
export interface FaceInjectivity {
 proven:boolean;projection:[number,number]|null;linearProjection?:[[number,number,number],[number,number,number]]|null;projectiveProjection?:number[][]|null;polarProjection?:number[][]|null;contractionUpper:number|null;spans:number
 reason:'global-projection-contraction'|'global-linear-projection-contraction'|'global-projective-projection-contraction'|'global-polar-projection-contraction'|'collapsed-boundary-requires-quotient-proof'|'projection-not-proven'|'work-limit'|'periodic-domain'
}
export interface SelfIntersection extends Omit<FaceContacts,'scope'> {
 scope:'within-face-and-distinct-face-pairs';absenceProven:boolean;allFacesInjective:boolean
 spans:number;maxSpans:number;faces:Array<{face:number;result:FaceInjectivity|null;quotientProof?:QuotientProof|null}>
}
export interface QuotientProof {
 collapsedEnd:number;poleEdge:number;poleVertex:number;proven:boolean;cells:number;reason:string
 sourceFrame:number[][]|null;weightedBounds:[number,number,number,number]|null;dominanceMarginLower:number|null;bandMarginsLower:number[]|null
 sourceSurface:NurbsBrep['faces'][number]['surface']
}
export function faceAbsenceProven(f:SelfIntersection['faces'][number]):boolean{return !!(f.result?.proven||f.quotientProof?.proven)}
export function selfIntersectionExpectation(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits,maxSpans:number){
 const spans=(knots:number[],degree:number,count:number)=>knots.slice(degree,count).filter((k,i)=>k<knots[degree+i+1]).length
 const projectiveData=model.faces.map(({surface:s})=>[0,1,2].map(axis=>{
  const anchor=s.controlPoints[s.controlPoints.length-1][0][axis]
  return [s.controlPoints[0][0][axis],anchor,s.controlPoints.reduce((n,row)=>row.reduce((m,p)=>Math.max(m,Math.abs(p[axis]-anchor)),n),0)]
 }))
 const collapsed=model.faces.map(({surface:s})=>{
  const p=s.controlPoints,eq=(a:number[],b:number[])=>a.every((n,k)=>n===b[k])
  return [0,p.length-1].some(i=>{
   const knots=i===0?s.knotsU.slice(0,s.degreeU+1):s.knotsU.slice(p.length)
   return knots.every(k=>k===knots[0])&&p[i].every(x=>eq(x,p[i][0]))
  })||[0,p[0].length-1].some(j=>{
   const knots=j===0?s.knotsV.slice(0,s.degreeV+1):s.knotsV.slice(p[0].length)
   return knots.every(k=>k===knots[0])&&p.every(row=>eq(row[j],p[0][j]))
  })
 })
 return {...faceContactExpectation(model,toleranceUv,limits),maxSpans,projectiveData,collapsed,quotients:model.faces.map((_,i)=>quotientExpectation(model,i)),polarCandidates:model.faces.map(({surface})=>polarCandidates(surface)),faceSpans:model.faces.map(({surface:s})=>spans(s.knotsU,s.degreeU,s.controlPoints.length)*spans(s.knotsV,s.degreeV,s.controlPoints[0].length))}
}
function quotientExpectation(model:NurbsBrep,face:number){
 const f=model.faces[face],s=f.surface,p=s.controlPoints,u=[s.knotsU[s.degreeU],s.knotsU[p.length]],v=[s.knotsV[s.degreeV],s.knotsV[p[0].length]]
 const eq=(a:number[],b:number[])=>a.length===b.length&&a.every((n,k)=>n===b[k])
 for(const loop of [f.outer,...f.holes])for(const ce of model.loops[loop].coedges){
  const edge=model.edges[ce.edge],curve=ce.pcurve,pole=model.vertices[edge.vertices[0]].point
  if(edge.vertices[0]!==edge.vertices[1]||!edge.curve.controlPoints.every(p=>eq(p,pole))||curve.degree!==1||curve.controlPoints.length!==2)continue
  const [a,b]=curve.controlPoints
  if(a[0]!==b[0]||!((a[1]===v[0]&&b[1]===v[1])||(a[1]===v[1]&&b[1]===v[0])))continue
  const end=u.findIndex(x=>x===a[0]);if(end<0)continue
  const index=(i:number)=>end===0?i:p.length-1-i
  if(!p[index(0)].every(point=>eq(point,pole)))continue
  const clamped=(k:number[],d:number)=>k.length===2*(d+1)&&k.slice(0,d+1).every(v=>v===k[d])&&k.slice(d+1).every(v=>v===k[d+1])
  const supported=!s.periodicU&&!s.periodicV&&s.degreeU>=2&&s.degreeU<=8&&s.degreeV<=8&&p.length===s.degreeU+1&&p[0].length===s.degreeV+1&&clamped(s.knotsU,s.degreeU)&&clamped(s.knotsV,s.degreeV)
  return {collapsedEnd:end,poleEdge:ce.edge,poleVertex:edge.vertices[0],sourceEvidence:surfaceEvidence(s),sourceFrame:supported?[p[index(0)][0],p[index(1)][0],p[index(2)][0],p[index(2)][p[0].length-1]]:null}
 }
 return null
}
function surfaceEvidence(s:NurbsBrep['faces'][number]['surface']):number[]|null{
 if(!s||!Number.isSafeInteger(s.degreeU)||!Number.isSafeInteger(s.degreeV)||!Array.isArray(s.knotsU)||!Array.isArray(s.knotsV)||!Array.isArray(s.controlPoints)||s.controlPoints.length<2||!Array.isArray(s.controlPoints[0])||s.controlPoints[0].length<2||!Array.isArray(s.weights))return null
 const p=s.controlPoints,rows=p.length,columns=p[0].length
 if(!p.every(row=>Array.isArray(row)&&row.length===columns&&row.every(point=>Array.isArray(point)&&point.length===3))||s.weights.length!==rows||!s.weights.every(row=>Array.isArray(row)&&row.length===columns)||(s.periodicU!=null&&typeof s.periodicU!=='boolean')||(s.periodicV!=null&&typeof s.periodicV!=='boolean'))return null
 const values=[s.degreeU,s.degreeV,s.knotsU.length,...s.knotsU,s.knotsV.length,...s.knotsV,rows,columns,...p.flat(2),...s.weights.flat(),Number(s.periodicU??false),Number(s.periodicV??false)]
 return values.every(Number.isFinite)?values:null
}
function validQuotient(expected:ReturnType<typeof quotientExpectation>,q:QuotientProof,remaining:number):boolean{
 if(!expected||remaining<256||q.collapsedEnd!==expected.collapsedEnd||q.poleEdge!==expected.poleEdge||q.poleVertex!==expected.poleVertex||typeof q.proven!=='boolean'||!Number.isSafeInteger(q.cells)||q.cells<0||q.cells>256||q.cells>remaining)return false
 const evidence=surfaceEvidence(q.sourceSurface)
 if(!evidence||!expected.sourceEvidence||evidence.length!==expected.sourceEvidence.length||!evidence.every((n,k)=>n===expected.sourceEvidence![k]))return false
 if(expected.sourceFrame===null){if(q.sourceFrame!==null)return false}
 else if(!Array.isArray(q.sourceFrame)||q.sourceFrame.length!==4||!q.sourceFrame.every((row,j)=>Array.isArray(row)&&row.length===3&&row.every((n,k)=>Number.isFinite(n)&&n===expected.sourceFrame![j][k])))return false
 if(q.cells===0)return !q.proven&&['unsupported-chart','boundary-not-collapsed','weighted-order-not-proven','frame-not-separated'].includes(q.reason)&&q.weightedBounds===null&&q.dominanceMarginLower===null&&q.bandMarginsLower===null
 if(q.reason==='weight-not-separated')return !q.proven&&q.weightedBounds===null&&q.dominanceMarginLower===null&&q.bandMarginsLower===null
 if(q.cells!==256||!Array.isArray(q.weightedBounds)||q.weightedBounds.length!==4||!q.weightedBounds.every(Number.isFinite)||typeof q.dominanceMarginLower!=='number'||!Number.isFinite(q.dominanceMarginLower))return false
 const [a,b,e,c]=q.weightedBounds,margin=q.dominanceMarginLower
 if(e<0||c<0)return false
 if(q.bandMarginsLower!==null&&(!Array.isArray(q.bandMarginsLower)||q.bandMarginsLower.length!==16||!q.bandMarginsLower.every(Number.isFinite)||Math.min(...q.bandMarginsLower)!==margin))return false
 if(q.proven){
  if(a<=0||b<=0||margin<=0)return false
  if(q.reason==='global-weighted-quotient-dominance')return q.bandMarginsLower===null&&margin<=a*b-c*e
  return q.reason==='global-localized-weighted-quotient-dominance'&&q.bandMarginsLower!==null&&q.bandMarginsLower.every(m=>m>0)
 }
 return q.reason==='weighted-projection-not-proven'&&(a<=0||b<=0||margin<=0)
}
function polarCandidates(s:NurbsBrep['faces'][number]['surface']):number[][][]{
 const p=s.controlPoints,w=s.weights
 if(s.degreeU!==2||p.length!==3||s.degreeV>8||!s.knotsU.slice(0,3).every(k=>k===s.knotsU[2])||!s.knotsU.slice(3).every(k=>k===s.knotsU[3])||!s.knotsV.slice(0,s.degreeV+1).every(k=>k===s.knotsV[s.degreeV]))return []
 const dot=(a:number[],b:number[])=>a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
 const cross=(a:number[],b:number[])=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
 const first=p[0][0],last=p[2][0],weights=[w[0][0],2*w[1][0],w[2][0]],denominator=weights[0]+weights[1]+weights[2]
 const middle=first.map((n,k)=>(n*weights[0]+p[1][0][k]*weights[1]+last[k]*weights[2])/denominator)
 const a=middle.map((n,k)=>n-first[k]),b=last.map((n,k)=>n-first[k]),n=cross(a,b),squared=dot(n,n)
 if(!Number.isFinite(squared)||squared===0)return []
 const an=dot(a,a),bn=dot(b,b),bx=cross(b,n),nx=cross(n,a)
 const center=first.map((v,k)=>v+(an*bx[k]+bn*nx[k])/(2*squared)),radial=middle.map((v,k)=>v-center[k]),radius=Math.sqrt(dot(radial,radial)),normal=Math.sqrt(dot(n,n))
 if(!center.every(Number.isFinite)||!Number.isFinite(radius)||radius===0||!Number.isFinite(normal)||normal===0)return []
 const x=radial.map(v=>v/radius),z=n.map(v=>v/normal),y=cross(z,x),affine=(d:number[])=>[...d,-dot(d,center)]
 return [1,-1].map(sign=>[affine(x),affine(y),affine(z.map(v=>v*sign))]).filter(c=>c.flat().every(Number.isFinite))
}
const linearProjections=[[[1,1,0],[0,0,1]],[[1,-1,0],[0,0,1]],[[1,0,1],[0,1,0]],[[1,0,-1],[0,1,0]],[[0,1,1],[1,0,0]],[[0,1,-1],[1,0,0]]]
export function validSelfIntersection(e:ReturnType<typeof selfIntersectionExpectation>,value:unknown):value is SelfIntersection {
 const r=value as SelfIntersection
 if(!r||r.scope!=='within-face-and-distinct-face-pairs'||r.maxSpans!==e.maxSpans||!Array.isArray(r.faces)||r.faces.length!==e.domains.length||!validFaceContacts(e,{...r,scope:'distinct-face-pairs'}))return false
 let used=0,all=true
 for(let i=0;i<r.faces.length;i++){
  const f=r.faces[i];if(!f||f.face!==i)return false
  const x=f.result
  if(used===e.maxSpans){if(x!==null||f.quotientProof!=null)return false;all=false;continue}
  if(!x||typeof x.proven!=='boolean'||!Number.isSafeInteger(x.spans)||x.spans<0)return false
  const periodic=e.domains[i].periodic,pole=e.collapsed[i],remaining=e.maxSpans-used,base=periodic||pole?0:Math.min(e.faceSpans[i],remaining)
  if(x.spans<base||x.spans>remaining)return false
  used+=x.spans
  if(x.proven){
   if(periodic||pole||typeof x.contractionUpper!=='number'||!Number.isFinite(x.contractionUpper)||x.contractionUpper<0||x.contractionUpper>=1)return false
   if(x.reason!=='global-polar-projection-contraction'&&x.polarProjection!=null)return false
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
    const candidate=[2,2,0,0,1,1].findIndex((axis,c)=>{
     const free=axis===2?[0,1]:axis===0?[1,2]:[0,2]
     const sign=c%2===0?1:-1,data=e.projectiveData[i]
     return basis.every((row,j)=>row.every((n,k)=>n===(j<2?(k===3?-data[free[j]][0]:k===free[j]?1:0):k===3?data[axis][2]-sign*data[axis][1]:k===axis?sign:0)))
    })
    if(candidate<0||x.spans!==e.faceSpans[i]*(97+16*(candidate+1)))return false
   }else if(x.reason==='global-polar-projection-contraction'){
    const basis=x.polarProjection
    if(x.projection!==null||x.linearProjection!==null||x.projectiveProjection!==null||!Array.isArray(basis)||basis.length!==3||!basis.every(row=>Array.isArray(row)&&row.length===4&&row.every(Number.isFinite)))return false
    const candidate=e.polarCandidates[i].findIndex(c=>c.every((row,j)=>row.every((n,k)=>n===basis[j][k])))
    if(candidate<0||x.spans!==e.faceSpans[i]*(193+256*(candidate+1)))return false
   }else return false
  }else{
   if(x.projection!==null||x.linearProjection!=null||x.projectiveProjection!=null||x.polarProjection!=null||x.contractionUpper!==null)return false
   if(periodic){if(x.spans!==0||x.reason!=='periodic-domain')return false}
   else if(pole){if(x.spans!==0||x.reason!=='collapsed-boundary-requires-quotient-proof')return false}
   else if(x.reason==='work-limit'){if(x.spans!==remaining)return false}
   else if(x.reason==='projection-not-proven'){
    const previous=e.faceSpans[i]*(x.projectiveProjection!==undefined?193:x.linearProjection===undefined?1:97)
    const polar=x.polarProjection===undefined?0:Math.min(e.polarCandidates[i].length,Math.max(0,Math.floor((remaining-previous)/(e.faceSpans[i]*256))))
    const expected=previous+polar*e.faceSpans[i]*256
    if(x.spans!==expected)return false
   }else return false
  }
  if(f.quotientProof!=null){
   if(x.proven||periodic||!pole||!validQuotient(e.quotients[i],f.quotientProof,e.maxSpans-used))return false
   used+=f.quotientProof.cells
  }
  all=all&&faceAbsenceProven(f)
 }
 return r.spans===used&&r.allFacesInjective===all&&r.absenceProven===(all&&r.allPairsClassified)
}
export function inspectSelfIntersection(model:NurbsBrep,toleranceUv:number,limits:FaceContactLimits,maxSpans:number):SelfIntersection {
 return callGeometryRust<SelfIntersection>('cad_self_intersection',{model,toleranceUv,...limits,maxSpans})
}
