import {validWallWitness,type SourceWallWitness} from './sourceWallTransport'
export interface SourceWallScanOptions {
 minimumMm:number;toleranceUv:number;grid:number;maxAttempts:number
 limits:{cells:number;domainCells:number;normalSpans:number;maxSineSquared:number}
}
export interface SourceWallScanResult {
 request:SourceWallScanOptions;thinFound:boolean;witness:SourceWallWitness|null
 attempts:number;refused:number;facesVisited:number;facesTotal:number;proposalsExhausted:boolean;wholeWallQualified:false
}
const count=(n:unknown,max:number)=>typeof n==='number'&&Number.isSafeInteger(n)&&n>=0&&n<=max
export function scanExpectation(o:SourceWallScanOptions):SourceWallScanOptions {
 if(!o||![o.minimumMm,o.toleranceUv].every(n=>Number.isFinite(n)&&n>0)
  ||!count(o.grid,8)||o.grid<1||!count(o.maxAttempts,256)||o.maxAttempts<1||!o.limits
  ||!count(o.limits.cells,1000000)||o.limits.cells<1||!count(o.limits.domainCells,8000000)||o.limits.domainCells<1
  ||!count(o.limits.normalSpans,100000)||o.limits.normalSpans<1||!Number.isFinite(o.limits.maxSineSquared)
  ||o.limits.maxSineSquared<0||o.limits.maxSineSquared>=1)throw new Error('Invalid original wall scan request')
 return structuredClone(o)
}
export function validScan(e:SourceWallScanOptions|undefined,r:SourceWallScanResult|null|undefined,faces:number,key:(v:unknown)=>string):boolean {
 if(!e)return r==null
 if(!r||key(r.request)!==key(e)||r.wholeWallQualified!==false||typeof r.thinFound!=='boolean'||typeof r.proposalsExhausted!=='boolean'
  ||r.facesTotal!==faces||faces<2||!count(r.attempts,e.maxAttempts)||!count(r.refused,r.attempts)||!count(r.facesVisited,faces))return false
 const total=faces*e.grid**2
 if(r.attempts!==Math.min(total,e.maxAttempts)||r.facesVisited!==Math.ceil(r.attempts/e.grid**2)||r.proposalsExhausted!==(r.attempts===total))return false
 if(r.witness===null)return !r.thinFound&&r.refused===r.attempts
 return r.refused<r.attempts&&validWallWitness(r.witness,f=>count(f,faces-1))&&r.thinFound===(r.witness.lengthMm[1]<e.minimumMm)
}
