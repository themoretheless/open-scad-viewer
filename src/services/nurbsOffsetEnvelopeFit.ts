import type {NurbsSurface} from './nurbsSurface'
import type {OffsetContactBandOptions,SurfaceDomain} from './nurbsSurfaceOffset'
import {callNurbsRust} from './geometry/nurbs'
export interface OffsetEnvelopeFitOptions extends OffsetContactBandOptions {
 candidate?:NurbsSurface;toleranceMm:number;maxCells:number
}
export interface EnvelopeFitCell {
 domain:SurfaceDomain;errorUpperMm:number|null;anchorErrorIntervalMm:[number,number]|null
 envelopeRegularityProven:boolean;candidateRegularityProven:boolean;admitted:boolean
}
export interface EnvelopeFitNode {domain:SurfaceDomain;splitAxis:0|1|null;children:[number,number]|null;leaf:number|null}
export interface EnvelopeFitQualification {
 method:'interval-envelope-patch-fit';scope:'pointwise-original-parameter-rectangle'
 cells:EnvelopeFitCell[];partition:EnvelopeFitNode[];visitedCells:number;envelopeQueries:number
 approximationWithinToleranceProven:boolean;finiteNurbsPatchProven:boolean;reason:string
 wholeCurveComplete:false;tangentToleranceProven:false;trimMembershipProven:false;embeddingProven:false;topologyAuthority:false
}
export interface OffsetEnvelopeFit {
 method:'interval-offset-finite-patch';scope:'constant-radius-contact-band';radiusMm:number
 candidateSource:'authored'|'section-proposal'
 /** Declared endpoint section count for proposals; fit oracle visits are counted separately. */
 proposalSections:0|2;candidateSurface:NurbsSurface|null;qualification:EnvelopeFitQualification|null;reason:string
 wholeCurveComplete:false;tangentToleranceProven:false;trimMembershipProven:false;embeddingProven:false;topologyAuthority:false
}
export interface EnvelopeFitExpectation {
 domain:SurfaceDomain;radiusMm:number;toleranceMm:number;maxCells:number;candidate:string|null
}
function surfaceSnapshot(s:NurbsSurface):string {
 return JSON.stringify([s.degreeU,s.degreeV,s.knotsU,s.knotsV,s.controlPoints,s.weights,s.periodicU??false,s.periodicV??false])
}
export function envelopeFitExpectation(o:OffsetEnvelopeFitOptions):EnvelopeFitExpectation {
 return {domain:[[...o.fixedInterval],[0,1]],radiusMm:Math.abs(o.distances[0]),toleranceMm:o.toleranceMm,maxCells:o.maxCells,candidate:o.candidate?surfaceSnapshot(o.candidate):null}
}
const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)
const interval=(v:unknown):v is [number,number]=>Array.isArray(v)&&v.length===2&&v.every(finite)&&v[0]<=v[1]
const rectangle=(v:unknown):v is SurfaceDomain=>Array.isArray(v)&&v.length===2&&v.every(interval)&&v.every(x=>x[0]<x[1])
function sameRectangle(a:SurfaceDomain,b:SurfaceDomain){return a.every((x,k)=>x[0]===b[k][0]&&x[1]===b[k][1])}
function validSurface(s:unknown,expected:EnvelopeFitExpectation):s is NurbsSurface {
 if(!s||typeof s!=='object')return false
 const v=s as NurbsSurface,n=v.controlPoints?.length,m=v.controlPoints?.[0]?.length
 if(!Array.isArray(v.controlPoints)||!Number.isSafeInteger(n)||n<2||n>256||!Number.isSafeInteger(m)||m<2||m>256
  ||!Number.isSafeInteger(v.degreeU)||v.degreeU<1||v.degreeU>=n||!Number.isSafeInteger(v.degreeV)||v.degreeV<1||v.degreeV>=m
  ||!Array.isArray(v.knotsU)||v.knotsU.length!==n+v.degreeU+1||!Array.isArray(v.knotsV)||v.knotsV.length!==m+v.degreeV+1)return false
 for(const knots of [v.knotsU,v.knotsV])if(!knots.every(finite)||knots.some((x,k)=>k>0&&x<knots[k-1]))return false
 if(!v.controlPoints.every(row=>Array.isArray(row)&&row.length===m&&row.every(p=>Array.isArray(p)&&p.length===3&&p.every(finite)))
  ||!Array.isArray(v.weights)||v.weights.length!==n||!v.weights.every(row=>Array.isArray(row)&&row.length===m&&row.every(w=>finite(w)&&w>0))
  ||typeof v.periodicU!=='boolean'||typeof v.periodicV!=='boolean')return false
 let lo=Infinity,hi=0;for(const row of v.weights)for(const w of row){lo=Math.min(lo,w);hi=Math.max(hi,w)}
 if(hi/lo>1e12)return false
 if(!sameRectangle([[v.knotsU[v.degreeU],v.knotsU[n]],[v.knotsV[v.degreeV],v.knotsV[m]]],expected.domain))return false
 if(expected.candidate!==null)return surfaceSnapshot(v)===expected.candidate
 return n===2&&m===3&&v.degreeU===1&&v.degreeV===2&&!v.periodicU&&!v.periodicV
  &&JSON.stringify(v.knotsU)===JSON.stringify([expected.domain[0][0],expected.domain[0][0],expected.domain[0][1],expected.domain[0][1]])
  &&JSON.stringify(v.knotsV)==='[0,0,0,1,1,1]'
}
function smooth(s:NurbsSurface):boolean {
 for(const [degree,knots,count,periodic] of [[s.degreeU,s.knotsU,s.controlPoints.length,s.periodicU],[s.degreeV,s.knotsV,s.controlPoints[0].length,s.periodicV]] as const){
  if(periodic)return false
  const start=knots[degree],end=knots[count]
  for(let i=0;i<knots.length;){let j=i+1;while(j<knots.length&&knots[j]===knots[i])j++
   if(knots[i]>start&&knots[i]<end&&degree-(j-i)<1)return false;i=j
  }
 }
 return true
}
const closedGates=(v:unknown):boolean=>!!v&&typeof v==='object'&&['wholeCurveComplete','tangentToleranceProven','trimMembershipProven','embeddingProven','topologyAuthority'].every(k=>(v as Record<string,unknown>)[k]===false)
/** Tree coverage is checked in linear time; area sums cannot establish absence of holes/overlaps. */
export function validEnvelopeFit(expected:EnvelopeFitExpectation,value:unknown):boolean {
 if(!value||typeof value!=='object')return false
 const v=value as OffsetEnvelopeFit
 if(!finite(expected.radiusMm)||expected.radiusMm<=0||!finite(expected.toleranceMm)||expected.toleranceMm<=0
  ||!Number.isSafeInteger(expected.maxCells)||expected.maxCells<1||expected.maxCells>1_000_000||!rectangle(expected.domain)
  ||v.method!=='interval-offset-finite-patch'||v.scope!=='constant-radius-contact-band'||v.radiusMm!==expected.radiusMm||!closedGates(v)
  ||v.candidateSource!==(expected.candidate===null?'section-proposal':'authored')||v.proposalSections!==(expected.candidate===null?2:0))return false
 if(v.candidateSurface===null)return expected.candidate===null&&v.qualification===null&&v.reason==='proposal-contact-unresolved'
 if(!validSurface(v.candidateSurface,expected)||!v.qualification)return false
 const q=v.qualification
 if(q.method!=='interval-envelope-patch-fit'||q.scope!=='pointwise-original-parameter-rectangle'||!closedGates(q)
  ||!Array.isArray(q.cells)||q.cells.length<1||!Array.isArray(q.partition)||q.partition.length!==q.visitedCells
  ||!Number.isSafeInteger(q.visitedCells)||q.visitedCells<1||q.visitedCells>expected.maxCells||q.visitedCells!==2*q.cells.length-1
  ||!Number.isSafeInteger(q.envelopeQueries)||q.envelopeQueries<q.visitedCells||q.envelopeQueries>2*q.visitedCells)return false
 let admitted=true,mismatch=false
 for(const c of q.cells){
  if(!c||!rectangle(c.domain)||typeof c.envelopeRegularityProven!=='boolean'||typeof c.candidateRegularityProven!=='boolean'||typeof c.admitted!=='boolean'
   ||(c.errorUpperMm!==null&&(!finite(c.errorUpperMm)||c.errorUpperMm<0))
   ||(c.anchorErrorIntervalMm!==null&&(!interval(c.anchorErrorIntervalMm)||c.anchorErrorIntervalMm[0]<0)))return false
  if(c.errorUpperMm!==null&&(c.anchorErrorIntervalMm===null||c.errorUpperMm<c.anchorErrorIntervalMm[0]))return false
  if(c.admitted!==(c.errorUpperMm!==null&&c.errorUpperMm<=expected.toleranceMm&&c.envelopeRegularityProven&&c.candidateRegularityProven))return false
  admitted&&=c.admitted;mismatch||=c.anchorErrorIntervalMm!==null&&c.anchorErrorIntervalMm[0]>expected.toleranceMm
 }
 const nodes=new Set<number>(),leaves=new Set<number>(),pending:[number,SurfaceDomain][]=[[0,expected.domain]]
 while(pending.length){
  const [i,domain]=pending.pop()!
  if(!Number.isSafeInteger(i)||i<0||i>=q.partition.length||nodes.has(i))return false
  nodes.add(i);const n=q.partition[i]
  if(!n||!rectangle(n.domain)||!sameRectangle(n.domain,domain))return false
  if(n.children===null){
   if(n.splitAxis!==null||!Number.isSafeInteger(n.leaf)||n.leaf!<0||n.leaf!>=q.cells.length||leaves.has(n.leaf!)||!sameRectangle(q.cells[n.leaf!].domain,domain))return false
   leaves.add(n.leaf!)
  }else{
   if(n.leaf!==null||(n.splitAxis!==0&&n.splitAxis!==1)||!Array.isArray(n.children)||n.children.length!==2)return false
   const axis=n.splitAxis,mid=domain[axis][0]+(domain[axis][1]-domain[axis][0])*.5
   if(!(mid>domain[axis][0]&&mid<domain[axis][1]))return false
   const left:SurfaceDomain=[[...domain[0]],[...domain[1]]],right:SurfaceDomain=[[...domain[0]],[...domain[1]]];left[axis][1]=mid;right[axis][0]=mid
   pending.push([n.children[1],right],[n.children[0],left])
  }
 }
 if(nodes.size!==q.partition.length||leaves.size!==q.cells.length||q.approximationWithinToleranceProven!==admitted||q.finiteNurbsPatchProven!==admitted||v.reason!==q.reason)return false
 const continuous=smooth(v.candidateSurface)
 if(admitted&&!continuous)return false
 return q.reason===(admitted?'pointwise-fit-within-tolerance':mismatch?'candidate-mismatch':!continuous?'candidate-continuity-unproven':'fit-work-limit')
}
/** The actual finite candidate is returned beside its full-domain qualification. */
export const fitNurbsOffsetEnvelope=(options:OffsetEnvelopeFitOptions):OffsetEnvelopeFit=>callNurbsRust('surface_offset_envelope_fit',options)
