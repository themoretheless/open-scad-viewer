import {describe,it,expect} from 'vitest'
import {isSurfaceAuditReport} from '../src/services/surfaceSelfIntersectionProtocol'
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
