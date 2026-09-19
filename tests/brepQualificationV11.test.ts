import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'

const root=resolve(import.meta.dirname,'..')
const bytes=(path:string)=>readFileSync(resolve(root,path))
const read=(path:string)=>JSON.parse(bytes(path).toString('utf8')) as Record<string,unknown>

describe('V11 direct interchange qualification',()=>{
  it('is an append-only V10 successor with strict schemas',()=>{
    const v10=read('docs/qualification/plans/g8-full-matrix-index-v10.json')
    const v11=read('docs/qualification/plans/g8-full-matrix-index-v11.json')
    expect(v11.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v10.json')
    expect((v11.capabilities as unknown[]).slice(0,(v10.capabilities as unknown[]).length)).toEqual(v10.capabilities)
    const ajv=new Ajv2020({strict:true,allErrors:true})
    const plan=ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v11.schema.json'))
    const evidence=ajv.compile(read('docs/qualification/brep-capability-evidence-v11.schema.json'))
    for(const slug of ['iges-interchange-2','step-interchange-4']){
      expect(plan(read(`docs/qualification/plans/${slug}.json`)),JSON.stringify(plan.errors)).toBe(true)
      expect(evidence(read(`docs/qualification/${slug}-evidence-v11.json`)),JSON.stringify(evidence.errors)).toBe(true)
    }
  })

  it('keeps the G0/G1 successor explicitly no-claim',()=>{
    const status=read('docs/qualification/g0-v15-g1-v32-refreeze-status-v1.json')
    const plan=read('docs/qualification/semantic-manifold-g1-plan-v32.json')
    expect(status).toMatchObject({qualificationClaim:'none',qualificationApproval:'not-approved',g0Closed:false,completedWorkUnits:0,completedCleanRuns:0,plannedWorkUnits:4740})
    expect(plan).toMatchObject({planId:'semantic-manifold-g1-plan-v32',lifecycle:{qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v32',priorResultsMayBeImported:false}})
  })
})
