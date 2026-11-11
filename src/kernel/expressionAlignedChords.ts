// Transport only. Rust owns support-product coverage and its certificate.
export interface AlignedChordRequest {
 minimumMm:number;maxSineSquared:number;toleranceUv:number
 controls:number;cells:number;spans:number;domainCells:number
}
type Rect=[[number,number],[number,number]]
export interface AlignedChordReport {
 version:1;qualified:boolean;wholeThicknessQualified:false;solidQualified:false
 report:{method:'expression-support-aligned-chord-coverage';scope:'original-retained-transition-support-products'
 minimum_mm:number;max_sine_squared:number;tolerance_uv:number;complete:boolean
 span_stages?:{position:number;normal:number;shared_corner:number;original_pair:number;retained_pair:number;corrected_pair:number}
 cells:number;spans:number;domain_cells:number;total_pairs:number;initial_pairs_checked:number
 intrinsic_checked:number;intrinsic_spans:number;intrinsic_unresolved:number
 profile_transition_checked:number;profile_transition_excluded:number;profile_transition_spans:number
 profile_transition_inconclusive?:number;profile_transition_unresolved?:number
 distance_excluded:number;angle_excluded:number;planar_excluded:number;intrinsic_excluded:number
 plane_controls:number;cache_hits:number;symmetry_reused:number;trim_excluded:number
 budget_pending:number;enclosure_pending:number;resolution_pending:number
 pending:{faces:[number,number];domains:[Rect,Rect];pending_reason:'budget'|'enclosure'|'resolution'}[]}
}
const count=(v:unknown,max=Number.MAX_SAFE_INTEGER):v is number=>Number.isSafeInteger(v)&&(v as number)>=0&&(v as number)<=max
const rect=(v:unknown):v is Rect=>Array.isArray(v)&&v.length===2&&v.every(d=>Array.isArray(d)&&d.length===2&&d.every(x=>typeof x==='number'&&Number.isFinite(x))&&d[0]<=d[1])
/** Parameter diagnostics only; uses the same bounds as request admission. */
export function alignedChordParameterErrors(value:unknown){
 const r=(value&&typeof value==='object'?value:{}) as Partial<AlignedChordRequest>
 return {
  minimumMm:typeof r.minimumMm!=='number'||!Number.isFinite(r.minimumMm)||r.minimumMm<=0,
  maxSineSquared:typeof r.maxSineSquared!=='number'||!Number.isFinite(r.maxSineSquared)||r.maxSineSquared<0||r.maxSineSquared>=1,
  toleranceUv:typeof r.toleranceUv!=='number'||!Number.isFinite(r.toleranceUv)||r.toleranceUv<=0||r.toleranceUv>1,
 }
}
export function validAlignedChordRequest(value:unknown):value is AlignedChordRequest {
 if(!value||typeof value!=='object')return false
 const r=value as AlignedChordRequest
 const errors=alignedChordParameterErrors(r)
 return !errors.minimumMm&&!errors.maxSineSquared&&!errors.toleranceUv
  &&count(r.controls,1000000)&&count(r.cells,100000)&&count(r.spans,1000000)&&count(r.domainCells,1000000)
}
export function validAlignedChordReport(request:AlignedChordRequest,faces:number,value:unknown):value is AlignedChordReport {
 if(!validAlignedChordRequest(request)||!count(faces)||faces<1||!value||typeof value!=='object')return false
 const v=value as AlignedChordReport,r=v.report
 if(v.version!==1||typeof v.qualified!=='boolean'||v.wholeThicknessQualified!==false||v.solidQualified!==false||!r
  ||r.method!=='expression-support-aligned-chord-coverage'||r.scope!=='original-retained-transition-support-products'
  ||r.minimum_mm!==request.minimumMm||r.max_sine_squared!==request.maxSineSquared||r.tolerance_uv!==request.toleranceUv
  ||typeof r.complete!=='boolean'||r.total_pairs!==faces*(faces+1)/2||!count(r.initial_pairs_checked,r.total_pairs)
  ||!count(r.cells,request.cells)||!count(r.spans,request.spans)||!count(r.domain_cells,request.domainCells))return false
 for(const k of ['intrinsic_checked','intrinsic_spans','intrinsic_unresolved','profile_transition_checked','profile_transition_excluded','profile_transition_spans','distance_excluded','angle_excluded','planar_excluded','intrinsic_excluded','plane_controls','cache_hits','symmetry_reused','trim_excluded','budget_pending','enclosure_pending','resolution_pending'] as const)if(!count(r[k]))return false
 if(r.initial_pairs_checked>r.cells
  ||r.distance_excluded+r.angle_excluded+r.planar_excluded+r.intrinsic_excluded+r.trim_excluded>r.cells
  ||r.intrinsic_spans+r.profile_transition_spans>r.spans||r.intrinsic_unresolved>r.intrinsic_checked
  ||r.profile_transition_excluded>r.profile_transition_checked||r.intrinsic_checked>r.cells||r.plane_controls>faces*request.controls)return false
 if(r.profile_transition_inconclusive!==undefined||r.profile_transition_unresolved!==undefined){
  if(!count(r.profile_transition_inconclusive)||!count(r.profile_transition_unresolved)
   ||r.profile_transition_excluded+r.profile_transition_inconclusive! + r.profile_transition_unresolved! !==r.profile_transition_checked)return false
 }
 if(r.span_stages!==undefined){
  const stages=r.span_stages
  if(!stages||typeof stages!=='object')return false
  let total=r.intrinsic_spans+r.profile_transition_spans
  for(const key of ['position','normal','shared_corner','original_pair','retained_pair','corrected_pair'] as const){
   if(!count(stages[key],r.spans))return false
   total+=stages[key]
  }
  if(total!==r.spans)return false
 }
 if(!Array.isArray(r.pending)||r.pending.length>2*r.cells+r.total_pairs)return false
 const reasons={budget:0,enclosure:0,resolution:0}
 for(const p of r.pending){
  if(!p||!Array.isArray(p.faces)||p.faces.length!==2||!p.faces.every(f=>count(f,faces-1))||p.faces[0]>p.faces[1]
   ||!Array.isArray(p.domains)||p.domains.length!==2||!p.domains.every(rect)||!Object.hasOwn(reasons,p.pending_reason))return false
  reasons[p.pending_reason]++
 }
 return reasons.budget===r.budget_pending&&reasons.enclosure===r.enclosure_pending&&reasons.resolution===r.resolution_pending
  &&r.complete===(r.pending.length===0)&&v.qualified===r.complete
  &&(!v.qualified||r.initial_pairs_checked===r.total_pairs)
}
