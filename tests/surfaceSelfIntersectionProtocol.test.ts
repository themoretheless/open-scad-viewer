import {describe,it,expect} from 'vitest'
import {isSurfaceAuditReport} from '../src/kernel/surfaceSelfIntersectionProtocol'
function present():any{return{schema:'nurbs-surface-self-intersection/1',status:'present',presenceProven:true,absenceProven:false,allContactBranchesEnumerated:false,witness:null,exactWitness:[[{numerator:'1',denominator:'12'},{numerator:'1',denominator:'2'}],[{numerator:'1',denominator:'4'},{numerator:'1',denominator:'2'}]],witnessCoordinates:'normalized-original-active-domain',reflectedAxis:0,injectivity:{proven:false,spans:100,reason:'work-limit'},maxSpans:100}}
describe('surface audit Worker boundary',()=>{
 it('keeps non-dyadic witnesses as exact strings',()=>{const r=present();expect(isSurfaceAuditReport(r)).toBe(true);expect(r.exactWitness[0][0].numerator).toBe('1')})
 it('rejects rounded, malformed and oversized exact scalars',()=>{
  for(const scalar of [{numerator:1,denominator:12},{numerator:'1',denominator:'0'},{numerator:'1e30',denominator:'2'},{numerator:'1',denominator:'2'.repeat(4097)}]){
   const r=present();r.exactWitness[0][0]=scalar;expect(isSurfaceAuditReport(r)).toBe(false)
  }
 })
 it('rejects contradictory verdicts and branch claims',()=>{
  for(const change of [{absenceProven:true},{presenceProven:false},{allContactBranchesEnumerated:true},{exactWitness:null},{maxSpans:0},{injectivity:{proven:false,spans:101,reason:'budget'}},{injectivity:{proven:true,spans:1,reason:'contradiction'}}])expect(isSurfaceAuditReport({...present(),...change})).toBe(false)
 })
 it('rejects invalid normalized floating witnesses without rounding',()=>{
  for(const witness of [[[0,.5],[.75,.5]],[[.25,.5],[1,.5]],[[NaN,.5],[.75,.5]],[[.25,.5]]])expect(isSurfaceAuditReport({...present(),witness})).toBe(false)
 })
 it('unresolved reports cannot smuggle a proof pair',()=>{
  const r={...present(),status:'unresolved',presenceProven:false};expect(isSurfaceAuditReport(r)).toBe(false)
  expect(isSurfaceAuditReport({...r,exactWitness:null})).toBe(true)
 })
})

describe('certified nonlinear UV boxes',()=>{
 const report=()=>({...present(),exactWitness:null,certifiedCrossing:{method:'original-planar-krawczyk',parameterBoxes:[[[.27,.28],[.5,.5]],[[.83,.84],[.5,.5]]],positionBounds:[[.1,.12],[.1,.12],[1,1]],cells:25}})
 it('accepts interval evidence without inventing an exact point',()=>expect(isSurfaceAuditReport(report())).toBe(true))
 it('rejects overlapping preimages, invalid enclosures and work claims',()=>{
  const a=report();a.certifiedCrossing.parameterBoxes[1]=a.certifiedCrossing.parameterBoxes[0]!;expect(isSurfaceAuditReport(a)).toBe(false)
  const b=report();b.certifiedCrossing.positionBounds[0]=[NaN,1];expect(isSurfaceAuditReport(b)).toBe(false)
  const c=report();c.certifiedCrossing.cells=101;expect(isSurfaceAuditReport(c)).toBe(false)
  const d=report();d.certifiedCrossing.parameterBoxes[0][0]=[.3,.2];expect(isSurfaceAuditReport(d)).toBe(false)
 })
})

describe('finite automatic probe metadata',()=>{
 const search=()=>({attempts:[{axis:0,parameter:.25,chargedCells:5,isoCoverageComplete:false,refusal:null}],chargedCells:5,budgetExhausted:false,surfaceCoverageComplete:false})
 it('admits bounded metadata without granting whole-surface coverage',()=>expect(isSurfaceAuditReport({...present(),probeSearch:search()})).toBe(true))
 it('rejects a global coverage claim or inconsistent resource receipt',()=>{
  expect(isSurfaceAuditReport({...present(),probeSearch:{...search(),surfaceCoverageComplete:true}})).toBe(false)
  expect(isSurfaceAuditReport({...present(),probeSearch:{...search(),chargedCells:6}})).toBe(false)
  expect(isSurfaceAuditReport({...present(),probeSearch:{...search(),attempts:Array(43).fill(search().attempts[0]),chargedCells:215}})).toBe(false)
 })
 it('complete sampled isos still leave the surface unresolved',()=>{
  const r={...present(),status:'unresolved',presenceProven:false,exactWitness:null,probeSearch:{...search(),attempts:[{...search().attempts[0],isoCoverageComplete:true}]}}
  expect(isSurfaceAuditReport(r)).toBe(true)
 })
})

describe('3D curve-surface source proofs',()=>{
 const report=()=>({...present(),reflectedAxis:null,exactWitness:null,certifiedCrossing:{method:'original-curve-surface-krawczyk',parameterBoxes:[[[.24,.26],[.5,.5]],[[.74,.76],[.44,.46]]],positionBounds:[[-.01,.01],[-.01,.01],[1.,1.01]],cells:50},probeSearch:{attempts:[{kind:'curve-surface-forward',axis:0,parameter:.5,chargedCells:50,isoCoverageComplete:false,refusal:null}],chargedCells:50,budgetExhausted:false,surfaceCoverageComplete:false}})
 it('accepts different transverse UVs with an explicit 3D proof kind',()=>expect(isSurfaceAuditReport(report())).toBe(true))
 it('rejects an unknown proof method',()=>{const r=report();r.certifiedCrossing.method='sampled-root';expect(isSurfaceAuditReport(r)).toBe(false)})
})
