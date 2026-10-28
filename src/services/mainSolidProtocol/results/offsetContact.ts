import type {OffsetContactBand,OffsetContactTangent,OffsetSourceBoundary,TrimmedOffsetContactBand} from '../../nurbsSurfaceOffset'
import {finite,type MainSolidExpectation} from '../../mainSolidProtocol'
import {arrayOf} from '../mainSolidExpectation'
export type OffsetContactExpectation=Extract<MainSolidExpectation,{kind:'offsetContactTangent'|'offsetSourceBoundary'|'trimmedOffsetContactBand'|'offsetContactBand'}>
export function validOffsetContact(job:OffsetContactExpectation,value:unknown,mainSolidResult:(job:MainSolidExpectation,value:unknown)=>boolean):boolean {
  if(job.kind==='offsetContactTangent') {
    const v=value as OffsetContactTangent,unique=v.contactStatus==='continuous-branch'
    if(v.method!=='interval-offset-center-tangent'||v.scope!=='constant-offset-contact-band'
      ||v.envelopeRegularityProven!==false||v.trimMembershipProven!==false||v.topologyAuthority!==false||v.wholeCurveComplete!==false
      ||typeof v.centerRegularityProven!=='boolean')return false
    if(!mainSolidResult({kind:'offsetContactBand',fixedAxis:job.fixedAxis,domains:job.domains},
      {method:'interval-offset-band-krawczyk',scope:'parameter-band-within-tube',status:v.contactStatus,witness:v.contactWitness,
       rootForEveryParameterProven:unique,uniqueWithinTube:unique,continuousBranchProven:unique,wholeCurveComplete:false,trimMembershipProven:false,topologyAuthority:false}))return false
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    if(v.centerTangentIntervalsMm===null)return v.parameterDerivativeIntervals===null&&v.speedIntervalMm===null&&!v.centerRegularityProven
      &&(v.reason==='offset-carriers-excluded'?v.contactStatus==='excluded':v.reason==='contact-band-unresolved'?v.contactStatus==='unresolved':unique&&['source-jet-continuity-unproven','offset-jacobian-unresolved','implicit-jacobian-unresolved'].includes(v.reason))
    if(!unique||!arrayOf(v.centerTangentIntervalsMm,3,interval)||!arrayOf(v.parameterDerivativeIntervals,3,interval)
      ||!interval(v.speedIntervalMm)||v.speedIntervalMm![0]<0||v.centerRegularityProven!==(v.speedIntervalMm![0]>0))return false
    if(v.centerRegularityProven&&v.centerTangentIntervalsMm.every(r=>r[0]<=0&&r[1]>=0))return false
    return v.reason===(v.centerRegularityProven?'regular-center-tangent':'center-regularity-unproven')
  }
  if(job.kind==='offsetSourceBoundary') {
    const v=value as OffsetSourceBoundary,count=(n:unknown,max:number)=>Number.isSafeInteger(n)&&Number(n)>=0&&Number(n)<=max
    if(v.method!=='offset-source-boundary-admission'||v.scope!=='original-trims-and-spatial-coedges'
      ||v.topologyAuthority!==false||v.wholeCurveComplete!==false||!mainSolidResult(job.trim,v.trim)
      ||typeof v.worldCoedgeIdentityProven!=='boolean'||typeof v.worldBoundaryWithinToleranceProven!=='boolean'
      ||v.toleranceMm!==job.toleranceMm||v.maxExactWork!==job.maxExactWork||v.maxAgreementCells!==job.maxAgreementCells
      ||v.totalCoedges!==job.addresses.length||!count(v.checkedCoedges,v.totalCoedges)
      ||!count(v.exactWork,job.maxExactWork)||!count(v.agreementCells,job.maxAgreementCells)
      ||!Array.isArray(v.audits)||v.audits.length>v.totalCoedges)return false
    if(!v.audits.every((a,i)=>{
      if(!a||a.side!==job.addresses[i][0]||a.loop!==job.addresses[i][1]||a.curve!==job.addresses[i][2]
        ||!['equal','different','unresolved','unsupported'].includes(a.exactStatus)
        ||!count(a.exactWork,job.maxExactWork)||!count(a.cells,job.maxAgreementCells))return false
      if(a.agreementStatus==='exact')return a.exactStatus==='equal'&&a.cells===0&&a.witnessParameter===null&&a.witnessDistanceMm===null
      if(a.exactStatus==='equal')return false
      if(a.agreementStatus==='mismatch')return a.cells>0&&finite(a.witnessParameter)&&a.witnessParameter!>=0&&a.witnessParameter!<=1
        &&arrayOf(a.witnessDistanceMm,2,finite)&&a.witnessDistanceMm![0]>job.toleranceMm&&a.witnessDistanceMm![1]>=a.witnessDistanceMm![0]
      return ['within-tolerance','unresolved'].includes(a.agreementStatus)&&a.witnessParameter===null&&a.witnessDistanceMm===null
        &&(a.agreementStatus!=='within-tolerance'||a.cells>0)
    }))return false
    if(v.exactWork!==v.audits.reduce((n,a)=>n+a.exactWork,0)||v.agreementCells!==v.audits.reduce((n,a)=>n+a.cells,0))return false
    const accepted=(a:OffsetSourceBoundary['audits'][number])=>a.agreementStatus==='exact'||a.agreementStatus==='within-tolerance'
    if(v.worldBoundaryWithinToleranceProven){
      const allExact=v.audits.every(a=>a.exactStatus==='equal')
      return v.trim.trimMembershipProven&&v.checkedCoedges===v.totalCoedges&&v.audits.length===v.totalCoedges
        &&v.audits.every(accepted)&&v.worldCoedgeIdentityProven===allExact
        &&v.reason===(allExact?'exact-source-boundaries':'source-boundaries-within-tolerance')
    }
    if(v.worldCoedgeIdentityProven)return false
    if(!v.trim.trimMembershipProven)return v.reason==='contact-not-admitted-on-trims'&&v.checkedCoedges===0&&v.audits.length===0
    if(!v.audits.length||!v.audits.slice(0,-1).every(accepted))return false
    const last=v.audits.at(-1)!
    if(v.reason==='boundary-work-limit')return last.agreementStatus==='unresolved'&&last.cells===0
      &&v.agreementCells===job.maxAgreementCells&&v.checkedCoedges===v.audits.length-1
    return v.checkedCoedges===v.audits.length&&((v.reason==='spatial-boundary-mismatch'&&last.agreementStatus==='mismatch')
      ||(v.reason==='spatial-boundary-unresolved'&&last.agreementStatus==='unresolved'))
  }
  if(job.kind==='trimmedOffsetContactBand') {
    const v=value as TrimmedOffsetContactBand,unique=v.contactStatus==='continuous-branch'
    if(v.method!=='interval-offset-band-trim-admission'||v.scope!=='audited-original-uv-regions'
      ||v.worldCoedgeIdentityProven!==false||v.topologyAuthority!==false||v.wholeCurveComplete!==false
      ||typeof v.trimMembershipProven!=='boolean'||v.continuousBranchProven!==unique)return false
    if(!mainSolidResult({kind:'offsetContactBand',fixedAxis:job.fixedAxis,domains:job.domains},
      {method:'interval-offset-band-krawczyk',scope:'parameter-band-within-tube',status:v.contactStatus,witness:v.witness,
       rootForEveryParameterProven:unique,uniqueWithinTube:unique,continuousBranchProven:v.continuousBranchProven,
       wholeCurveComplete:v.wholeCurveComplete,trimMembershipProven:false,topologyAuthority:v.topologyAuthority}))return false
    const count=(n:unknown,max:number)=>Number.isSafeInteger(n)&&Number(n)>=0&&Number(n)<=max
    if(!count(v.pairs,job.maxPairs)||!count(v.cells,job.maxCells)||!count(v.domainCells,job.maxDomainCells)
      ||!Array.isArray(v.regions)||v.regions.length>2||!Array.isArray(v.classifications)||v.classifications.length>2)return false
    if(!v.regions.every((r,i)=>r&&[true,false,null].includes(r.valid)&&typeof r.reason==='string'
      &&count(r.pairs,job.maxPairs)&&count(r.cells,job.maxCells)&&count(r.domainCells,job.maxDomainCells)
      &&(r.problemLoops===null||arrayOf(r.problemLoops,2,n=>count(n,job.loopCounts[i]-1)))))return false
    if(!v.classifications.every(c=>c&&['inside','outside','unresolved'].includes(c.location)&&typeof c.reason==='string'&&count(c.cells,job.maxDomainCells)))return false
    if(v.pairs!==v.regions.reduce((n,r)=>n+r.pairs,0)||v.cells!==v.regions.reduce((n,r)=>n+r.cells,0)
      ||v.domainCells!==v.regions.reduce((n,r)=>n+r.domainCells,0)+v.classifications.reduce((n,c)=>n+c.cells,0))return false
    if(v.trimMembershipProven)return unique&&v.reason==='contact-band-inside-trims'
      &&v.regions.length===2&&v.regions.every(r=>r.valid===true)
      &&v.classifications.length===2&&v.classifications.every(c=>c.location==='inside')
    switch(v.reason){
      case 'offset-carriers-excluded':return v.contactStatus==='excluded'&&v.regions.length===0&&v.classifications.length===0
      case 'offset-contact-unresolved':return v.contactStatus==='unresolved'&&v.regions.length===0&&v.classifications.length===0
      case 'invalid-trim-region':return unique&&v.regions.at(-1)?.valid===false&&v.classifications.length===0
      case 'trim-region-unresolved':return unique&&v.regions.at(-1)?.valid===null&&v.classifications.length===0
      case 'contact-outside-trim':return unique&&v.regions.length===2&&v.regions.every(r=>r.valid===true)&&v.classifications.at(-1)?.location==='outside'
      case 'contact-trim-unresolved':return unique&&v.regions.length===2&&v.regions.every(r=>r.valid===true)&&v.classifications.at(-1)?.location==='unresolved'
      case 'trim-work-limit':return unique&&(v.pairs===job.maxPairs||v.cells===job.maxCells||v.domainCells===job.maxDomainCells)
      default:return false
    }
  }
  if(job.kind==='offsetContactBand') {
    const v=value as OffsetContactBand
    const unique=v.status==='continuous-branch'
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    if(v.method!=='interval-offset-band-krawczyk'||v.scope!=='parameter-band-within-tube'
      ||!['continuous-branch','excluded','unresolved'].includes(v.status)
      ||v.rootForEveryParameterProven!==unique||v.uniqueWithinTube!==unique||v.continuousBranchProven!==unique
      ||v.wholeCurveComplete!==false||v.trimMembershipProven!==false||v.topologyAuthority!==false)return false
    if(!unique)return v.witness===null
    const w=v.witness
    if(!w||!finite(w.contractionUpper)||w.contractionUpper<0||w.contractionUpper>=.5
      ||!arrayOf(w.centerIntervalMm,3,interval)||!arrayOf(w.firstUV,2,interval)||!arrayOf(w.secondUV,2,interval))return false
    return [w.firstUV,w.secondUV].every((uv,side)=>uv.every((r,axis)=>{
      const d=job.domains[side][axis]
      return side===0&&axis===job.fixedAxis?r[0]===d[0]&&r[1]===d[1]:r[0]>d[0]&&r[1]<d[1]
    }))
  }
  return false
}
