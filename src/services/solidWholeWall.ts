import {callGeometryRust} from './geometry/kernel'
import {materialExpectation,validMaterial,type MaterialOptions,type MaterialChord,type NormalAudit} from './solidMaterialVolume'
export interface WholeWallOptions extends MaterialOptions {
 normalAudit:NormalAudit;toleranceMm:number;maxDistanceCells:number;maxDistanceDomainCells:number
 coverageLimits:{maxFacePairs:number;maxPlaneControls:number;maxNormalSpans:number}
}
export interface WholeWallPair {faces:[number,number];lowerBoundMm:number|null;reason:'planar-self-excluded'|'coplanar-chord-excluded'|'endpoint-normal-excluded'|'self-pair-unresolved'|'distance-work-limit'|'complete-face-distance-bound'}
export interface WholeWallResult {
 method:'bounded-whole-material-wall';scope:'aligned-material-chords-over-all-original-face-pairs'
 toleranceMm:number;maxDistanceCells:number;maxDistanceDomainCells:number;coverageLimits:WholeWallOptions['coverageLimits']
 intervalMm:[number,number]|null;converged:boolean;reason:string;candidate:MaterialChord
 coverage:{totalPairs:number;enumerationComplete:boolean;lowerBoundMm:number;planeControls:number;normalSpans:number;cells:number;domainCells:number;pairs:WholeWallPair[]}
}
const work=(n:unknown,max:number)=>typeof n==='number'&&Number.isSafeInteger(n)&&n>=0&&n<=max
export function wholeWallExpectation(o:WholeWallOptions){
 return {material:materialExpectation(o,'chord'),faces:o.model.faces.length,
  toleranceMm:o.toleranceMm,maxDistanceCells:o.maxDistanceCells,maxDistanceDomainCells:o.maxDistanceDomainCells,
  coverageLimits:structuredClone(o.coverageLimits)}
}
export function validWholeWall(e:ReturnType<typeof wholeWallExpectation>,value:unknown):value is WholeWallResult {
 const r=value as WholeWallResult,l=e.coverageLimits
 if(!e.material.normalAudit||!Number.isFinite(e.toleranceMm)||e.toleranceMm<=0||
  !work(e.maxDistanceCells,1000000)||e.maxDistanceCells<1||!work(e.maxDistanceDomainCells,8000000)||e.maxDistanceDomainCells<1||
  !l||!work(l.maxFacePairs,100000)||l.maxFacePairs<1||!work(l.maxPlaneControls,1000000)||l.maxPlaneControls<1||!work(l.maxNormalSpans,100000)||l.maxNormalSpans<1)return false
 if(!r||r.method!=='bounded-whole-material-wall'||r.scope!=='aligned-material-chords-over-all-original-face-pairs'||
  r.toleranceMm!==e.toleranceMm||r.maxDistanceCells!==e.maxDistanceCells||r.maxDistanceDomainCells!==e.maxDistanceDomainCells||
  !r.coverageLimits||r.coverageLimits.maxFacePairs!==l.maxFacePairs||r.coverageLimits.maxPlaneControls!==l.maxPlaneControls||r.coverageLimits.maxNormalSpans!==l.maxNormalSpans||
  !validMaterial(e.material,r.candidate)||r.candidate.method!=='continuous-material-chord'||typeof r.converged!=='boolean')return false
 const d=r.coverage,total=e.faces*(e.faces+1)/2
 if(!Number.isSafeInteger(total)||!d||d.totalPairs!==total||!Array.isArray(d.pairs)||d.pairs.length!==Math.min(total,l.maxFacePairs)||
  d.enumerationComplete!==(d.pairs.length===total)||!work(d.planeControls,l.maxPlaneControls)||!work(d.normalSpans,l.maxNormalSpans)||
  !work(d.cells,e.maxDistanceCells)||!work(d.domainCells,e.maxDistanceDomainCells)||!Number.isFinite(d.lowerBoundMm)||d.lowerBoundMm<0)return false
 let a=0,b=0,lower=Infinity
 for(const p of d.pairs){
  if(!p||!Array.isArray(p.faces)||p.faces.length!==2||p.faces[0]!==a||p.faces[1]!==b)return false
  if(p.reason==='planar-self-excluded'){if(a!==b||p.lowerBoundMm!==null)return false}
  else if(p.reason==='coplanar-chord-excluded'||p.reason==='endpoint-normal-excluded'){if(a===b||p.lowerBoundMm!==null)return false}
  else if(p.reason==='self-pair-unresolved'){if(a!==b||p.lowerBoundMm!==0)return false}
  else if(p.reason==='distance-work-limit'){if(a===b||p.lowerBoundMm!==0)return false}
  else if(p.reason==='complete-face-distance-bound'){if(a===b||p.lowerBoundMm===null||!Number.isFinite(p.lowerBoundMm)||p.lowerBoundMm<0)return false}
  else return false
  if(p.lowerBoundMm!==null)lower=Math.min(lower,p.lowerBoundMm)
  if(++b===e.faces){a++;b=a}
 }
 if(!d.enumerationComplete||!Number.isFinite(lower))lower=0
 if(d.lowerBoundMm!==lower)return false
 const c=r.candidate
 let reason=!c.proven?'material-chord-unproven':c.normalAlignment!=='angular-tolerance'?'candidate-normal-unproven':d.enumerationComplete?'whole-wall-bounds':'face-pair-limit'
 if(r.reason!==reason)return false
 if(!c.proven||c.normalAlignment!=='angular-tolerance')return r.intervalMm===null&&!r.converged
 const interval=r.intervalMm,upper=c.lengthIntervalMm![1]
 return Array.isArray(interval)&&interval.length===2&&interval[0]===lower&&interval[1]===upper&&lower<=upper&&
  r.converged===(d.enumerationComplete&&upper-lower<e.toleranceMm)
}
export const inspectWholeWall=(options:WholeWallOptions):WholeWallResult=>callGeometryRust('cad_whole_wall',options)
