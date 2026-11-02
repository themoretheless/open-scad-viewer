import type {NurbsCurve} from './nurbsCurve'
import type {NurbsSurface} from './nurbsSurface'
import type {ParameterBox3} from './surfaceIsoTargetProtocol'
export const MAX_PATCH_EXACT_WORK=1_000_000
export interface OriginalPatchContactsQuery {curve:NurbsCurve;patch:NurbsSurface;tolerance:number;maxSpans:number;maxExactWork:number}
export interface OriginalPatchContactsRequest extends OriginalPatchContactsQuery {type:'original-patch-contacts/1';id:string;module:WebAssembly.Module}
export interface OriginalPatchContactsReport {
 schema:'nurbs-original-affine-patch-contacts/1'
 status:'present'|'absent-on-patch'|'unresolved'
 presenceProven:boolean;patchAbsenceProven:boolean;curvePatchCoverageComplete:boolean
 parameterCoordinates:['curve-natural-t','patch-natural-u','patch-natural-v']
 targetWeightsSeparable?:boolean;targetUvInjectivityProven?:boolean
 contacts:{method:'original-patch-plane-polynomial';uvMethod?:'separable-projective'|'injective-bilinear-krawczyk'|'positive-bilinear-global-inverse'|'exact-boundary-projective'|'original-affine-coordinate-bounds'|'original-polynomial-common-root'|'original-polynomial-coordinate-sign';parameterBox:ParameterBox3;positionBounds:ParameterBox3;sourceSpans:number[];multiplicities:number[]}[]
 scalarProofs:{span:number;status:'roots'|'coincident'|'indeterminate';normalizedOriginalSpanRoots:{interval:[number,number];multiplicity:number}[];reason:string|null;workUsed:number}[]
 proofContextSerialized:false
 unresolvedSpans:number[];ambiguousParameters:[number,number][]
 spans:number;maxSpans:number;exactWork:number;maxExactWork:number
 allSurfaceBranchesEnumerated:false;trimAdmission:false;bodyAdmission:false
}
const integer=(x:unknown,lo:number,hi:number):x is number=>typeof x==='number'&&Number.isInteger(x)&&x>=lo&&x<=hi
const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&[x[0],x[1]].every(v=>typeof v==='number'&&Number.isFinite(v))&&x[0]<=x[1]
const box=(x:unknown):x is ParameterBox3=>Array.isArray(x)&&x.length===3&&[0,1,2].every(i=>interval(x[i]))
const reasons=['PrecisionExhausted','ResourceLimit','Cancelled','DeadlineExceeded','MissingProof']
/** Structural evidence validation, not a portable exact-predicate certificate. */
export function isOriginalPatchContactsReport(value:unknown):value is OriginalPatchContactsReport {
 if(!value||typeof value!=='object')return false
 const r=value as OriginalPatchContactsReport
 if(r.schema!=='nurbs-original-affine-patch-contacts/1'||!['present','absent-on-patch','unresolved'].includes(r.status)
  ||r.proofContextSerialized!==false||r.allSurfaceBranchesEnumerated!==false||r.trimAdmission!==false||r.bodyAdmission!==false
  ||!integer(r.maxSpans,1,100000)||!integer(r.spans,0,Math.min(r.maxSpans,256))||!integer(r.maxExactWork,0,MAX_PATCH_EXACT_WORK)||!integer(r.exactWork,0,r.maxExactWork)
  ||!Array.isArray(r.parameterCoordinates)||r.parameterCoordinates.length!==3||[0,1,2].some(i=>r.parameterCoordinates[i]!==['curve-natural-t','patch-natural-u','patch-natural-v'][i])
  ||!Array.isArray(r.contacts)||r.contacts.length>6400||!Array.isArray(r.scalarProofs)||r.scalarProofs.length!==r.spans
  ||!Array.isArray(r.unresolvedSpans)||r.unresolvedSpans.length>256||!Array.from(r.unresolvedSpans).every(i=>integer(i,1,255))||new Set(r.unresolvedSpans).size!==r.unresolvedSpans.length
  ||!Array.isArray(r.ambiguousParameters)||r.ambiguousParameters.length>6400||!Array.from(r.ambiguousParameters).every(interval)
  ||typeof r.curvePatchCoverageComplete!=='boolean'||(r.curvePatchCoverageComplete&&r.spans===0)||r.curvePatchCoverageComplete!==(r.unresolvedSpans.length===0&&r.ambiguousParameters.length===0)
  ||r.presenceProven!==(r.contacts.length>0)||r.patchAbsenceProven!==(r.curvePatchCoverageComplete&&r.contacts.length===0)
  ||r.status!==(r.presenceProven?'present':r.patchAbsenceProven?'absent-on-patch':'unresolved'))return false
 if(r.targetWeightsSeparable!==undefined||r.targetUvInjectivityProven!==undefined){
  if(typeof r.targetWeightsSeparable!=='boolean'||typeof r.targetUvInjectivityProven!=='boolean'||(r.targetWeightsSeparable&&!r.targetUvInjectivityProven)||(r.presenceProven&&!r.targetUvInjectivityProven))return false
 }
 const proofs=new Map<number,OriginalPatchContactsReport['scalarProofs'][number]>()
 let work=0
 for(const p of r.scalarProofs){
  if(!p||!integer(p.span,1,255)||proofs.has(p.span)||!['roots','coincident','indeterminate'].includes(p.status)||!integer(p.workUsed,0,r.maxExactWork)||!Array.isArray(p.normalizedOriginalSpanRoots)||p.normalizedOriginalSpanRoots.length>25)return false
  if(p.status==='indeterminate'){if(typeof p.reason!=='string'||!reasons.includes(p.reason)||p.normalizedOriginalSpanRoots.length!==0||!r.unresolvedSpans.includes(p.span))return false}
  else if(p.reason!==null)return false
  if(p.status==='coincident'&&(p.normalizedOriginalSpanRoots.length!==0||!r.unresolvedSpans.includes(p.span)))return false
  for(const root of p.normalizedOriginalSpanRoots)if(!root||!interval(root.interval)||root.interval[0]<0||root.interval[1]>1||!integer(root.multiplicity,1,25))return false
  proofs.set(p.span,p);work+=p.workUsed
 }
 if(work!==r.exactWork)return false
 for(const c of r.contacts){
  if(!c||c.method!=='original-patch-plane-polynomial'||Object.hasOwn(c,'crossingSign')||!box(c.parameterBox)||!box(c.positionBounds)
   ||!Array.isArray(c.sourceSpans)||c.sourceSpans.length<1||c.sourceSpans.length>256||!Array.isArray(c.multiplicities)||c.multiplicities.length!==c.sourceSpans.length
   ||new Set(c.sourceSpans).size!==c.sourceSpans.length)return false
  if(c.uvMethod!==undefined&&!['separable-projective','injective-bilinear-krawczyk','positive-bilinear-global-inverse','exact-boundary-projective','original-affine-coordinate-bounds','original-polynomial-common-root','original-polynomial-coordinate-sign'].includes(c.uvMethod))return false
  if(c.uvMethod==='injective-bilinear-krawczyk'&&(r.targetWeightsSeparable!==false||r.targetUvInjectivityProven!==true))return false
  if((c.uvMethod==='positive-bilinear-global-inverse'||c.uvMethod==='exact-boundary-projective'||c.uvMethod==='original-affine-coordinate-bounds'||c.uvMethod==='original-polynomial-common-root'||c.uvMethod==='original-polynomial-coordinate-sign')&&r.targetUvInjectivityProven!==true)return false
  if(r.targetWeightsSeparable===false&&!['injective-bilinear-krawczyk','positive-bilinear-global-inverse','exact-boundary-projective','original-affine-coordinate-bounds','original-polynomial-common-root','original-polynomial-coordinate-sign'].includes(c.uvMethod??''))return false
  if(c.uvMethod==='separable-projective'&&r.targetWeightsSeparable===false)return false
  for(let i=0;i<c.sourceSpans.length;i++){
   const span=c.sourceSpans[i],multiplicity=c.multiplicities[i]
   if(!integer(span,1,255)||!integer(multiplicity,1,25))return false
   const proof=proofs.get(span)
   if(proof?.status!=='roots'||!proof.normalizedOriginalSpanRoots.some(root=>root.multiplicity===multiplicity))return false
  }
 }
 return true
}
