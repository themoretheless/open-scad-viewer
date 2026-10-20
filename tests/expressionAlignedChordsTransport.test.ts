import {describe,it,expect} from 'vitest'
import {validAlignedChordRequest,validAlignedChordReport,alignedChordParameterErrors} from '../src/services/expressionAlignedChords'
describe('aligned chord coverage transport',()=>{
 const request={minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:100,cells:10,spans:100,domainCells:100}
 const pending={version:1,qualified:false,wholeThicknessQualified:false,solidQualified:false,report:{
  method:'expression-support-aligned-chord-coverage',scope:'original-retained-transition-support-products',
  minimum_mm:1,max_sine_squared:1e-4,tolerance_uv:0.05,complete:false,cells:0,spans:0,domain_cells:0,total_pairs:1,initial_pairs_checked:0,
  intrinsic_checked:0,intrinsic_spans:0,intrinsic_unresolved:0,profile_transition_checked:0,profile_transition_excluded:0,profile_transition_spans:0,
  distance_excluded:0,angle_excluded:0,planar_excluded:0,intrinsic_excluded:0,plane_controls:0,cache_hits:0,symmetry_reused:0,trim_excluded:0,
  budget_pending:1,enclosure_pending:0,resolution_pending:0,pending:[{faces:[0,0],domains:[[[0,1],[0,1]],[[0,1],[0,1]]],pending_reason:'budget'}]}}
 it('validates optional transition outcome accounting and preserves legacy reports',()=>{
  expect(validAlignedChordReport(request,1,pending)).toBe(true)
  const current={...pending,report:{...pending.report,profile_transition_inconclusive:0,profile_transition_unresolved:0}}
  expect(validAlignedChordReport(request,1,current)).toBe(true)
  for(const patch of [{profile_transition_inconclusive:undefined},{profile_transition_unresolved:NaN},{profile_transition_unresolved:-1},{profile_transition_inconclusive:1}])
   expect(validAlignedChordReport(request,1,{...current,report:{...current.report,...patch}})).toBe(false)
 })
 it('accepts zero-work refusal without promoting thickness or solid admission',()=>{
  expect(validAlignedChordReport({...request,cells:0,spans:0,domainCells:0},1,pending)).toBe(true)
  for(const patch of [{qualified:true},{wholeThicknessQualified:true},{solidQualified:true}])expect(validAlignedChordReport(request,1,{...pending,...patch})).toBe(false)
 })
 it('rejects stale thresholds, malformed regions and inconsistent work',()=>{
  for(const mutate of [(v:any)=>v.report.minimum_mm=2,(v:any)=>v.report.pending[0].faces=[0,1],(v:any)=>v.report.pending[0].domains[0][0]=[1,0],(v:any)=>v.report.intrinsic_spans=1,(v:any)=>v.report.budget_pending=0,(v:any)=>v.report.intrinsic_unresolved=1]){
   const v=structuredClone(pending);mutate(v);expect(validAlignedChordReport(request,1,v)).toBe(false)
  }
 })
 it('requires all original pairs before accepting complete coverage',()=>{
  const complete={...pending,qualified:true,report:{...pending.report,complete:true,cells:1,initial_pairs_checked:1,budget_pending:0,pending:[]}}
  expect(validAlignedChordReport(request,1,complete)).toBe(true)
  expect(validAlignedChordReport(request,1,{...complete,report:{...complete.report,initial_pairs_checked:0}})).toBe(false)
 })
 it('rejects fabricated completed coverage with no processed cells',()=>{
  const forged={...pending,qualified:true,report:{...pending.report,complete:true,initial_pairs_checked:1,budget_pending:0,pending:[]}}
  expect(validAlignedChordReport(request,1,forged)).toBe(false)
  const doubleCounted={...pending,report:{...pending.report,cells:1,angle_excluded:1,distance_excluded:1}}
  expect(validAlignedChordReport(request,1,doubleCounted)).toBe(false)
 })
 it('bounds aggregate plane controls by the per-face native limit',()=>{
  const complete={...pending,qualified:true,report:{...pending.report,complete:true,cells:3,total_pairs:3,initial_pairs_checked:3,budget_pending:0,pending:[],plane_controls:200}}
  expect(validAlignedChordReport(request,2,complete)).toBe(true)
  expect(validAlignedChordReport(request,2,{...complete,report:{...complete.report,plane_controls:201}})).toBe(false)
 })
 it('requires every supplied span stage to be finite, integral and exactly accounted',()=>{
  const stages={position:1,normal:2,shared_corner:3,original_pair:4,retained_pair:5,corrected_pair:6}
  const value={...pending,report:{...pending.report,spans:21,span_stages:stages}}
  expect(validAlignedChordReport(request,1,value)).toBe(true)
  for(const patch of [{normal:1},{normal:NaN},{normal:-1},{normal:0.5},{normal:undefined},{normal:101}]){
   expect(validAlignedChordReport(request,1,{...value,report:{...value.report,span_stages:{...stages,...patch}}})).toBe(false)
  }
  expect(validAlignedChordReport(request,1,{...value,report:{...value.report,span_stages:null}})).toBe(false)
 })
 it('rejects nonfinite or unbounded requests',()=>{
  expect(validAlignedChordRequest(request)).toBe(true)
  for(const patch of [{minimumMm:0},{minimumMm:NaN},{maxSineSquared:1},{toleranceUv:0},{controls:1000001},{cells:100001},{spans:1000001},{domainCells:Infinity}])expect(validAlignedChordRequest({...request,...patch})).toBe(false)
 })
})

it('identifies invalid editable parameters with the same request bounds',()=>{
 const base={minimumMm:1,maxSineSquared:0,toleranceUv:1,controls:1000,cells:10000,spans:100000,domainCells:100000}
 expect(alignedChordParameterErrors(base)).toEqual({minimumMm:false,maxSineSquared:false,toleranceUv:false})
 for(const [field,value] of [['minimumMm',0],['minimumMm',NaN],['minimumMm',''],['toleranceUv',0],['toleranceUv',Infinity],['toleranceUv',1.01],['maxSineSquared',-0.1],['maxSineSquared',1]] as const){
  const request={...base,[field]:value}
  expect(alignedChordParameterErrors(request)[field]).toBe(true)
  expect(validAlignedChordRequest(request)).toBe(false)
 }
 expect(validAlignedChordRequest({...base,minimumMm:1e-12})).toBe(true)
 expect(alignedChordParameterErrors({...base,spans:1000001})).toEqual(alignedChordParameterErrors(base))
 expect(validAlignedChordRequest({...base,spans:1000001})).toBe(false)
})
