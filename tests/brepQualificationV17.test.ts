import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'
const read=(path:string)=>JSON.parse(readFileSync(path,'utf8'))

describe('G8 append-only STEP /9 qualification',()=>{
  it('ships bounded /9 without rewriting /8',()=>{
    const index=read('docs/qualification/plans/g8-full-matrix-index-v17.json')
    const matrix=read('docs/qualification/brep-full-closed-matrix-v17.json')
    const registry=read('docs/qualification/brep-capability-registry-release-full-v17.json')
    expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v16.json')
    expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v16.json')
    expect(registry.successorOf).toBe('docs/qualification/brep-capability-registry-release-full-v16.json')
    expect(index.capabilities.find((entry:{id:string})=>entry.id==='step-interchange/9'))
      .toMatchObject({maturity:'Qualified',releaseState:'shipped'})
    expect(matrix.admittedOps).toContain('step-interchange/9')
    expect(registry.capabilities).toContain('step-interchange/9')
    expect(read('docs/qualification/plans/step-interchange-8.json')).toMatchObject({
      capability:'step-interchange/8',successorOf:'step-interchange/7',
    })
  })

  it('binds scoped oracle, browser, and exact-gap boundaries',()=>{
    const plan=read('docs/qualification/plans/step-interchange-9.json')
    const evidence=read('docs/qualification/step-interchange-9-evidence-v12.json')
    const run=read('docs/qualification/step-interchange-9-run-v1.json')
    expect(plan).toMatchObject({capability:'step-interchange/9',successorOf:'step-interchange/8',
      lifecycle:{status:'qualified'},unresolvedRows:[]})
    expect(plan.claimBoundary.forbiddenClaims).toContain('Authored affine occurrence graphs')
    expect(plan.claimBoundary.forbiddenClaims).toContain('Tessellated strips, fans, texture coordinates, or presentation fidelity')
    expect(evidence).toMatchObject({state:'qualified',maturity:'Qualified',unresolvedRows:[]})
    expect(run.validator).toMatchObject({version:'0.9.1',
      sourceCommit:'ed686ee1d9cb8bf763ab8d61ef6d417c3b45c146'})
  })
})
