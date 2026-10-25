import {readFileSync} from 'node:fs'
import {expressionShellExpectation,validExpressionShell} from '../src/services/expressionShell'
import {describe,it,expect} from 'vitest'
import {validNeighborTangencyRequest,validNeighborTangencyReport} from '../src/services/expressionNeighborTangency'
describe('actual shell neighbor tangency transport',()=>{
 const request={controls:1000,cells:10000,spans:100000,maxSineSquared:1e-6}
 const pending={version:1,qualified:false,basis:'owned-full-Hermite-actual-shell-neighbors',upperSineSquared:null,cells:0,spans:0,reason:'budget-exhausted',solidQualified:false}
 it('rejects nonfinite angular tolerances and unsupported budgets',()=>{
  for(const maxSineSquared of [NaN,Infinity,-1,1])expect(validNeighborTangencyRequest({...request,maxSineSquared})).toBe(false)
  expect(validNeighborTangencyRequest({...request,cells:100001})).toBe(false)
  expect(validNeighborTangencyRequest(request)).toBe(true)
 })
 it('keeps a zero-work response unproven and rejects fabricated certificates',()=>{
  const zero={...request,spans:0}
  expect(validNeighborTangencyReport(zero,pending)).toBe(true)
  expect(validNeighborTangencyReport(zero,{...pending,qualified:true,upperSineSquared:[0,0,0,0]})).toBe(false)
  expect(validNeighborTangencyReport(request,{...pending,solidQualified:true})).toBe(false)
  expect(validNeighborTangencyReport(request,{...pending,upperSineSquared:[0,0,0,0]})).toBe(false)
 })
 it('rejects incomplete, nonfinite, over-tolerance and over-budget authority',()=>{
  const claimed={...pending,qualified:true,reason:'profile-transition-neighbor-tangency-qualified',cells:10,spans:20}
  expect(validNeighborTangencyReport(request,{...claimed,upperSineSquared:[0,0,0,0]})).toBe(true)
  for(const upperSineSquared of [[0,0,0],[0,0,NaN,0],[0,0,2e-6,0],[1e-10,0,0,0]])
   expect(validNeighborTangencyReport(request,{...claimed,upperSineSquared})).toBe(false)
  expect(validNeighborTangencyReport(request,{...claimed,upperSineSquared:[0,0,0,0],spans:100001})).toBe(false)
 })
})

it('validates native actual-neighbor reports and rejects stale angular requests',()=>{
 const fixtures=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json',import.meta.url),'utf8'))
 for(const value of Object.values(fixtures) as any[])expect(validExpressionShell(expressionShellExpectation(value.request),value)).toBe(true)
 const original=fixtures.admitted,expected=expressionShellExpectation(original.request)
 const stale=structuredClone(original)
 stale.request.neighborTangencyCheck.maxSineSquared=1e-7
 expect(validExpressionShell(expected,stale)).toBe(false)
 const forged=structuredClone(original)
 forged.shell.neighborTangencyCheck.solidQualified=true
 expect(validExpressionShell(expected,forged)).toBe(false)
})

it('rejects contradictory native tangency authority and reason',()=>{
 const fixtures=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json',import.meta.url),'utf8'))
 const expected=expressionShellExpectation(fixtures.admitted.request)
 for(const reason of ['profile-transition-neighbor-budget-exhausted','profile-transition-neighbor-angle-unproven','unknown-qualified-reason']){
  const value=structuredClone(fixtures.admitted)
  value.shell.neighborTangencyCheck.reason=reason
  expect(validExpressionShell(expected,value)).toBe(false)
 }
 const value=structuredClone(fixtures.unproven)
 value.shell.neighborTangencyCheck.reason='profile-transition-neighbor-tangency-qualified'
 expect(validExpressionShell(expressionShellExpectation(value.request),value)).toBe(false)
})
