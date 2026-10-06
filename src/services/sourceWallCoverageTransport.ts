/** Transport checks only. Original coefficient proofs remain native. */
export interface SourceWallCoverageOptions {
 minimumMm:number
 limits:{pairs:number;planeControls:number;normalSpans:number;gapCells:number;gapSpans:number;maxSineSquared:number}
}
export interface SourceWallCoverageResult {
 request:SourceWallCoverageOptions;wholeWallQualified:boolean;lowerMm:number|null
 enumerationComplete:boolean;totalPairs:number
 pairs:{faces:[number,number];proven:boolean;reason:string}[]
 planeControls:number;normalSpans:number;cells:number;spans:number;reason:string
}
const count=(n:unknown,max:number)=>typeof n==='number'&&Number.isSafeInteger(n)&&n>=0&&n<=max
export function coverageExpectation(o:SourceWallCoverageOptions):SourceWallCoverageOptions {
 if(!o||!Number.isFinite(o.minimumMm)||o.minimumMm<=0||!o.limits)throw new Error('Invalid continuous wall coverage request')
 const w=o.limits
 for(const [name,max] of [['pairs',100000],['planeControls',1000000],['normalSpans',100000],['gapCells',100000],['gapSpans',100000]] as const)
  if(!count(w[name],max)||w[name]<1)throw new Error('Invalid continuous wall coverage work')
 if(!Number.isFinite(w.maxSineSquared)||w.maxSineSquared<0||w.maxSineSquared>=1)throw new Error('Invalid continuous wall coverage angle')
 return structuredClone(o)
}
export function validCoverage(e:SourceWallCoverageOptions|undefined,r:SourceWallCoverageResult|null|undefined,faces:number,key:(v:unknown)=>string):boolean {
 if(!e)return r==null
 if(!r||key(r.request)!==key(e)||!count(faces,4096)||faces<2||typeof r.wholeWallQualified!=='boolean'
  ||typeof r.enumerationComplete!=='boolean'||!Array.isArray(r.pairs))return false
 const total=faces*(faces+1)/2,expected=Math.min(total,e.limits.pairs)
 if(r.totalPairs!==total||r.pairs.length!==expected||r.enumerationComplete!==(expected===total)
  ||!count(r.planeControls,e.limits.planeControls)||!count(r.normalSpans,e.limits.normalSpans)
  ||!count(r.cells,e.limits.gapCells)||!count(r.spans,e.limits.gapSpans))return false
 let index=0,all=true
 for(let a=0;a<faces;a++)for(let b=a;b<faces&&index<expected;b++,index++) {
  const p=r.pairs[index]
  if(!p||!Array.isArray(p.faces)||p.faces.length!==2||p.faces[0]!==a||p.faces[1]!==b
   ||typeof p.proven!=='boolean'||typeof p.reason!=='string'||!p.reason.startsWith('source-'))return false
  const allowed=a===b
   ?p.proven?['source-wall-planar-self-excluded','source-wall-curved-self-excluded']:['source-wall-curved-self-unproven']
   :p.proven?['source-wall-coplanar-excluded','source-wall-normal-pair-excluded','source-face-gap-qualified']
    :['source-wall-pair-unproven','source-face-gap-work-limit','source-face-gap-resolution-limit']
  if(!allowed.includes(p.reason))return false
  all=all&&p.proven
 }
 const qualified=r.enumerationComplete&&all
 if(r.wholeWallQualified!==qualified)return false
 return qualified?r.lowerMm===e.minimumMm&&r.reason==='source-whole-wall-lower-qualified'
  :r.lowerMm===null&&r.reason===(r.enumerationComplete?'source-wall-coverage-unproven':'source-wall-coverage-pair-limit')
}
