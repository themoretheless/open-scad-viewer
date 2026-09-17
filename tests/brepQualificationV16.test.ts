import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'
const read=(path:string)=>JSON.parse(readFileSync(path,'utf8'))

describe('G8 append-only STEP /8 qualification',()=>{
  it('ships /8 while preserving rejected /5 through /7',()=>{
    const index=read('docs/qualification/plans/g8-full-matrix-index-v16.json')
    const matrix=read('docs/qualification/brep-full-closed-matrix-v16.json')
    const registry=read('docs/qualification/brep-capability-registry-release-full-v16.json')
    expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v15.json')
    expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v15.json')
    expect(registry.successorOf).toBe('docs/qualification/brep-capability-registry-release-full-v15.json')
    expect(index.capabilities.find((entry:{id:string})=>entry.id==='step-interchange/8'))
      .toMatchObject({maturity:'Qualified',releaseState:'shipped'})
    expect(matrix.admittedOps).toContain('step-interchange/8')
    expect(registry.capabilities).toContain('step-interchange/8')
    for(const id of ['step-interchange/5','step-interchange/6','step-interchange/7']){
      expect(matrix.pendingQualification).toContain(id)
      expect(registry.excludedPendingQualification).toContain(id)
    }
  })

  it('binds real external, whole-domain, sense, and browser artifacts',()=>{
    const plan=read('docs/qualification/plans/step-interchange-8.json')
    const evidence=read('docs/qualification/step-interchange-8-evidence-v12.json')
    const run=read('docs/qualification/step-interchange-8-run-v1.json')
    expect(plan).toMatchObject({capability:'step-interchange/8',lifecycle:{status:'qualified'},unresolvedRows:[]})
    expect(evidence).toMatchObject({state:'qualified',maturity:'Qualified',unresolvedRows:[]})
    expect(evidence.runs.map((entry:{artifactHash:string})=>entry.artifactHash)).toEqual([
      'c8ee3ec2394fe1e3e06785518f544d334862165f825e859d9bcc4c3003c526c2',
      'dbf3bde4c6508e151cdc9fade2886cc86d2cc08e4bd4d99b25c74d48fa3a2087',
      '6abd7a82d47cfc59ec395b5d772b442ea25873336c67eabf22257c067ab37e79',
    ])
    expect(run.validator).toMatchObject({sourceCommit:'ed686ee1d9cb8bf763ab8d61ef6d417c3b45c146',
      executableSha256:'a9258791296493ed5de470b1e98f8ba44dbb1bdd7fd8b2e29662bade36e07043',
      schemaSha256:'e7e93cf97880fd87d634e4b9ee58400da0a1be6c06a1de76ec13807ecdc15ccb'})
  })
})
