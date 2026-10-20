import {describe,it,expect} from 'vitest'
import {validExpressionDisplay,type ExpressionDisplayLimits,type ExpressionDisplayReport} from '../src/services/expressionDisplay'
const limits:ExpressionDisplayLimits={divisions:2,cells:8,controlsPerEnclosure:1000,spans:1000,toleranceUv:1e-8,domainCells:512,domainCellsPerCell:32}
// Structural transport fixtures; they do not qualify any actual geometry.
const report=():ExpressionDisplayReport=>({trimQualified:false,enclosed:1,pending:1,spans:1,domainCells:16,inside:1,outside:0,unresolvedTrim:1,cells:[
 {face:0,domain:[[0,1],[0,1]],bounds:[[0,1],[0,1],[0,1]],location:'inside'},
 {face:1,domain:[[0,1],[0,1]],bounds:null,location:'unresolved'},
]})
describe('native expression display transport',()=>{
 it('keeps unresolved face coverage and independent work counters',()=>{
  expect(validExpressionDisplay(limits,2,report())).toBe(true)
  expect(validExpressionDisplay(limits,2,{...report(),trimQualified:true})).toBe(false)
  expect(validExpressionDisplay(limits,2,{...report(),unresolvedTrim:0,inside:2})).toBe(false)
  expect(validExpressionDisplay(limits,2,{...report(),enclosed:2,pending:0})).toBe(false)
 })
 it('rejects missing or duplicate faces and unbounded output',()=>{
  const value=report();value.cells[1]=structuredClone(value.cells[0])
  expect(validExpressionDisplay(limits,2,value)).toBe(false)
  expect(validExpressionDisplay(limits,3,report())).toBe(false)
  expect(validExpressionDisplay({...limits,divisions:0},2,report())).toBe(false)
  expect(validExpressionDisplay(limits,2,{...report(),spans:1001})).toBe(false)
  expect(validExpressionDisplay(limits,2,{...report(),domainCells:513})).toBe(false)
 })
 it('rejects nonfinite, reversed and malformed intervals',()=>{
  for(const bounds of [[[0,Infinity],[0,1],[0,1]],[[1,0],[0,1],[0,1]],[[0,1]]]){
   const value=report();value.cells[0].bounds=bounds as ExpressionDisplayReport['cells'][number]['bounds']
   expect(validExpressionDisplay(limits,2,value)).toBe(false)
  }
  const value=report();value.cells[1].domain[1]=[NaN,1]
  expect(validExpressionDisplay(limits,2,value)).toBe(false)
 })
})
