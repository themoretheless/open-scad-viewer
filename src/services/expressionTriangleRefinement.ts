// Transport checks only. Native Rust owns material, error and incidence proofs.
export interface TriangleRefinementLimits {
 maxDeviationMm:number;cells:number;spans:number;spansPerQuery:number;controls:number
 domainCells:number;domainCellsPerTriangle:number;toleranceUv:number;maxDepth:number;regionalSpans:number
}
export interface RefinementTriangle {face:number;vertices:number[];uv:number[][]}
export interface TriangleRefinementReport {
 version:1;limits:TriangleRefinementLimits;solidQualified:false;closedOrientedIncidence:boolean
 initialTriangles:number;initialChecked:number;splitTriangles:number;cells:number;spans:number;domainCells:number
 regional:{spans:number;queries:number;accepted:number}
 accepted:(RefinementTriangle & {deviationUpperMm:number})[]
 pending:(RefinementTriangle & {reason:string})[]
 numericFailures:(RefinementTriangle & {stage:string;code:string;message:string})[]
}
const count=(v:unknown):v is number=>Number.isSafeInteger(v)&&Number(v)>=0
const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)
export function validTriangleRefinementLimits(v:TriangleRefinementLimits):boolean {
 if(!v||typeof v!=='object')return false
 return finite(v.maxDeviationMm)&&v.maxDeviationMm>0&&finite(v.toleranceUv)&&v.toleranceUv>0
  &&(['cells','spans','spansPerQuery','domainCellsPerTriangle','regionalSpans'] as const).every(k=>count(v[k])&&v[k]<=100000)
  &&count(v.controls)&&v.controls<=1000000&&count(v.domainCells)&&v.domainCells<=1000000&&count(v.maxDepth)&&v.maxDepth<=32
}
export function validTriangleRefinement(limits:TriangleRefinementLimits,faces:number,points:(number[]|null)[],initial:number,value:unknown):value is TriangleRefinementReport {
 if(!value||typeof value!=='object')return false
 const v=value as TriangleRefinementReport
 if(v.version!==1||v.solidQualified!==false||typeof v.closedOrientedIncidence!=='boolean'||!validTriangleRefinementLimits(v.limits))return false
 if(Object.keys(limits).some(k=>v.limits[k as keyof TriangleRefinementLimits]!==limits[k as keyof TriangleRefinementLimits]))return false
 if(![v.initialTriangles,v.initialChecked,v.splitTriangles,v.cells,v.spans,v.domainCells].every(count)
  ||v.initialTriangles!==initial||v.initialChecked>initial||v.splitTriangles>v.cells||v.cells>limits.cells||v.spans>limits.spans||v.domainCells>limits.domainCells)return false
 const leaves=initial+2*v.splitTriangles
 if(!Array.isArray(v.accepted)||!Array.isArray(v.pending)||v.accepted.length+v.pending.length!==leaves||!Array.isArray(v.numericFailures)||v.numericFailures.length>3*limits.cells)return false
 const triangle=(t:RefinementTriangle)=>t&&count(t.face)&&t.face<faces&&Array.isArray(t.vertices)&&t.vertices.length===3&&new Set(t.vertices).size===3&&t.vertices.every(i=>count(i)&&i<points.length&&points[i]!==null)&&Array.isArray(t.uv)&&t.uv.length===3&&t.uv.every(p=>Array.isArray(p)&&p.length===2&&p.every(finite))
 const text=(s:unknown)=>typeof s==='string'&&s.length>0
 if(!v.accepted.every(t=>triangle(t)&&finite(t.deviationUpperMm)&&t.deviationUpperMm>=0&&t.deviationUpperMm<=limits.maxDeviationMm)
  ||!v.pending.every(t=>triangle(t)&&text(t.reason))||!v.numericFailures.every(t=>triangle(t)&&text(t.stage)&&text(t.code)&&text(t.message)))return false
 const addresses=[...v.accepted,...v.pending].map(t=>JSON.stringify([t.face,t.vertices,t.uv]))
 if(new Set(addresses).size!==leaves)return false
 const r=v.regional
 return !!r&&[r.spans,r.queries,r.accepted].every(count)&&r.spans<=limits.regionalSpans&&r.queries<=leaves&&r.accepted<=v.accepted.length
}
