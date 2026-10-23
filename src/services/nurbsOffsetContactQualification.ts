import type {NurbsCurve} from './nurbsCurve'
import type {NurbsSurface} from './nurbsSurface'
import type {OffsetContactBandOptions} from './nurbsSurfaceOffset'
import {callNurbsRust} from './geometry/nurbs'
import {envelopeFitExpectation,validEnvelopeFit} from './nurbsOffsetEnvelopeFit'
export interface OffsetContactQualificationOptions extends OffsetContactBandOptions {
 candidate:NurbsSurface;sourcePcurves:[NurbsCurve,NurbsCurve];firstLoops:NurbsCurve[][];secondLoops:NurbsCurve[][]
 toleranceMm:number;toleranceUv:number;maxFitCells:number;maxUvCells:number;maxAgreementCells:number;rootRefinements:number
 maxPairs:number;maxTrimCells:number;maxDomainCells:number;maxSineSquared:number
 maxTangentPositionCells:number;maxNormalCells:number;maxNormalSpans:number
}
type RecordValue=Record<string,unknown>
export interface OffsetContactQualification {
 method:'offset-contact-qualification-delivery';request:RecordValue;candidateSurface:NurbsSurface
 qualification:RecordValue
}
export interface ContactQualificationExpectation {snapshot:string;options:OffsetContactQualificationOptions}
function canonical(v:unknown):string {
 const sorted=(x:unknown):unknown=>Array.isArray(x)?x.map(sorted):x&&typeof x==='object'
  ?Object.fromEntries(Object.entries(x).sort(([a],[b])=>a.localeCompare(b)).map(([k,v])=>[k,sorted(v)])):x
 return JSON.stringify(sorted(v))
}
export function contactQualificationExpectation(options:OffsetContactQualificationOptions):ContactQualificationExpectation {
 const copy=JSON.parse(JSON.stringify(options)) as OffsetContactQualificationOptions
 return {snapshot:canonical({op:'surface_offset_contact_qualification',...copy}),options:copy}
}
const record=(x:unknown):x is RecordValue=>!!x&&typeof x==='object'&&!Array.isArray(x)
const finite=(x:unknown):x is number=>typeof x==='number'&&Number.isFinite(x)
const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&x.every(finite)&&x[0]<=x[1]
const count=(x:unknown,max:number)=>typeof x==='number'&&Number.isSafeInteger(x)&&x>=0&&x<=max
const closed=(v:RecordValue,keys:string[])=>keys.every(k=>v[k]===false)
const curveSnapshot=(v:unknown)=>record(v)?canonical([v.degree,v.knots,v.controlPoints,v.weights,v.periodic??false]):''
function coverage(cells:unknown,key:string,domain:[number,number]):cells is RecordValue[] {
 if(!Array.isArray(cells)||cells.length===0||!cells.every(record))return false
 let start=domain[0]
 for(const cell of cells){const d=cell[key];if(!interval(d)||d[0]!==start||d[0]>=d[1])return false;start=d[1]}
 return start===domain[1]
}
function validPositions(value:unknown,max:number):value is RecordValue[] {
 return Array.isArray(value)&&value.length<=2&&value.every(p=>record(p)&&['within-tolerance','mismatch','unresolved'].includes(String(p.status))
  &&count(p.cells,max)&&typeof p.cells==='number'&&p.cells>0
  &&(p.witnessParameter===null||(finite(p.witnessParameter)&&p.witnessParameter>=0&&p.witnessParameter<=1))
  &&(p.witnessDistanceMm===null||(interval(p.witnessDistanceMm)&&p.witnessDistanceMm[0]>=0)))
  &&value.reduce((sum,p)=>sum+(p as RecordValue).cells as number,0)<=max
}
function normalsRegular(value:unknown):boolean {
 return Array.isArray(value)&&value.length===2&&value.every(n=>Array.isArray(n)&&n.length===3&&n.every(interval)
  &&n.some(d=>d[0]>0||d[1]<0))
}
function validTangent(value:unknown,o:OffsetContactQualificationOptions):value is RecordValue {
 if(!record(value)||!validPositions(value.positions,o.maxTangentPositionCells)||!coverage(value.cells,'interval',[0,1])
  ||!count(value.visitedCells,o.maxNormalCells)||!count(value.normalSpanVisits,o.maxNormalSpans))return false
 const positions=value.positions.length===2&&value.positions.every(p=>p.status==='within-tolerance')
 if(value.positionsProven!==positions)return false
 let admitted=true,evaluated=0,spans=0
 for(const c of value.cells){
  if(c.normalSpans===null){if(c.aligned!==null||c.sineSquaredInterval!==null||c.normalComponents!==null||c.reason!==null)return false;admitted=false;continue}
  if(!count(c.normalSpans,o.maxNormalSpans)||typeof c.normalSpans!=='number'||c.normalSpans<1||typeof c.reason!=='string')return false
  evaluated++;spans+=c.normalSpans
  const sine=c.sineSquaredInterval
  if(sine!==null&&(!interval(sine)||sine[0]<0||sine[1]>1))return false
  const verdict=sine===null?null:(sine as [number,number])[1]<=o.maxSineSquared?true:(sine as [number,number])[0]>o.maxSineSquared?false:null
  if(c.aligned!==verdict)return false
  if(c.aligned===true&&!normalsRegular(c.normalComponents))return false
  admitted&&=c.aligned===true
 }
 if(typeof value.visitedCells!=='number'||typeof value.normalSpanVisits!=='number'||evaluated>value.visitedCells||spans>value.normalSpanVisits)return false
 return value.tangentPlanesProven===(positions&&admitted)
}
export function validContactQualification(expected:ContactQualificationExpectation,value:unknown):value is OffsetContactQualification {
 if(!record(value)||value.method!=='offset-contact-qualification-delivery'||canonical(value.request)!==expected.snapshot||!record(value.qualification))return false
 const o=expected.options,q=value.qualification,c=q.contacts
 if(!record(c)||q.method!=='interval-offset-contact-qualification'||q.scope!=='original-contact-branch-and-tangent-planes'
  ||!closed(q,['wholeCurveComplete','replacementFaceTrimsProven','stitchedTopologyProven','radiusToleranceProven','globalG1Proven','embeddingProven','topologyAuthority'])
  ||c.method!=='interval-offset-contact-trim-curves'||c.scope!=='full-source-contact-branch'
  ||!closed(c,['originalWorldBoundaryIdentityProven','replacementFaceTrimsProven','stitchedTopologyProven','wholeCurveComplete','tangentToleranceProven','embeddingProven','topologyAuthority'])
  ||!Array.isArray(c.sourcePcurves)||c.sourcePcurves.length!==2||c.sourcePcurves.some((p,i)=>curveSnapshot(p)!==curveSnapshot(o.sourcePcurves[i]))
  ||typeof c.contactTrimCurvesProven!=='boolean'||typeof c.patchBoundaryIdentityProven!=='boolean')return false
 if(!record(c.fit)||!validEnvelopeFit(envelopeFitExpectation({...o,maxCells:o.maxFitCells}),{
  method:'interval-offset-finite-patch',scope:'constant-radius-contact-band',radiusMm:Math.abs(o.distances[0]),candidateSource:'authored',proposalSections:0,
  candidateSurface:value.candidateSurface,qualification:c.fit,reason:c.fit.reason,
  wholeCurveComplete:false,tangentToleranceProven:false,trimMembershipProven:false,embeddingProven:false,topologyAuthority:false,
 }))return false
 if(!count(c.uvCells,o.maxUvCells)||!count(c.agreementCells,o.maxAgreementCells)||!count(c.domainCells,o.maxDomainCells)
  ||!Array.isArray(c.sourceParameterReports)||c.sourceParameterReports.length>2||!Array.isArray(c.sourceMembershipReports)||c.sourceMembershipReports.length>2
  ||!Array.isArray(c.sourceAgreementReports)||c.sourceAgreementReports.length>2||!record(c.trim))return false
 for(const [i,p] of c.sourceParameterReports.entries()){
  if(!record(p)||p.side!==i||p.method!=='interval-contact-pcurve-correspondence'||!coverage(p.cells,'parameterInterval',o.fixedInterval)
   ||!count(p.visitedCells,o.maxUvCells)||p.sourceParameterCorrespondenceProven!==p.cells.every(cell=>cell.admitted===true&&finite(cell.errorUpperUv)&&cell.errorUpperUv<=o.toleranceUv))return false
 }
 const t=c.trim
 const base=c.fit.approximationWithinToleranceProven===true&&t.trimMembershipProven===true&&t.continuousBranchProven===true
  &&c.patchBoundaryIdentityProven===true&&Array.isArray(t.regions)&&t.regions.length===2&&t.regions.every(r=>record(r)&&r.valid===true)
  &&c.sourceParameterReports.length===2&&c.sourceParameterReports.every(p=>record(p)&&p.sourceParameterCorrespondenceProven===true)
  &&c.sourceMembershipReports.length===2&&c.sourceMembershipReports.every(p=>record(p)&&p.location==='inside')
  &&c.sourceAgreementReports.length===2&&c.sourceAgreementReports.every((p,i)=>record(p)&&p.side===i&&p.status==='within-tolerance'&&count(p.cells,o.maxAgreementCells))
 if(c.contactTrimCurvesProven!==base)return false
 if(c.patchBoundaryIdentityProven===true){
  if(!Array.isArray(c.worldCurves)||c.worldCurves.length!==2||!record(value.candidateSurface))return false
  const surface=value.candidateSurface as unknown as NurbsSurface
  for(let i=0;i<2;i++){const j=i===0?0:surface.controlPoints[0].length-1
   if(curveSnapshot(c.worldCurves[i])!==curveSnapshot({degree:surface.degreeU,knots:surface.knotsU,controlPoints:surface.controlPoints.map(row=>row[j]),weights:surface.weights.map(row=>row[j]),periodic:surface.periodicU??false}))return false
  }
 }
 if(!Array.isArray(q.tangentPlanes)||q.tangentPlanes.length!==(base?2:0)||!q.tangentPlanes.every(r=>validTangent(r,o)))return false
 const qualified=base&&q.tangentPlanes.every(r=>record(r)&&r.positionsProven===true&&r.tangentPlanesProven===true)
 return q.contactCurvesAndTangentPlanesProven===qualified&&q.reason===(qualified?'contact-tangent-planes-qualified':base?'contact-tangent-planes-unqualified':'contact-trim-curves-unqualified')
}
export const qualifyNurbsOffsetContacts=(options:OffsetContactQualificationOptions):OffsetContactQualification=>
 callNurbsRust('surface_offset_contact_qualification',options)
