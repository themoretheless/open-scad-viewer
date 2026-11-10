import type {NurbsCurve} from '../kernel/nurbsCurve'
import type {ParameterBox3} from './surfaceIsoTargetProtocol'
export const MAX_QUADRIC_EXACT_WORK=1_000_000
export type QuadricMatrix=[[number,number,number,number],[number,number,number,number],[number,number,number,number],[number,number,number,number]]
export interface OriginalQuadricCarrierQuery {curve:NurbsCurve;quadric:QuadricMatrix;constantProducts:[number,number][];tolerance:number;maxSpans:number;maxExactWork:number}
export interface OriginalQuadricCarrierRequest extends OriginalQuadricCarrierQuery {type:'original-quadric-carrier-roots/1';id:string;module:WebAssembly.Module}
export interface OriginalQuadricCarrierReport {
 schema:'nurbs-original-quadric-carrier-roots/1';status:'present-on-carrier'|'absent-on-carrier'|'unresolved'
 presenceProven:boolean;carrierAbsenceProven:boolean;curveCarrierCoverageComplete:boolean
 parameterCoordinates:['curve-natural-t']
 roots:{method:'original-quadric-polynomial';parameter:[number,number];positionBounds:ParameterBox3;sourceSpans:number[];multiplicities:number[];normalizedOriginalSpanParameters?:[number,number][]}[]
 scalarProofs:{span:number;status:'roots'|'coincident'|'indeterminate';normalizedOriginalSpanRoots:{interval:[number,number];multiplicity:number}[];reason:string|null;workUsed:number}[]
 unresolvedSpans:number[];spans:number;maxSpans:number;exactWork:number;maxExactWork:number
 proofContextSerialized:false;finiteSurfaceUvAdmission:false;allSurfaceBranchesEnumerated:false;trimAdmission:false;bodyAdmission:false
}
const integer=(x:unknown,lo:number,hi:number):x is number=>typeof x==='number'&&Number.isInteger(x)&&x>=lo&&x<=hi
const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&[x[0],x[1]].every(v=>typeof v==='number'&&Number.isFinite(v))&&x[0]<=x[1]
const box=(x:unknown):x is ParameterBox3=>Array.isArray(x)&&x.length===3&&[0,1,2].every(i=>interval(x[i]))
const reasons=['PrecisionExhausted','ResourceLimit','Cancelled','DeadlineExceeded','MissingProof']
/** Structural validation of native evidence; this is not a portable proof certificate. */
export function isOriginalQuadricCarrierReport(value:unknown):value is OriginalQuadricCarrierReport {
 if(!value||typeof value!=='object')return false
 const r=value as OriginalQuadricCarrierReport
 if(r.schema!=='nurbs-original-quadric-carrier-roots/1'||!['present-on-carrier','absent-on-carrier','unresolved'].includes(r.status)
  ||r.proofContextSerialized!==false||r.finiteSurfaceUvAdmission!==false||r.allSurfaceBranchesEnumerated!==false||r.trimAdmission!==false||r.bodyAdmission!==false
  ||!integer(r.maxSpans,1,100000)||!integer(r.spans,0,Math.min(r.maxSpans,256))||!integer(r.maxExactWork,0,MAX_QUADRIC_EXACT_WORK)||!integer(r.exactWork,0,r.maxExactWork)
  ||!Array.isArray(r.parameterCoordinates)||r.parameterCoordinates.length!==1||r.parameterCoordinates[0]!=='curve-natural-t'
  ||!Array.isArray(r.roots)||r.roots.length>12800||!Array.isArray(r.scalarProofs)||r.scalarProofs.length!==r.spans
  ||!Array.isArray(r.unresolvedSpans)||r.unresolvedSpans.length>256||!Array.from(r.unresolvedSpans).every(i=>integer(i,1,255))||new Set(r.unresolvedSpans).size!==r.unresolvedSpans.length
  ||typeof r.curveCarrierCoverageComplete!=='boolean'||(r.curveCarrierCoverageComplete&&r.spans===0)||r.curveCarrierCoverageComplete!==(r.unresolvedSpans.length===0)
  ||r.presenceProven!==(r.roots.length>0)||r.carrierAbsenceProven!==(r.curveCarrierCoverageComplete&&r.roots.length===0)
  ||r.status!==(r.presenceProven?'present-on-carrier':r.carrierAbsenceProven?'absent-on-carrier':'unresolved'))return false
 let work=0
 const proofs=new Map<number,OriginalQuadricCarrierReport['scalarProofs'][number]>()
 for(const p of r.scalarProofs){
  if(!p||!integer(p.span,1,255)||proofs.has(p.span)||!integer(p.workUsed,0,r.maxExactWork)||!['roots','coincident','indeterminate'].includes(p.status)||!Array.isArray(p.normalizedOriginalSpanRoots)||p.normalizedOriginalSpanRoots.length>50)return false
  if(p.status==='indeterminate'){if(typeof p.reason!=='string'||!reasons.includes(p.reason)||p.normalizedOriginalSpanRoots.length!==0||!r.unresolvedSpans.includes(p.span))return false}
  else if(p.reason!==null)return false
  if(p.status==='coincident'&&(p.normalizedOriginalSpanRoots.length!==0||!r.unresolvedSpans.includes(p.span)))return false
  for(const root of p.normalizedOriginalSpanRoots)if(!root||!interval(root.interval)||root.interval[0]<0||root.interval[1]>1||!integer(root.multiplicity,1,50))return false
  proofs.set(p.span,p);work+=p.workUsed
 }
 if(work!==r.exactWork)return false
 for(const root of r.roots){
  if(!root||root.method!=='original-quadric-polynomial'||Object.hasOwn(root,'crossingSign')||!interval(root.parameter)||!box(root.positionBounds)
   ||!Array.isArray(root.sourceSpans)||root.sourceSpans.length<1||root.sourceSpans.length>256||new Set(root.sourceSpans).size!==root.sourceSpans.length
   ||!Array.isArray(root.multiplicities)||root.multiplicities.length!==root.sourceSpans.length)return false
  for(let i=0;i<root.sourceSpans.length;i++){
   const span=root.sourceSpans[i],multiplicity=root.multiplicities[i]
   if(!integer(span,1,255)||!integer(multiplicity,1,50))return false
   const p=proofs.get(span)
   if(root.normalizedOriginalSpanParameters!==undefined){
    const a=root.normalizedOriginalSpanParameters
    if(!Array.isArray(a)||a.length!==root.sourceSpans.length||!interval(a[i])||a[i]![0]<0||a[i]![1]>1||!p?.normalizedOriginalSpanRoots.some(r=>r.interval[0]===a[i]![0]&&r.interval[1]===a[i]![1]&&r.multiplicity===multiplicity))return false
   }
   if(p?.status!=='roots'||!p.normalizedOriginalSpanRoots.some(r=>r.multiplicity===multiplicity))return false
  }
 }
 return true
}

/** Scalar/options preflight; Rust owns original curve geometry validation. */
export function validQuadricQueryOptions(query:OriginalQuadricCarrierQuery):boolean {
 return !!query&&Number.isFinite(query.tolerance)&&query.tolerance>0&&Number.isInteger(query.maxSpans)&&query.maxSpans>=1&&query.maxSpans<=100000
  &&Number.isInteger(query.maxExactWork)&&query.maxExactWork>=0&&query.maxExactWork<=MAX_QUADRIC_EXACT_WORK
  &&Array.isArray(query.quadric)&&query.quadric.length===4&&[0,1,2,3].every(i=>{const row=query.quadric[i];return Array.isArray(row)&&row.length===4&&[0,1,2,3].every(j=>typeof row[j]==='number'&&Number.isFinite(row[j]))})
  &&Array.isArray(query.constantProducts)&&query.constantProducts.length<=16&&Array.from(query.constantProducts).every(p=>Array.isArray(p)&&p.length===2&&[p[0],p[1]].every(v=>typeof v==='number'&&Number.isFinite(v)))
}
