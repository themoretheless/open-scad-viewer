import {readFileSync} from 'node:fs'
import {describe,it,expect} from 'vitest'
import {expressionInspectionExpectation,validExpressionInspection,type ExpressionInspectionOptions} from '../src/services/expressionShellInspection'
import {mainSolidExpectation,mainSolidResult,MAIN_SOLID_JOB_KINDS} from '../src/services/mainSolidProtocol'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-native-2026-10-09.json',import.meta.url),'utf8'))
const {op:_,...options}=fixture.request as ExpressionInspectionOptions&{op:string}
describe('native expression inspection transport',()=>{
 it('preserves actual native unresolved contact coverage',()=>{
  const pending=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-pending-2026-10-09.json',import.meta.url),'utf8'))
  const expected=expressionInspectionExpectation(pending.request)
  expect(validExpressionInspection(expected,pending)).toBe(true)
  expect(pending.shell.inspection.contacts.pendingPairs).toBeGreaterThan(0)
  expect(validExpressionInspection(expected,{...pending,embeddingQualified:true})).toBe(false)
 })
 it('accepts actual native complete contacts and volume without solid admission',()=>{
  expect(MAIN_SOLID_JOB_KINDS.expressionShellInspect).toBe(true)
  const expected=mainSolidExpectation({kind:'expressionShellInspect',options})
  expect(mainSolidResult(expected,fixture.result)).toBe(true)
  expect(mainSolidResult(expected,{...fixture.result,materialQualified:true})).toBe(false)
 })
 it('preserves projected-region refusal without promoting embedding',()=>{
  const {op:_,...request}=fixture.refusal.request as ExpressionInspectionOptions&{op?:string}
  expect(validExpressionInspection(expressionInspectionExpectation(request),fixture.refusal)).toBe(true)
  expect(validExpressionInspection(expressionInspectionExpectation(request),{...fixture.refusal,embeddingQualified:true})).toBe(false)
 })
 it('rejects missing, duplicated and nonfinite contact coverage',()=>{
  const expected=expressionInspectionExpectation(options)
  for(const change of ['missing','duplicate','nonfinite'] as const){
   const result=structuredClone(fixture.result),c=result.shell.inspection.contacts
   if(change==='missing')c.pairs.pop()
   if(change==='duplicate')c.pairs[1]=structuredClone(c.pairs[0])
   if(change==='nonfinite')c.pairs[0].pendingProducts[0][0][0][0]=Infinity
   expect(validExpressionInspection(expected,result),change).toBe(false)
  }
 })
 it('rejects forged pending counts and volume bounds',()=>{
  const expected=expressionInspectionExpectation(options)
  const pending=structuredClone(fixture.result);pending.shell.inspection.contacts.pairs[0].proof='pending'
  expect(validExpressionInspection(expected,pending)).toBe(false)
  const volume=structuredClone(fixture.result);volume.shell.inspection.volume.positiveBounds[0]=0
  expect(validExpressionInspection(expected,volume)).toBe(false)
  const budget=structuredClone(fixture.result);budget.shell.inspection.contacts.exactWork=options.inspectionLimits.contacts.exactWork+1
  expect(validExpressionInspection(expected,budget)).toBe(false)
 })
})

it('keeps aligned chord coverage absent until embedding is natively qualified',()=>{
 const pending=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-pending-2026-10-09.json',import.meta.url),'utf8'))
 pending.request.alignedChordCheck={minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:100,cells:0,spans:0,domainCells:0}
 pending.shell.alignedChordCheck=null
 const expected=expressionInspectionExpectation(pending.request)
 expect(validExpressionInspection(expected,pending)).toBe(true)
 const forged=structuredClone(pending);forged.shell.alignedChordCheck={version:1,qualified:true,wholeThicknessQualified:true,solidQualified:true}
 expect(validExpressionInspection(expected,forged)).toBe(false)
 const stale=structuredClone(pending);stale.request.alignedChordCheck.minimumMm=2
 expect(validExpressionInspection(expected,stale)).toBe(false)
 const missing=structuredClone(pending);delete missing.shell.alignedChordCheck
 expect(validExpressionInspection(expected,missing)).toBe(false)
})
