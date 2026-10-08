import {expect,it} from 'vitest'
import {mainSolidResult} from '../src/services/mainSolidProtocol'

const refused=()=>({document:null,error:'Native continuous certificate exhausted',report:{
 accepted:false,sampledControlDeviation:0,budget:.001,stations:17,sections:5,
 continuousBound:false,method:'double-reflection-fourfold-section-refinement',
 continuousCertificate:{errorUpper:null,withinBudget:false,cells:0,maxCells:0,
  method:'native-interval',reason:'cell-budget-exhausted',
  scope:'matched-parameter-profile-deviation',regularityCertified:false,
  globalEmbeddingCertified:false,seamSmoothnessCertified:false},
}})
const accepts=(value:unknown)=>mainSolidResult({kind:'surfaceBuild'},value)

it('retains a native refusal even when the sampled diagnostic is zero',()=>{
 expect(accepts(refused())).toBe(true)
 const forged=refused();forged.report.accepted=true
 expect(accepts(forged)).toBe(false)
})
it('rejects null and incomplete continuous reports without throwing',()=>{
 for(const continuousCertificate of [null,{}, {errorUpper:0,withinBudget:true}]) {
  const value={...refused(),report:{...refused().report,continuousCertificate}}
  expect(accepts(value)).toBe(false)
 }
})
it('does not accept a smooth-seam label without exact native seam evidence',()=>{
 const value={...refused(),report:{...refused().report,seamContinuity:'G2'}}
 expect(accepts(value)).toBe(false)
})
