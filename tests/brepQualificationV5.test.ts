import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'
import {BREP_CAPABILITY_MATRIX,assertBrepCapabilityAllowsTopology} from '../src/services/geometry/brepCapability'

type Row={id:string;plan:string;evidence:string;maturity:string;releaseState:'shipped'|'candidate';dependencies:string[]}
const root=resolve(import.meta.dirname,'..')
const read=(path:string)=>JSON.parse(readFileSync(resolve(root,path),'utf8')) as Record<string,unknown>
const frozenV4=[
 'docs/qualification/plans/brep-capability-qualification-plan-v4.schema.json',
 'docs/qualification/brep-capability-evidence-v4.schema.json',
 'docs/qualification/plans/g8-full-matrix-index-v4.json',
 'docs/qualification/brep-full-closed-matrix-v4.json',
 'docs/qualification/brep-capability-registry-release-full-v4.json',
 'docs/qualification/plans/nurbs-boolean-bezier-le3-6.json',
 'docs/qualification/nurbs-boolean-bezier-le3-6-evidence-v4.json',
] as const
const digest=(paths:readonly string[])=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}

describe('V5 bounded general NURBS qualification',()=>{
 it('preserves every frozen V4 contract byte and leaves /6 unavailable',()=>{
  expect(digest(frozenV4)).toBe('9ed2b1af05c5b3fdd155f29061c31ec5fdb4cd8271e4bd98b9e94bd2a691a0cf')
  const v4=read('docs/qualification/plans/g8-full-matrix-index-v4.json')
  const v5=read('docs/qualification/plans/g8-full-matrix-index-v5.json')
  expect((v5.capabilities as Row[]).slice(0,-1)).toEqual(v4.capabilities)
  expect((v5.capabilities as Row[]).find(row=>row.id==='nurbs-boolean-bezier-le3/6')).toMatchObject({
   maturity:'Unavailable',releaseState:'candidate',
  })
 })

 it('ships only the exact /7 finite cell with Qualified dependency closure',()=>{
  const index=read('docs/qualification/plans/g8-full-matrix-index-v5.json')
  const matrix=read('docs/qualification/brep-full-closed-matrix-v5.json')
  const release=read('docs/qualification/brep-capability-registry-release-full-v5.json')
  const rows=index.capabilities as Row[]
  const byId=new Map(rows.map(row=>[row.id,row]))
  const row=byId.get('nurbs-boolean-bezier-le3/7')
  expect(row).toMatchObject({maturity:'Qualified',releaseState:'shipped'})
  expect(row?.dependencies).not.toContain('nurbs-boolean-bezier-le3/6')
  for(const dependency of row?.dependencies??[]) expect(byId.get(dependency),dependency).toMatchObject({
   maturity:'Qualified',releaseState:'shipped',
  })
  expect(release.capabilities).toContain(row?.id)
  expect(matrix.admittedOps).toContain(row?.id)
  expect(release.excludedPendingQualification).toContain('nurbs-boolean-bezier-le3/6')
  expect(matrix.pendingQualification).toContain('nurbs-boolean-bezier-le3/6')
  expect(release.unresolvedInShippedMatrix).toEqual([])
  expect(matrix.unresolvedInShippedMatrix).toEqual([])
 })

 it('binds the complete implementation and states all explicit refusals',()=>{
  const plan=read('docs/qualification/plans/nurbs-boolean-bezier-le3-7.json')
  const evidence=read('docs/qualification/nurbs-boolean-bezier-le3-7-evidence-v5.json')
  expect(plan).toMatchObject({
   schemaVersion:5,capability:'nurbs-boolean-bezier-le3/7',
   successorOf:'nurbs-boolean-bezier-le3/6',
   lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
   evidence:{state:'qualified'},unresolvedRows:[],
  })
  expect((plan.bindings as {implementation:string[]}).implementation).toEqual(expect.arrayContaining([
   'crates/brep-core/src/lib.rs','crates/brep-core/src/nurbs_ss_g6.rs',
   'crates/brep-core/src/uv_arrangement.rs','crates/brep-core/src/trim_sew.rs',
   'crates/brep-core/src/solid_audit.rs','crates/brep-core/src/operations.rs',
   'crates/brep-core/src/transactions.rs','crates/brep-topology/src/lib.rs',
   'crates/geometry-bridge/src/lib.rs','crates/geometry-bridge/src/tests.rs',
   'src/services/geometry/kernel.ts','src/services/geometry/brep.ts',
   'src/services/geometry/brepCapability.ts',
  ]))
  const exclusions=(plan.scope as {excluded:string[]}).excluded.join('\n')
  for(const refusal of [
   'Tangency','coincidence','ambiguous joins','Periodic','poles','Degree above 3',
   'unsupported rational boundary parameterizations','Union','reversed difference','Resource overflow',
   'no-fallback',
  ]) expect(exclusions).toContain(refusal)
  expect(evidence).toMatchObject({
   schemaVersion:5,capability:'nurbs-boolean-bezier-le3/7',
   maturity:'Qualified',state:'qualified',unresolvedRows:[],
   attestation:{fabricatedRuns:false},
  })
  expect((evidence.runs as unknown[]).length).toBeGreaterThan(0)
  expect((evidence.oracles as unknown[]).length).toBeGreaterThan(0)
 })

 it('validates plan and evidence under strict draft-2020 AJV',()=>{
  const ajv=new Ajv2020({strict:true,allErrors:true})
  const planSchema=read('docs/qualification/plans/brep-capability-qualification-plan-v5.schema.json')
  const evidenceSchema=read('docs/qualification/brep-capability-evidence-v5.schema.json')
  expect(ajv.compile(planSchema)(read('docs/qualification/plans/nurbs-boolean-bezier-le3-7.json'))).toBe(true)
  expect(ajv.compile(evidenceSchema)(read('docs/qualification/nurbs-boolean-bezier-le3-7-evidence-v5.json'))).toBe(true)
 })

 it('exposes /7 through the runtime registry while /6 stays fail-closed',()=>{
  expect(BREP_CAPABILITY_MATRIX.find(row=>row.id==='nurbs-boolean-bezier-le3/7')).toMatchObject({
   maturity:'Qualified',permitsTopologyChange:true,
  })
  expect(()=>assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/7')).not.toThrow()
  expect(()=>assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/6'))
   .toThrow('is Unavailable; refuse topology change')
 })
})
