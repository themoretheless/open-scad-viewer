/** Transport checks only. Original coefficient proofs remain native. */
export interface SourceWallCoverageOptions {
 minimumMm:number
 adaptiveSelf?:{planeControls:number;cells:number;spans:number;cellsPerFace:number;spansPerFace:number}
 limits:{pairs:number;planeControls:number;normalSpans:number;gapCells:number;gapSpans:number;maxSineSquared:number}
}
export interface SourceWallCoverageResult {
 adaptiveSelf?:{faces:{face:number;qualified:boolean;reason:string;cells:number;spans:number;pending:number;uncertain:number[][][]|null}[];planeControls:number;cells:number;spans:number;allSelfPairsQualified:boolean}|null
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
 if(o.adaptiveSelf)for(const name of ['planeControls','cells','spans','cellsPerFace','spansPerFace'] as const)
  if(!count(o.adaptiveSelf[name],name==='planeControls'?1000000:100000)||o.adaptiveSelf[name]<1)throw new Error('Invalid adaptive self-wall work')
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
 const adaptive=r.adaptiveSelf,w=e.adaptiveSelf
 if(!w){if(adaptive!=null)return false}else{
  const a=adaptive
  if(!a||!Array.isArray(a.faces)||a.faces.length!==faces||typeof a.allSelfPairsQualified!=='boolean'
   ||!count(a.planeControls,w.planeControls)||!count(a.cells,w.cells)||!count(a.spans,w.spans))return false
  let cells=0,spans=0,allSelf=true
  for(let i=0;i<faces;i++){
   const f=a.faces[i]
   if(!f||f.face!==i||typeof f.qualified!=='boolean'||!count(f.cells,w.cellsPerFace)||!count(f.spans,w.spansPerFace)
    ||!count(f.pending,100001)||f.qualified!==(f.pending===0)||f.qualified!==(f.uncertain===null))return false
   const allowed=f.qualified?['source-self-wall-planar-excluded','source-self-wall-qualified']
    :['source-self-wall-total-work-limit','source-self-wall-resolution-limit','source-self-wall-work-limit']
   if(!allowed.includes(f.reason))return false
   if(!f.qualified){
    const u=f.uncertain
    if(!Array.isArray(u)||u.length!==2||u.some(d=>!Array.isArray(d)||d.length!==2||d.some(t=>!Array.isArray(t)||t.length!==2||!t.every(Number.isFinite)||t[0]>t[1])))return false
   }
   cells+=f.cells;spans+=f.spans;allSelf=allSelf&&f.qualified
  }
  if(cells!==a.cells||spans!==a.spans||allSelf!==a.allSelfPairsQualified)return false
 }
 let index=0,all=true
 for(let a=0;a<faces;a++)for(let b=a;b<faces&&index<expected;b++,index++) {
  const p=r.pairs[index]
  if(!p||!Array.isArray(p.faces)||p.faces.length!==2||p.faces[0]!==a||p.faces[1]!==b
   ||typeof p.proven!=='boolean'||typeof p.reason!=='string'||!p.reason.startsWith('source-'))return false
  const allowed=a===b
   ?p.proven?['source-wall-planar-self-excluded','source-wall-curved-self-excluded','source-wall-self-certified']:['source-wall-curved-self-unproven']
   :p.proven?['source-wall-coplanar-excluded','source-wall-normal-pair-excluded','source-face-gap-qualified']
    :['source-wall-pair-unproven','source-face-gap-work-limit','source-face-gap-resolution-limit']
  if(!allowed.includes(p.reason))return false
  if(p.reason==='source-wall-self-certified'&&(!adaptive||!adaptive.faces[p.faces[0]]?.qualified))return false
  all=all&&p.proven
 }
 const qualified=r.enumerationComplete&&all
 if(r.wholeWallQualified!==qualified)return false
 return qualified?r.lowerMm===e.minimumMm&&r.reason==='source-whole-wall-lower-qualified'
  :r.lowerMm===null&&r.reason===(r.enumerationComplete?'source-wall-coverage-unproven':'source-wall-coverage-pair-limit')
}
