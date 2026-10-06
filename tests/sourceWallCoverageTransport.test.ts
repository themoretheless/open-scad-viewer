import {describe,it,expect} from 'vitest'
import {coverageExpectation,validCoverage,type SourceWallCoverageOptions,type SourceWallCoverageResult} from '../src/services/sourceWallCoverageTransport'
const key=JSON.stringify
const request:SourceWallCoverageOptions={minimumMm:9.99,limits:{pairs:21,planeControls:100,normalSpans:100,gapCells:100,gapSpans:100,maxSineSquared:1e-6}}
function result():SourceWallCoverageResult {
 const pairs:SourceWallCoverageResult['pairs']=[]
 for(let a=0;a<6;a++)for(let b=a;b<6;b++)pairs.push({faces:[a,b],proven:true,reason:a===b?'source-wall-planar-self-excluded':'source-face-gap-qualified'})
 return {request:structuredClone(request),wholeWallQualified:true,lowerMm:9.99,enumerationComplete:true,totalPairs:21,pairs,planeControls:24,normalSpans:30,cells:3,spans:3,reason:'source-whole-wall-lower-qualified'}
}
describe('continuous original wall coverage transport',()=>{
 it('binds threshold and complete canonical pairs',()=>{
  expect(validCoverage(request,result(),6,key)).toBe(true)
  for(const mutate of [(r:SourceWallCoverageResult)=>r.request.minimumMm++,r=>r.lowerMm=10,r=>r.pairs[3].faces=[1,1],r=>r.pairs[3].proven=false,r=>r.cells=101,r=>delete r.pairs[3]]) {
   const r=result();mutate(r);expect(validCoverage(request,r,6,key)).toBe(false)
  }
 })
 it('checks native proof reason against pair class and proven status',()=>{
  const r=result();r.pairs[0].reason='source-wall-curved-self-excluded'
  expect(validCoverage(request,r,6,key)).toBe(true)
  for(const [index,reason] of [[0,'source-face-gap-qualified'],[1,'source-wall-curved-self-excluded'],[1,'source-wall-pair-unproven'],[0,'source-invented-exclusion']] as const){
   const changed=result();changed.pairs[index].reason=reason
   expect(validCoverage(request,changed,6,key)).toBe(false)
  }
 })
 it('preserves complete unresolved and truncated refusals',()=>{
  const r=result();r.pairs[1].proven=false;r.pairs[1].reason='source-face-gap-work-limit';r.wholeWallQualified=false;r.lowerMm=null;r.reason='source-wall-coverage-unproven'
  expect(validCoverage(request,r,6,key)).toBe(true)
  const partial=structuredClone(request);partial.limits.pairs=20
  r.request=partial;r.pairs.pop();r.enumerationComplete=false;r.reason='source-wall-coverage-pair-limit'
  expect(validCoverage(partial,r,6,key)).toBe(true)
  r.wholeWallQualified=true;expect(validCoverage(partial,r,6,key)).toBe(false)
 })
 it('clones bounded requests and rejects unrequested certificates',()=>{
  const e=coverageExpectation(request);e.limits.pairs=1;expect(request.limits.pairs).toBe(21)
  expect(validCoverage(undefined,result(),6,key)).toBe(false)
  for(const minimum of [0,-1,Infinity,NaN])expect(()=>coverageExpectation({...request,minimumMm:minimum})).toThrow()
  expect(()=>coverageExpectation({...request,limits:{...request.limits,gapCells:0}})).toThrow()
 })
})
