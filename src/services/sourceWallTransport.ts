/** Transport checks only. Rust owns source geometry and material admission. */
export interface SourceWallOptions {
 groups:[number[],number[]];minimumMm:number;toleranceMm:number;toleranceUv:number;grid:number;maxAttempts:number
 limits:{gapCells:number;gapSpans:number;cells:number;domainCells:number;normalSpans:number;maxSineSquared:number}
}
export interface SourceWallResult {
 request:SourceWallOptions;qualified:boolean;converged:boolean;intervalMm:[number,number]|null;reason:string
 clearance:{reason:string;cells:number;spans:number;uncertainFaces:[number,number]|null;uncertainUv:[[number[],number[]],[number[],number[]]]|null}
 search:{attempts:number;refused:number;candidatesExhausted:boolean}
}
const count=(n:unknown,max:number)=>typeof n==='number'&&Number.isSafeInteger(n)&&n>=0&&n<=max
const interval=(v:unknown):v is [number,number]=>Array.isArray(v)&&v.length===2&&v.every(Number.isFinite)&&v[0]<=v[1]
export function wallExpectation(o:SourceWallOptions,faces:number):SourceWallOptions {
 if(!o||!Array.isArray(o.groups)||o.groups.length!==2||!o.groups.every(g=>Array.isArray(g)&&g.length>0&&g.every((n,i)=>count(n,faces-1)&&!g.slice(0,i).includes(n)))
  ||o.groups[0].some(n=>o.groups[1].includes(n))||![o.minimumMm,o.toleranceMm,o.toleranceUv].every(n=>Number.isFinite(n)&&n>0)
  ||!count(o.grid,8)||o.grid<1||!count(o.maxAttempts,256)||o.maxAttempts<1||!o.limits
  ||!['gapCells','gapSpans','normalSpans'].every(k=>count(o.limits[k as 'gapCells'],100000)&&o.limits[k as 'gapCells']>0)
  ||!count(o.limits.cells,1000000)||o.limits.cells<1||!count(o.limits.domainCells,8000000)||o.limits.domainCells<1
  ||!Number.isFinite(o.limits.maxSineSquared)||o.limits.maxSineSquared<0||o.limits.maxSineSquared>=1)throw new Error('Invalid source wall qualification request')
 return structuredClone(o)
}
function nextUp(n:number):number {
 const b=new DataView(new ArrayBuffer(8));b.setFloat64(0,n);b.setBigUint64(0,b.getBigUint64(0)+1n);return b.getFloat64(0)
}
export function validWall(e:SourceWallOptions|undefined,r:SourceWallResult|null|undefined,key:(v:unknown)=>string):boolean {
 if(!e)return r==null
 if(!r||key(r.request)!==key(e)||typeof r.qualified!=='boolean'||typeof r.converged!=='boolean')return false
 const c=r.clearance,s=r.search
 if(!c||!s||!count(c.cells,e.limits.gapCells)||!count(c.spans,e.limits.gapSpans)||!count(s.attempts,e.maxAttempts)
  ||!count(s.refused,s.attempts)||typeof s.candidatesExhausted!=='boolean')return false
 const total=e.groups[0].length*e.grid**2
 if(s.candidatesExhausted?s.attempts!==total:s.attempts!==e.maxAttempts||s.attempts>=total)return false
 if(c.reason==='source-face-gap-qualified'){
  if(c.uncertainFaces!==null||c.uncertainUv!==null)return false
 }else{
  if(!['source-face-gap-work-limit','source-face-gap-resolution-limit'].includes(c.reason)
   ||!Array.isArray(c.uncertainFaces)||c.uncertainFaces.length!==2
   ||!e.groups.every((g,i)=>g.includes(c.uncertainFaces![i]!))
   ||!Array.isArray(c.uncertainUv)||c.uncertainUv.length!==2||!c.uncertainUv.every(q=>Array.isArray(q)&&q.length===2&&q.every(interval)))return false
 }
 if(r.qualified)return r.reason==='source-wall-searched-thickness-bounds-qualified'&&c.reason==='source-face-gap-qualified'
  &&s.attempts>s.refused&&interval(r.intervalMm)&&r.intervalMm[0]>=e.minimumMm
  &&r.converged===(nextUp(r.intervalMm[1]-r.intervalMm[0])<=e.toleranceMm)
 return r.intervalMm===null&&!r.converged&&(r.reason==='source-wall-clearance-unproven'&&c.reason!=='source-face-gap-qualified'
  ||r.reason==='source-wall-search-witness-unproven'&&c.reason==='source-face-gap-qualified'&&s.attempts===s.refused)
}
