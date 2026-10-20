import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {expressionShellExpectation,validExpressionShell} from '../src/services/expressionShell'
const fixtures=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-support-tangency-dispatch-2026-10-09.json',import.meta.url),'utf8'))
it('accepts actual native support joins and bound-free exhausted work',()=>{
 for(const value of Object.values(fixtures) as any[]) {
  expect(validExpressionShell(expressionShellExpectation(value.request),value)).toBe(true)
 }
})
it('rejects missing boundaries, stale limits and fabricated body admission',()=>{
 const original=fixtures.admitted,expected=expressionShellExpectation(original.request)
 for(const change of [
  (v:any)=>{v.shell.supportTangencyCheck.boundaries.pop()},
  (v:any)=>{v.shell.supportTangencyCheck.neighborBodyQualified=true},
  (v:any)=>{v.shell.supportTangencyCheck.solidQualified=true},
  (v:any)=>{v.request.supportTangencyCheck.spans--},
  (v:any)=>{v.shell.supportTangencyCheck.cells++},
  (v:any)=>{v.shell.supportTangencyCheck.boundaries[0].pending=[[[0,1],[0,1]]]},
 ]) {
  const modified=structuredClone(original);change(modified)
  expect(validExpressionShell(expected,modified)).toBe(false)
 }
})
it('rejects a forged certificate when native support work is disabled',()=>{
 const modified=structuredClone(fixtures.unproven)
 modified.shell.supportTangencyCheck=structuredClone(fixtures.admitted.shell.supportTangencyCheck)
 expect(validExpressionShell(expressionShellExpectation(modified.request),modified)).toBe(false)
})

it('rejects contradictory proof status and reasons at report and seam level',()=>{
 const original=fixtures.admitted,expected=expressionShellExpectation(original.request)
 for(const change of [
  (v:any)=>{v.shell.supportTangencyCheck.reason='profile-transition-support-tangency-work-limit'},
  (v:any)=>{v.shell.supportTangencyCheck.boundaries[0].reason='full-section-join-regularity-unproven'},
  (v:any)=>{v.shell.supportTangencyCheck.boundaries[2].reason='full-section-join-qualified'},
 ]){
  const modified=structuredClone(original);change(modified)
  expect(validExpressionShell(expected,modified)).toBe(false)
 }
 const modified=structuredClone(fixtures.unproven)
 modified.shell.supportTangencyCheck.reason='profile-transition-support-tangency-qualified'
 expect(validExpressionShell(expressionShellExpectation(modified.request),modified)).toBe(false)
})
