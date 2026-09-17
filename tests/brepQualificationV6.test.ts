import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'
import {BREP_CAPABILITY_MATRIX,assertBrepCapabilityAllowsTopology} from '../src/services/geometry/brepCapability'

type Row={id:string;plan:string;evidence:string;maturity:string;releaseState:'shipped'|'candidate';dependencies:string[]}
const root=resolve(import.meta.dirname,'..')
const read=(path:string)=>JSON.parse(readFileSync(resolve(root,path),'utf8')) as Record<string,unknown>
const frozenV5=[
 'docs/qualification/plans/brep-capability-qualification-plan-v5.schema.json',
 'docs/qualification/brep-capability-evidence-v5.schema.json',
 'docs/qualification/plans/g8-full-matrix-index-v5.json',
 'docs/qualification/brep-full-closed-matrix-v5.json',
 'docs/qualification/brep-capability-registry-release-full-v5.json',
 'docs/qualification/plans/nurbs-boolean-bezier-le3-7.json',
 'docs/qualification/nurbs-boolean-bezier-le3-7-evidence-v5.json',
] as const
const digest=(paths:readonly string[])=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}

describe('V6 exact-region NURBS operation qualification',()=>{
 it('preserves every frozen V5 byte and appends only /8',()=>{
  expect(digest(frozenV5)).toBe('0b78690a3746037f619b44bf9b0ed9d3bb766fa4cb24c48ba4c461a4acc43cd2')
  const v5=read('docs/qualification/plans/g8-full-matrix-index-v5.json')
  const v6=read('docs/qualification/plans/g8-full-matrix-index-v6.json')
  expect((v6.capabilities as Row[]).slice(0,-1)).toEqual(v5.capabilities)
  expect((v6.capabilities as Row[]).at(-1)).toMatchObject({
   id:'nurbs-boolean-bezier-le3/8',maturity:'Qualified',releaseState:'shipped',
  })
 })

 it('qualifies union and reversed difference while refusing contact ownership',()=>{
  const plan=read('docs/qualification/plans/nurbs-boolean-bezier-le3-8.json')
  const evidence=read('docs/qualification/nurbs-boolean-bezier-le3-8-evidence-v6.json')
  expect(plan).toMatchObject({
   schemaVersion:6,capability:'nurbs-boolean-bezier-le3/8',
   successorOf:'nurbs-boolean-bezier-le3/7',
   lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
   evidence:{state:'qualified'},unresolvedRows:[],
  })
  const rows=plan.matrix as {id:string;expected:string}[]
  expect(rows.find(row=>row.id==='NB8-UNION-EXACT-REGION-PARTITION')?.expected).toBe('Complete')
  expect(rows.find(row=>row.id==='NB8-REVERSED-DIFFERENCE-EXACT-REGION-PARTITION')?.expected).toBe('Complete')
  expect(rows.find(row=>row.id.includes('TANGENCY'))?.expected).toBe('typed-refuse')
  expect(rows.find(row=>row.id.includes('COINCIDENCE'))?.expected).toBe('typed-refuse')
  expect(evidence).toMatchObject({
   schemaVersion:6,capability:'nurbs-boolean-bezier-le3/8',
   maturity:'Qualified',state:'qualified',unresolvedRows:[],
   attestation:{fabricatedRuns:false},
  })
 })

 it('validates strict schemas and exposes /8 through the runtime registry',()=>{
  const ajv=new Ajv2020({strict:true,allErrors:true})
  expect(ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v6.schema.json'))(
   read('docs/qualification/plans/nurbs-boolean-bezier-le3-8.json'),
  )).toBe(true)
  expect(ajv.compile(read('docs/qualification/brep-capability-evidence-v6.schema.json'))(
   read('docs/qualification/nurbs-boolean-bezier-le3-8-evidence-v6.json'),
  )).toBe(true)
  expect(BREP_CAPABILITY_MATRIX.find(row=>row.id==='nurbs-boolean-bezier-le3/8')).toMatchObject({
   maturity:'Qualified',permitsTopologyChange:true,
  })
  expect(()=>assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/8')).not.toThrow()
 })
})
