import {readFileSync} from 'node:fs'
import {describe,it,expect} from 'vitest'
import {validExpressionRadiusRequest,validExpressionRadiusReport} from '../src/services/expressionRadiusCheck'
import {expressionShellExpectation,validExpressionShell} from '../src/services/expressionShell'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json',import.meta.url),'utf8'))
describe('native expression radius transport',()=>{
 for(const state of ['admitted','violation','unproven'])it(`accepts actual native ${state} response`,()=>{
  const result=fixture[state],request=result.request.radiusCheck
  expect(validExpressionRadiusRequest(request)).toBe(true)
  expect(validExpressionRadiusReport(request,result.shell.radiusCheck)).toBe(true)
  expect(validExpressionShell(expressionShellExpectation(result.request),result)).toBe(true)
 })
 it('rejects contradictory admission, violation and whole-solid flags',()=>{
  const result=fixture.violation,request=result.request.radiusCheck
  for(const mutation of [{qualified:true},{violationQualified:false},{solidQualified:true},{errorLowerMm:-1},{errorUpperMm:0},{spans:request.spans+1},{coefficientWork:request.coefficientWork+1},{errorLowerMm:NaN}]){
   expect(validExpressionRadiusReport(request,{...result.shell.radiusCheck,...mutation})).toBe(false)
  }
 })
 it('rejects missing reports and changed law identities',()=>{
  const source=fixture.admitted,expected=expressionShellExpectation(source.request)
  const missing=structuredClone(source);delete missing.shell.radiusCheck
  expect(validExpressionShell(expected,missing)).toBe(false)
  const stale=structuredClone(source);stale.request.radiusCheck.radius.controlPoints[0][0]+=0.25
  expect(validExpressionShell(expected,stale)).toBe(false)
 })
 it('rejects fabricated qualification and violation with disabled native work',()=>{
  const request=fixture.admitted.request.radiusCheck
  for(const disabled of ['controls','spans','coefficientWork']){
   const limits={...request,[disabled]:0}
   const invented={version:1,solidQualified:false,spans:0,coefficientWork:0,reason:'fabricated',
    errorLowerMm:0,errorUpperMm:0,qualified:true,violationQualified:false}
   expect(validExpressionRadiusReport(limits,invented)).toBe(false)
   expect(validExpressionRadiusReport(limits,{...invented,errorLowerMm:1,errorUpperMm:2,qualified:false,violationQualified:true})).toBe(false)
   expect(validExpressionRadiusReport(limits,{...invented,errorLowerMm:null,errorUpperMm:null,qualified:false})).toBe(true)
  }
 })
 it('rejects malformed and unbounded requests before dispatch',()=>{
  const request=fixture.admitted.request.radiusCheck
  for(const change of [{spans:100001},{coefficientWork:100000001},{controls:-1},{toleranceMm:0},{lawDomains:[[0,0],[0,1]]},{centers:null},{radius:false}])expect(validExpressionRadiusRequest({...request,...change})).toBe(false)
  expect(()=>expressionShellExpectation({...fixture.admitted.request,radiusCheck:null})).toThrow()
 })
})
