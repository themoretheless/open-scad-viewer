import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'
import {
 assertBrepCapabilityAllowsTopology,
 BREP_CAPABILITY_MATRIX,
} from '../src/services/geometry/brepCapability'
import {
 auditedBentRmfSweep,
 auditedMultiSectionLoft,
 transformNurbsBrep,
} from '../src/services/geometry/brep'

type Row={id:string;plan:string;evidence:string;maturity:string;releaseState:'shipped'|'candidate'}
const root=resolve(import.meta.dirname,'..')
const read=(path:string)=>JSON.parse(readFileSync(resolve(root,path),'utf8')) as Record<string,unknown>
const sections=[
 [[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],
 [[-1.5,-1,2],[1.5,-1,2],[1.5,1,2],[-1.5,1,2]],
 [[-1,-0.75,5],[1,-0.75,5],[1,0.75,5],[-1,0.75,5]],
] as [number,number,number][][]

describe('V9 exact loft and bent RMF sweep qualification',()=>{
 it('appends two qualified successors without rewriting V8',()=>{
  const v8=read('docs/qualification/plans/g8-full-matrix-index-v8.json')
  const v9=read('docs/qualification/plans/g8-full-matrix-index-v9.json')
  const oldRows=v8.capabilities as Row[]
  const rows=v9.capabilities as Row[]
  expect(rows.slice(0,oldRows.length)).toEqual(oldRows)
  expect(rows.slice(-2).map(row=>[row.id,row.maturity,row.releaseState])).toEqual([
   ['analytic-solid-loft/2','Qualified','shipped'],
   ['exact-parallel-frame-sweep/2','Qualified','shipped'],
  ])
 })

 it('validates both V9 plans and evidence with strict AJV',()=>{
  const ajv=new Ajv2020({strict:true,allErrors:true})
  const validatePlan=ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v9.schema.json'))
  const validateEvidence=ajv.compile(read('docs/qualification/brep-capability-evidence-v9.schema.json'))
  for(const slug of ['analytic-solid-loft-2','exact-parallel-frame-sweep-2']){
   expect(validatePlan(read(`docs/qualification/plans/${slug}.json`)),JSON.stringify(validatePlan.errors)).toBe(true)
   expect(validateEvidence(read(`docs/qualification/${slug}-evidence-v9.json`)),JSON.stringify(validateEvidence.errors)).toBe(true)
  }
 })

 it('publishes exact multi-section topology and survives cyclic correspondence permutation',()=>{
  const result=auditedMultiSectionLoft(sections)
  expect(result.certificate.capability).toBe('analytic-solid-loft/2')
  expect(result.audit.ok&&result.namingComplete).toBe(true)
  expect(result.audit.selfIntersectionPairsChecked).toBeGreaterThan(0)
  expect([result.model.vertices.length,result.model.edges.length,result.model.faces.length]).toEqual([12,20,10])
  const cyclic=sections.map(section=>[...section.slice(1),section[0]])
  const permuted=auditedMultiSectionLoft(cyclic)
  expect([permuted.model.vertices.length,permuted.model.edges.length,permuted.model.faces.length]).toEqual([12,20,10])
  const placed=transformNurbsBrep(result.model,[[0,-1,0,4],[1,0,0,-2],[0,0,1,3],[0,0,0,1]])
  expect([placed.vertices.length,placed.edges.length,placed.faces.length]).toEqual([12,20,10])
 })

 it('publishes bent RMF twist/scale certificates and reversal-stable topology',()=>{
  const profile=[[-0.2,-0.2],[0.2,-0.2],[0.2,0.2],[-0.2,0.2]] as [number,number][]
  const path=[[0,0,0],[0,0,3],[0,1,6],[0,3,9]] as [number,number,number][]
  const twist=[0,0.1,0.2,0.3],scales=[1,1.1,1.2,1.25]
  const result=auditedBentRmfSweep(profile,path,twist,scales)
  expect(result.certificate.capability).toBe('exact-parallel-frame-sweep/2')
  expect(result.certificate.notes).toContain('zero_span_curvature_bernstein_hulls')
  expect(result.audit.ok&&result.namingComplete).toBe(true)
  expect([result.model.vertices.length,result.model.edges.length,result.model.faces.length]).toEqual([16,28,14])
  const reversed=auditedBentRmfSweep(profile,[...path].reverse(),[...twist].reverse(),[...scales].reverse())
  expect([reversed.model.vertices.length,reversed.model.edges.length,reversed.model.faces.length]).toEqual([16,28,14])
 })

 it('detects law/topology mutations and refuses atomically',()=>{
  const profile=[[-0.2,-0.2],[0.2,-0.2],[0.2,0.2],[-0.2,0.2]] as [number,number][]
  const path=[[0,0,0],[0,0,3],[0,1,6]] as [number,number,number][]
  const before=JSON.stringify({profile,path})
  expect(()=>auditedBentRmfSweep(profile,path,[0,0,0],[1,0,1])).toThrow(/Scale Bernstein bounds/)
  expect(()=>auditedBentRmfSweep(profile,[[0,0,0],[0,0,3],[0,0,0]],[0,0,0],[1,1,1])).toThrow(/Closed path holonomy/)
  expect(()=>auditedMultiSectionLoft([...sections.slice(0,2),sections[2].slice(0,3)])).toThrow(/correspondence/)
  expect(JSON.stringify({profile,path})).toBe(before)
 })

 it('exposes only the exact successor cells as topology-authoring',()=>{
  for(const id of ['analytic-solid-loft/2','exact-parallel-frame-sweep/2']){
   expect(BREP_CAPABILITY_MATRIX.find(row=>row.id===id)).toMatchObject({maturity:'Qualified',permitsTopologyChange:true})
   expect(()=>assertBrepCapabilityAllowsTopology(id)).not.toThrow()
  }
 })
})
