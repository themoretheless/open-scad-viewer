// Transport only: native full-Hermite identities own support G1 admission.
export interface SupportTangencyRequest {controls:number;cells:number;spans:number}
type Rect=[[number,number],[number,number]]
export interface SupportTangencyReport {
 version:1;qualified:boolean;basis:'owned-full-Hermite-original-profile-support'
 neighborBodyQualified:false;solidQualified:false;cells:number;spans:number;reason:string
 boundaries:{index:number;kind:'section'|'rail';high:boolean;qualified:boolean;regular:Rect[];pending:Rect[];cells:number;spans:number;reason:string}[]
}
const count=(v:unknown,max:number)=>Number.isSafeInteger(v)&&(v as number)>=0&&(v as number)<=max
export function validSupportTangencyRequest(value:unknown):value is SupportTangencyRequest {
 if(!value||typeof value!=='object')return false
 const v=value as SupportTangencyRequest
 return count(v.controls,1000000)&&count(v.cells,100000)&&count(v.spans,1000000)
}
const rect=(v:unknown)=>Array.isArray(v)&&v.length===2&&v.every(d=>Array.isArray(d)&&d.length===2&&d.every(x=>typeof x==='number'&&Number.isFinite(x))&&d[0]<=d[1])
export function validSupportTangencyReport(request:SupportTangencyRequest,value:unknown):value is SupportTangencyReport {
 if(!value||typeof value!=='object')return false
 const v=value as SupportTangencyReport
 if(v.version!==1||v.basis!=='owned-full-Hermite-original-profile-support'||v.neighborBodyQualified!==false||v.solidQualified!==false
  ||typeof v.qualified!=='boolean'||!v.reason||typeof v.reason!=='string'||!count(v.cells,request.cells)||!count(v.spans,request.spans)
  ||!Array.isArray(v.boundaries)||v.boundaries.length>4)return false
 const qualifiedReason='profile-transition-support-tangency-qualified'
 if(v.qualified!==(v.reason===qualifiedReason))return false
 let cells=0,spans=0
 for(const [i,b] of v.boundaries.entries()) {
  if(!b||b.index!==i||b.kind!==(i<2?'section':'rail')||b.high!==(i%2===1)||typeof b.qualified!=='boolean'
   ||typeof b.reason!=='string'||!b.reason||!count(b.cells,request.cells)||!count(b.spans,request.spans)
   ||!Array.isArray(b.regular)||!Array.isArray(b.pending)||b.regular.length+b.pending.length>2*request.cells+1
   ||!b.regular.every(rect)||!b.pending.every(rect)||b.qualified!==(b.pending.length===0&&b.regular.length>0))return false
  const boundaryQualifiedReason=i<2?'full-section-join-qualified':'full-side-join-qualified'
  if(b.qualified!==(b.reason===boundaryQualifiedReason))return false
  cells+=b.cells;spans+=b.spans
 }
 if(cells!==v.cells||spans!==v.spans)return false
 if((request.controls===0||request.cells===0||request.spans===0)&&v.boundaries.length!==0)return false
 return v.qualified===(v.boundaries.length===4&&v.boundaries.every(b=>b.qualified))
}
