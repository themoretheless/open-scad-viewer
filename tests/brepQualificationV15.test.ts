import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'
const read=(path:string)=>JSON.parse(readFileSync(path,'utf8'))

describe('G8 append-only STEP /7 audit disposition',()=>{
  it('keeps /7 out of the shipped registry while blockers remain',()=>{
    const index=read('docs/qualification/plans/g8-full-matrix-index-v15.json')
    const matrix=read('docs/qualification/brep-full-closed-matrix-v15.json')
    const registry=read('docs/qualification/brep-capability-registry-release-full-v15.json')
    expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v14.json')
    expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v14.json')
    expect(registry.successorOf).toBe('docs/qualification/brep-capability-registry-release-full-v14.json')
    expect(index.capabilities.find((entry:{id:string})=>entry.id==='step-interchange/7'))
      .toMatchObject({maturity:'ResearchOnly',releaseState:'candidate'})
    expect(matrix.admittedOps).not.toContain('step-interchange/7')
    expect(matrix.pendingQualification).toContain('step-interchange/7')
    expect(registry.capabilities).not.toContain('step-interchange/7')
    expect(registry.excludedPendingQualification).toContain('step-interchange/7')
  })

  it('records the unavailable semantic checker without fabricating a pass',()=>{
    const plan=read('docs/qualification/plans/step-interchange-7.json')
    const evidence=read('docs/qualification/step-interchange-7-evidence-v12.json')
    const validator=read('tools/step-semantic-validator-manifest.json')
    expect(plan.lifecycle.status).toBe('rejected')
    expect(evidence).toMatchObject({state:'rejected',maturity:'ResearchOnly',
      runs:[{id:'step-interchange-7-browser-workbench-indexeddb',result:'pass'}]})
    expect(evidence.attestation.fabricatedRuns).toBe(false)
    expect(validator).toMatchObject({status:'unavailable',validator:null})
  })
})
