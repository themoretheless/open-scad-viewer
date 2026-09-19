import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'
import {BREP_CAPABILITY_MATRIX} from '../src/services/geometry/brepCapability'

const root=resolve(import.meta.dirname,'..')
const bytes=(path:string)=>readFileSync(resolve(root,path))
const read=(path:string)=>JSON.parse(bytes(path).toString('utf8')) as Record<string,unknown>
const slugs=['close-topology-1','tolerant-complex-heal-1','close-topology-step-1','close-topology-iges-1']

describe('V12 close-topology qualification',()=>{
  it('is an append-only V11 successor with schema-valid finite-cell evidence',()=>{
    const v11=read('docs/qualification/plans/g8-full-matrix-index-v11.json')
    const v12=read('docs/qualification/plans/g8-full-matrix-index-v12.json')
    expect(v12.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v11.json')
    expect((v12.capabilities as unknown[]).slice(0,(v11.capabilities as unknown[]).length)).toEqual(v11.capabilities)
    const ajv=new Ajv2020({strict:true,allErrors:true})
    const plan=ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v12.schema.json'))
    const evidence=ajv.compile(read('docs/qualification/brep-capability-evidence-v12.schema.json'))
    for(const slug of slugs){
      expect(plan(read(`docs/qualification/plans/${slug}.json`)),JSON.stringify(plan.errors)).toBe(true)
      expect(evidence(read(`docs/qualification/${slug}-evidence-v12.json`)),JSON.stringify(evidence.errors)).toBe(true)
    }
  })

  it('publishes only the four bounded capabilities and explicit refusals',()=>{
    for(const id of ['close-topology/1','tolerant-complex-heal/1','close-topology-step/1','close-topology-iges/1'])
      expect(BREP_CAPABILITY_MATRIX.find(row=>row.id===id)?.maturity).toBe('Qualified')
    const matrix=read('docs/qualification/brep-full-closed-matrix-v12.json')
    expect(matrix.explicitRefuse).toEqual(expect.arrayContaining([
      'nonmanifold-unbounded-branching','nonmanifold-singular-carrier',
      'mixed-dimensional-solid-audit-entry','heal-tolerance-growth','heal-large-gap',
    ]))
  })

  it('keeps the next G0/G1 freeze no-claim',()=>{
    const status=read('docs/qualification/g0-v16-g1-v33-refreeze-status-v1.json')
    const plan=read('docs/qualification/semantic-manifold-g1-plan-v33.json')
    expect(status).toMatchObject({qualificationClaim:'none',qualificationApproval:'not-approved',g0Closed:false,completedWorkUnits:0,completedCleanRuns:0,plannedWorkUnits:4740})
    expect(plan).toMatchObject({planId:'semantic-manifold-g1-plan-v33',lifecycle:{qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v33',priorResultsMayBeImported:false}})
  })
})
