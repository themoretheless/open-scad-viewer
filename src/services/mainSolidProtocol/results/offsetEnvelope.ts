import type {OffsetEnvelope} from '../../nurbsSurfaceOffset'
import {finite,type MainSolidExpectation} from '../../mainSolidProtocol'
import {arrayOf} from '../mainSolidExpectation'
export function validOffsetEnvelope(job:Extract<MainSolidExpectation,{kind:'offsetEnvelope'}>,value:unknown,validateTangent:(job:MainSolidExpectation,value:unknown)=>boolean):boolean {
    const v=value as OffsetEnvelope
    if(!Number.isSafeInteger(job.maxCells)||job.maxCells<1||job.maxCells>1_000_000
      ||v.method!=='interval-rolling-ball-envelope'||v.scope!=='constant-radius-local-contact-band'
      ||v.radiusMm!==job.radiusMm||!finite(v.radiusMm)||v.radiusMm<=0
      ||v.wholeCurveComplete!==false||v.finiteNurbsPatchProven!==false||v.trimMembershipProven!==false
      ||v.embeddingProven!==false||v.topologyAuthority!==false||typeof v.envelopeRegularityProven!=='boolean'
      ||!validateTangent(job.tangent,v.centerTangent)||!Array.isArray(v.cells)
      ||!Number.isSafeInteger(v.visitedCells)||v.visitedCells<0||v.visitedCells>job.maxCells)return false
    if(v.cells.length===0)return v.visitedCells===0&&!v.envelopeRegularityProven
      &&(v.reason==='center-regularity-unproven'?!v.centerTangent.centerRegularityProven:
        v.centerTangent.centerRegularityProven&&['source-jet-enclosure-unresolved','antipodal-contact-unresolved','arc-weight-unresolved'].includes(v.reason))
    if(!v.centerTangent.centerRegularityProven||v.visitedCells!==2*v.cells.length-1)return false
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    let end=0,regular=true
    for(const c of v.cells) {
      if(!c||!interval(c.arcParameter)||c.arcParameter[0]!==end||c.arcParameter[1]<=end||c.arcParameter[1]>1
        ||!arrayOf(c.imageIntervalsMm,3,interval)||!arrayOf(c.centerDerivativeIntervalsMm,3,interval)
        ||!arrayOf(c.arcDerivativeIntervalsMm,3,interval)||!interval(c.areaSpeedIntervalMm2)
        ||c.areaSpeedIntervalMm2[0]<0||typeof c.regularityProven!=='boolean'
        ||c.regularityProven!==(c.areaSpeedIntervalMm2[0]>0))return false
      if(c.regularityProven&&(c.centerDerivativeIntervalsMm.every(r=>r[0]<=0&&r[1]>=0)
        ||c.arcDerivativeIntervalsMm.every(r=>r[0]<=0&&r[1]>=0)))return false
      if(c.regularityProven) {
        const mul=(a:number[],b:number[])=>{const p=[a[0]*b[0],a[0]*b[1],a[1]*b[0],a[1]*b[1]];return [Math.min(...p),Math.max(...p)]}
        const crossIncludesZero=[0,1,2].every(k=>{
          const a=mul(c.centerDerivativeIntervalsMm[(k+1)%3],c.arcDerivativeIntervalsMm[(k+2)%3])
          const b=mul(c.centerDerivativeIntervalsMm[(k+2)%3],c.arcDerivativeIntervalsMm[(k+1)%3])
          return a[0]-b[1]<=0&&a[1]-b[0]>=0
        })
        if(crossIncludesZero)return false
      }
      end=c.arcParameter[1];regular&&=c.regularityProven
    }
    return end===1&&v.envelopeRegularityProven===regular
      &&v.reason===(regular?'regular-local-envelope':'envelope-regularity-unproven')
  
}
