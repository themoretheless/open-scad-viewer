import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'

const read=(path:string)=>JSON.parse(readFileSync(path,'utf8'))

describe('G8 append-only STEP /5 release',()=>{
  it('advances index, matrix, and registry without rewriting v12',()=>{
    const index=read('docs/qualification/plans/g8-full-matrix-index-v13.json')
    const matrix=read('docs/qualification/brep-full-closed-matrix-v13.json')
    const registry=read('docs/qualification/brep-capability-registry-release-full-v13.json')
    expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v12.json')
    expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v12.json')
    expect(registry.successorOf).toBe('docs/qualification/brep-capability-registry-release-full-v12.json')
    expect(index.capabilities.find((entry:{id:string})=>entry.id==='step-interchange/5')).toMatchObject({maturity:'Qualified',releaseState:'shipped'})
    expect(matrix.admittedOps).toContain('step-interchange/5')
    expect(registry.capabilities).toContain('step-interchange/5')
  })

  it('records the audit rejection without rewriting the historical v13 index',()=>{
    const plan=read('docs/qualification/plans/step-interchange-5.json')
    const evidence=read('docs/qualification/step-interchange-5-evidence-v12.json')
    expect(plan.lifecycle.status).toBe('rejected')
    expect(plan.unresolvedRows.length).toBeGreaterThan(0)
    expect(plan.scope.excluded.join(' ')).toMatch(/SEAM_CURVE/)
    expect(plan.scope.excluded.join(' ')).toMatch(/Pole/)
    expect(evidence.state).toBe('rejected')
    expect(evidence.maturity).toBe('AnalyticComplete')
    expect(evidence.unresolvedRows.length).toBeGreaterThan(0)
    expect(evidence.runs).toEqual([])
  })
})
