import {MAX_QUADRIC_EXACT_WORK,isOriginalQuadricCarrierReport,type OriginalQuadricCarrierQuery,type OriginalQuadricCarrierReport} from './originalQuadricCarrierProtocol.ts'
import type {NurbsSurface} from './nurbsSurface'
import type {ParameterBox3} from './surfaceIsoTargetProtocol'
export interface OriginalQuadricGraphQuery extends OriginalQuadricCarrierQuery {patch:NurbsSurface}
export interface OriginalQuadricGraphRequest extends OriginalQuadricGraphQuery {type:'original-quadric-graph-contacts/1';id:string;module:WebAssembly.Module}
export interface OriginalQuadricGraphReport {
 schema:'nurbs-original-quadric-graph-contacts/1';status:'present-on-patch'|'absent-on-patch'|'unresolved'
 presenceProven:boolean;patchAbsenceProven:boolean;curvePatchCoverageComplete:boolean;targetGraphProven:boolean
 targetMembership:{status:'equal'|'different'|'indeterminate';reason:string|null;workUsed:number}
 carrier:OriginalQuadricCarrierReport|null
 contacts:{method:'original-quadric-graph';uvMethod:'exact-singleton'|'original-affine-control-bounds'|'quadric-coordinate-sign';parameterBox:ParameterBox3;positionBounds:ParameterBox3;sourceSpans:number[];multiplicities:number[];normalizedOriginalSpanParameters:[number,number][]}[]
 ambiguousParameters:[number,number][];parameterCoordinates:['curve-natural-t','patch-natural-u','patch-natural-v']
 boundaryProofs:{span:number;normalizedRoot:[number,number];edge:number;coordinateSign:-1|0|1|null;reason:string|null;workUsed:number}[]
 boundaryExactWork:number;exactWork:number;maxExactWork:number;maxSpans:number
 proofContextSerialized:false;finiteSurfaceUvAdmission:boolean;allSurfaceBranchesEnumerated:false;trimAdmission:false;bodyAdmission:false
}
const integer=(x:unknown,lo:number,hi:number):x is number=>typeof x==='number'&&Number.isInteger(x)&&x>=lo&&x<=hi
const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&[x[0],x[1]].every(v=>typeof v==='number'&&Number.isFinite(v))&&x[0]<=x[1]
const box=(x:unknown):x is ParameterBox3=>Array.isArray(x)&&x.length===3&&[0,1,2].every(i=>interval(x[i]))
const same=(a:unknown[],b:unknown[])=>a.length===b.length&&a.every((v,i)=>v===b[i])
const reasons=['PrecisionExhausted','ResourceLimit','Cancelled','DeadlineExceeded','MissingProof']
/** Native evidence structure/provenance validation, not a portable geometry proof. */
export function isOriginalQuadricGraphReport(value:unknown):value is OriginalQuadricGraphReport {
 if(!value||typeof value!=='object')return false
 const r=value as OriginalQuadricGraphReport
 if(r.schema!=='nurbs-original-quadric-graph-contacts/1'||!['present-on-patch','absent-on-patch','unresolved'].includes(r.status)
  ||r.proofContextSerialized!==false||r.allSurfaceBranchesEnumerated!==false||r.trimAdmission!==false||r.bodyAdmission!==false
  ||!integer(r.maxSpans,1,100000)||!integer(r.maxExactWork,0,MAX_QUADRIC_EXACT_WORK)||!integer(r.exactWork,0,r.maxExactWork)||!integer(r.boundaryExactWork,0,r.exactWork)
  ||!Array.isArray(r.parameterCoordinates)||!same(r.parameterCoordinates,['curve-natural-t','patch-natural-u','patch-natural-v'])
  ||!Array.isArray(r.contacts)||r.contacts.length>12800||!Array.isArray(r.ambiguousParameters)||r.ambiguousParameters.length>12800||!Array.from(r.ambiguousParameters).every(interval)
  ||!Array.isArray(r.boundaryProofs)||r.boundaryProofs.length>51200||typeof r.targetGraphProven!=='boolean'||typeof r.curvePatchCoverageComplete!=='boolean'
  ||r.presenceProven!==(r.contacts.length>0)||r.finiteSurfaceUvAdmission!==r.presenceProven||r.patchAbsenceProven!==(r.curvePatchCoverageComplete&&r.contacts.length===0)
  ||r.status!==(r.presenceProven?'present-on-patch':r.patchAbsenceProven?'absent-on-patch':'unresolved'))return false
 const t=r.targetMembership
 if(!t||!['equal','different','indeterminate'].includes(t.status)||!integer(t.workUsed,0,r.exactWork)||r.targetGraphProven!==(t.status==='equal'))return false
 if(t.status==='indeterminate'){if(typeof t.reason!=='string'||!reasons.includes(t.reason))return false}else if(t.reason!==null)return false
 if(!r.targetGraphProven){
  return r.carrier===null&&r.contacts.length===0&&r.ambiguousParameters.length===0&&r.boundaryProofs.length===0&&r.boundaryExactWork===0&&!r.curvePatchCoverageComplete&&r.exactWork===t.workUsed
 }
 const carrier=r.carrier
 if(!isOriginalQuadricCarrierReport(carrier)||carrier.maxSpans!==r.maxSpans||carrier.maxExactWork!==r.maxExactWork-t.workUsed
  ||r.exactWork!==t.workUsed+carrier.exactWork+r.boundaryExactWork||r.curvePatchCoverageComplete!==(carrier.curveCarrierCoverageComplete&&r.ambiguousParameters.length===0))return false
 const key=(p:{sourceSpans:number[];multiplicities:number[];normalizedOriginalSpanParameters?:[number,number][]})=>JSON.stringify([p.sourceSpans,p.multiplicities,p.normalizedOriginalSpanParameters])
 const rootMap=new Map<string,OriginalQuadricCarrierReport['roots'][number]>()
 for(const root of carrier.roots){if(!root.normalizedOriginalSpanParameters||rootMap.has(key(root)))return false;rootMap.set(key(root),root)}
 const boundaryKeys=new Set<string>()
 let work=0
 for(const p of r.boundaryProofs){
  if(!p||!integer(p.span,1,255)||!integer(p.edge,0,3)||!interval(p.normalizedRoot)||p.normalizedRoot[0]<0||p.normalizedRoot[1]>1||!integer(p.workUsed,0,r.boundaryExactWork))return false
  if(p.coordinateSign===null){if(typeof p.reason!=='string'||!reasons.includes(p.reason))return false}
  else if(!integer(p.coordinateSign,-1,1)||p.reason!==null)return false
  const scalar=carrier.scalarProofs.find(s=>s.span===p.span)
  if(scalar?.status!=='roots'||!scalar.normalizedOriginalSpanRoots.some(root=>same(root.interval,p.normalizedRoot)))return false
  const k=JSON.stringify([p.span,p.normalizedRoot,p.edge]);if(boundaryKeys.has(k))return false;boundaryKeys.add(k)
  work+=p.workUsed
 }
 if(work!==r.boundaryExactWork)return false
 const claimedRoots=new Set<string>()
 for(const c of r.contacts){
  if(!c||c.method!=='original-quadric-graph'||!['exact-singleton','original-affine-control-bounds','quadric-coordinate-sign'].includes(c.uvMethod)||Object.hasOwn(c,'crossingSign')||!box(c.parameterBox)||!box(c.positionBounds)
   ||!Array.isArray(c.sourceSpans)||c.sourceSpans.length===0||c.sourceSpans.length>256||!Array.isArray(c.multiplicities)||c.multiplicities.length!==c.sourceSpans.length
   ||!Array.isArray(c.normalizedOriginalSpanParameters)||c.normalizedOriginalSpanParameters.length!==c.sourceSpans.length||!Array.from(c.normalizedOriginalSpanParameters).every(interval))return false
  const rootKey=key(c),root=rootMap.get(rootKey)
  if(claimedRoots.has(rootKey))return false;claimedRoots.add(rootKey)
  if(!root||root.parameter[0]>c.parameterBox[0][1]||c.parameterBox[0][0]>root.parameter[1])return false
  const normalized=root?.normalizedOriginalSpanParameters
  if(!root||!normalized||normalized.length!==c.sourceSpans.length)return false
  if(c.uvMethod==='exact-singleton'){if(normalized[0]![0]!==normalized[0]![1])return false}
  else if(normalized[0]![0]===normalized[0]![1])return false
  if(c.uvMethod==='quadric-coordinate-sign'){
   const relevant=r.boundaryProofs.filter(p=>{const i=c.sourceSpans.indexOf(p.span);return i>=0&&same(p.normalizedRoot,normalized[i]!)})
   if(!relevant.some(p=>p.coordinateSign!==null)||relevant.some(p=>p.coordinateSign!==null&&((p.edge%2===0&&p.coordinateSign<0)||(p.edge%2===1&&p.coordinateSign>0))))return false
  }
 }
 return true
}
