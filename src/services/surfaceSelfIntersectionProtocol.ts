import type {NurbsSurface} from './nurbsSurface'
export type ExactUvScalar={numerator:string;denominator:string}
export type ExactUvPair=[[ExactUvScalar,ExactUvScalar],[ExactUvScalar,ExactUvScalar]]
export interface SurfaceSelfIntersectionReport {
 schema:'nurbs-surface-self-intersection/1'
 status:'absent'|'present'|'unresolved'
 absenceProven:boolean
 presenceProven:boolean
 allContactBranchesEnumerated:false
 witness:[[number,number],[number,number]]|null
 exactWitness:ExactUvPair|null
 witnessCoordinates:'normalized-original-active-domain'
 reflectedAxis:0|1|null
 injectivity:{proven:boolean;spans:number;reason:string}
 maxSpans:number
}
export interface SurfaceAuditRequest {type:'surface-self-audit/1';id:string;module:WebAssembly.Module;surface:NurbsSurface;maxSpans:number}
export function isSurfaceAuditReport(value:unknown):value is SurfaceSelfIntersectionReport {
 if(!value||typeof value!=='object')return false
 const r=value as SurfaceSelfIntersectionReport
 if(r.schema!=='nurbs-surface-self-intersection/1'||!['absent','present','unresolved'].includes(r.status)
  ||r.absenceProven!==(r.status==='absent')||r.presenceProven!==(r.status==='present')
  ||r.allContactBranchesEnumerated!==false||r.witnessCoordinates!=='normalized-original-active-domain'
  ||![null,0,1].includes(r.reflectedAxis)||!Number.isInteger(r.maxSpans)||r.maxSpans<1||r.maxSpans>100000)return false
 if(!r.injectivity||typeof r.injectivity.proven!=='boolean'||!Number.isInteger(r.injectivity.spans)
  ||r.injectivity.spans<0||r.injectivity.spans>r.maxSpans||typeof r.injectivity.reason!=='string'||r.injectivity.reason.length>1024)return false
 if(r.injectivity.proven!==(r.status==='absent'))return false
 if(r.status!=='present')return r.witness===null&&r.exactWitness===null
 const exact=r.exactWitness
 if(!Array.isArray(exact)||exact.length!==2||!exact.every(uv=>Array.isArray(uv)&&uv.length===2&&uv.every(x=>
  x&&typeof x==='object'&&typeof x.numerator==='string'&&typeof x.denominator==='string'
  &&x.numerator.length<=4096&&x.denominator.length<=4096&&/^[1-9]\d*$/.test(x.numerator)&&/^[1-9]\d*$/.test(x.denominator))))return false
 return r.witness===null||(Array.isArray(r.witness)&&r.witness.length===2&&r.witness.every(uv=>Array.isArray(uv)&&uv.length===2&&uv.every(x=>typeof x==='number'&&Number.isFinite(x)&&x>0&&x<1)))
}
