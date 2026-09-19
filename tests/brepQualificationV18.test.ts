import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'
const read=(path:string)=>JSON.parse(readFileSync(path,'utf8'))

describe('G8 append-only STEP /10 qualification',()=>{
 it('ships /10 without rewriting /9',()=>{
  const index=read('docs/qualification/plans/g8-full-matrix-index-v18.json')
  const matrix=read('docs/qualification/brep-full-closed-matrix-v18.json')
  const registry=read('docs/qualification/brep-capability-registry-release-full-v18.json')
  expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v17.json')
  expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v17.json')
  expect(registry.successorOf).toBe('docs/qualification/brep-capability-registry-release-full-v17.json')
  expect(index.capabilities.find((entry:{id:string})=>entry.id==='step-interchange/10'))
   .toMatchObject({maturity:'Qualified',releaseState:'shipped'})
  expect(matrix.admittedOps).toContain('step-interchange/10')
  expect(registry.capabilities).toContain('step-interchange/10')
  expect(read('docs/qualification/plans/step-interchange-9.json')).toMatchObject({
   capability:'step-interchange/9',successorOf:'step-interchange/8',
  })
 })

 it('binds actual product, browser and scope-audit evidence',()=>{
  const plan=read('docs/qualification/plans/step-interchange-10.json')
  const evidence=read('docs/qualification/step-interchange-10-evidence-v12.json')
  const run=read('docs/qualification/step-interchange-10-run-v1.json')
  expect(plan).toMatchObject({capability:'step-interchange/10',successorOf:'step-interchange/9',
   lifecycle:{status:'qualified'},unresolvedRows:[]})
  expect(plan.claimBoundary.forbiddenClaims).toContain('Implicit filesystem or network resolution')
  expect(plan.claimBoundary.forbiddenClaims).toContain('Procedural CSG evaluation or conversion')
  expect(evidence).toMatchObject({state:'qualified',maturity:'Qualified',unresolvedRows:[]})
  expect(run.scopeAudit.remainingRequiredGeometryProductStepGaps).toEqual([])
  expect(run.graphOracle).toMatchObject({definitions:1,occurrences:3,exactSourceRoundtrip:true})
 })
})
