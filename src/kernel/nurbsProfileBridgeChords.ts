import type {NurbsSurface} from './nurbsSurface'
import {callNurbsRust} from './geometry/nurbs'
type Rect=[[number,number],[number,number]]
export interface ProfileBridgeChordOptions {
 surfaces:[NurbsSurface,NurbsSurface];pivots:[[number,number],[number,number]]
 axes:[0|1,0|1];parameters:[number,number];scales:[number,number]
 pair:[Rect,Rect];maxSineSquared:number;maxSpans:number;maxControls:number
 coverage?:{domain:Rect;cells:number;spans:number}
}
export interface ProfileBridgeChordReport {
 version:1;method:'owned-profile-bridge-endpoint-chords';request:Record<string,unknown>
 proposedExpression:true;wholeThicknessQualified:false;solidQualified:false
 profileControls:number;spans:number;excluded:boolean|null
 reason:'profile-bridge-profile-unproven'|'profile-bridge-chord-excluded'|'profile-bridge-chord-unexcluded'|'profile-bridge-chord-unproven'
 profilePositionDeviationUpperMm?:[[number,number,number],[number,number,number]]
 profileWeightDeviationUpper?:[number,number]
 coverage?:{cells:number;spans:number;exclusions:number;unresolvedPairs:[Rect,Rect][];completeExclusion:boolean}
}
function canonical(value:unknown):string {
 const sorted=(v:unknown):unknown=>{
  if(typeof v==='number'&&!Number.isFinite(v))throw new Error('Finite profile bridge request numbers required')
  return Array.isArray(v)?v.map(sorted):v&&typeof v==='object'
   ?Object.fromEntries(Object.entries(v).filter(([,x])=>x!==undefined).sort(([a],[b])=>a.localeCompare(b)).map(([k,x])=>[k,sorted(x)])):v
 }
 return JSON.stringify(sorted(value))
}
const count=(v:unknown,max:number):v is number=>typeof v==='number'&&Number.isSafeInteger(v)&&v>=0&&v<=max
const nonnegative=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)&&v>=0
const finiteRect=(v:unknown):v is Rect=>Array.isArray(v)&&v.length===2&&v.every(a=>Array.isArray(a)&&a.length===2&&a.every(x=>typeof x==='number'&&Number.isFinite(x))&&a[0]<a[1])
export function profileBridgeChordExpectation(options:ProfileBridgeChordOptions){
 const copy=structuredClone(options)
 if(!count(copy.maxControls,1000000)||!count(copy.maxSpans,100000)||!nonnegative(copy.maxSineSquared)||copy.maxSineSquared>=1
  ||(copy.coverage!==undefined&&(!count(copy.coverage.cells,100000)||!count(copy.coverage.spans,100000)||!finiteRect(copy.coverage.domain))))
  throw Object.assign(new Error('Invalid profile bridge work or angle bounds'),{code:'CAD_REQUEST'})
 return {requestIdentity:canonical({op:'profile_section_bridge_chords',...copy}),maxControls:copy.maxControls,maxSpans:copy.maxSpans,coverage:copy.coverage}
}
export function validProfileBridgeChordReport(expected:ReturnType<typeof profileBridgeChordExpectation>,value:unknown):value is ProfileBridgeChordReport {
 if(!value||typeof value!=='object')return false
 const r=value as ProfileBridgeChordReport
 try{if(canonical(r.request)!==expected.requestIdentity)return false}catch{return false}
 if(r.version!==1||r.method!=='owned-profile-bridge-endpoint-chords'||r.proposedExpression!==true
  ||r.wholeThicknessQualified!==false||r.solidQualified!==false||!count(r.profileControls,expected.maxControls)
  ||!count(r.spans,expected.maxSpans)||![true,false,null].includes(r.excluded))return false
 if((expected.maxSpans===0||expected.maxControls===0)&&(r.excluded!==null||r.spans!==0))return false
 if(expected.coverage===undefined){if(r.coverage!==undefined)return false}
 else {
  const c=r.coverage,e=expected.coverage
  if(!c||!count(c.cells,e.cells)||!count(c.spans,e.spans)||!count(c.exclusions,c.cells)
   ||typeof c.completeExclusion!=='boolean'||!Array.isArray(c.unresolvedPairs)
   ||c.unresolvedPairs.length>2*e.cells+1||c.completeExclusion!==(c.unresolvedPairs.length===0))return false
  if((e.cells===0||e.spans===0||expected.maxControls===0)&&(c.completeExclusion||c.cells!==0||c.spans!==0||c.exclusions!==0))return false
  const rect=(v:unknown):boolean=>Array.isArray(v)&&v.length===2&&v.every((axis,k)=>
   Array.isArray(axis)&&axis.length===2&&axis.every(x=>typeof x==='number'&&Number.isFinite(x))
   &&axis[0]<axis[1]&&axis[0]>=e.domain[k][0]&&axis[1]<=e.domain[k][1])
  if(!c.unresolvedPairs.every(pair=>Array.isArray(pair)&&pair.length===2&&pair.every(rect)))return false
  if(c.completeExclusion&&(c.cells===0||c.spans===0||c.exclusions===0))return false
  if(r.reason==='profile-bridge-profile-unproven'&&(c.completeExclusion||c.cells!==0||c.spans!==0||c.exclusions!==0))return false
  if((c.cells===0||r.reason==='profile-bridge-profile-unproven')&&canonical(c.unresolvedPairs)!==canonical([[e.domain,e.domain]]))return false
 }
 if(r.reason==='profile-bridge-profile-unproven')return r.excluded===null&&r.spans===0
 if(r.reason!==(r.excluded===true?'profile-bridge-chord-excluded':r.excluded===false?'profile-bridge-chord-unexcluded':'profile-bridge-chord-unproven'))return false
 const p=r.profilePositionDeviationUpperMm,w=r.profileWeightDeviationUpper
 return Array.isArray(p)&&p.length===2&&p.every(row=>Array.isArray(row)&&row.length===3&&row.every(nonnegative))
  &&Array.isArray(w)&&w.length===2&&w.every(nonnegative)
}
/** Native proposed-expression diagnostic; no TS geometry or solid admission. */
export function inspectProfileBridgeChords(options:ProfileBridgeChordOptions):ProfileBridgeChordReport {
 const expected=profileBridgeChordExpectation(options)
 const report=callNurbsRust<unknown>('profile_section_bridge_chords',options)
 if(!validProfileBridgeChordReport(expected,report))throw Object.assign(new Error('Invalid native profile bridge chord report'),{code:'CAD_PROTOCOL'})
 return report
}
