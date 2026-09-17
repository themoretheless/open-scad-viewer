import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'
import {
 assertBrepCapabilityAllowsTopology,
 BREP_CAPABILITY_MATRIX,
} from '../src/services/geometry/brepCapability'
import {
 createBrepBox,
 exactConvexChamfer,
 exactConvexPrismFillet,
 exactVariableRadiusFillet,
} from '../src/services/geometry/brep'

type Row={id:string;plan:string;evidence:string;maturity:string;releaseState:'shipped'|'candidate'}
const root=resolve(import.meta.dirname,'..')
const read=(path:string)=>JSON.parse(readFileSync(resolve(root,path),'utf8')) as Record<string,unknown>

describe('V7 exact finite blend qualification',()=>{
 it('appends two qualified and two unavailable rows without rewriting V6',()=>{
  const v6=read('docs/qualification/plans/g8-full-matrix-index-v6.json')
  const v7=read('docs/qualification/plans/g8-full-matrix-index-v7.json')
  const rows=v7.capabilities as Row[]
  expect(rows.slice(0,(v6.capabilities as Row[]).length)).toEqual(v6.capabilities)
  expect(rows.slice(-4).map(row=>[row.id,row.maturity,row.releaseState])).toEqual([
   ['exact-convex-prism-edge-fillet/1','Qualified','shipped'],
   ['exact-convex-straight-edge-chamfer/1','Qualified','shipped'],
   ['exact-valence3-corner-blend/1','Unavailable','candidate'],
   ['exact-variable-radius-fillet/1','Unavailable','candidate'],
  ])
 })

 it('validates every V7 plan and evidence with strict AJV',()=>{
  const ajv=new Ajv2020({strict:true,allErrors:true})
  const validatePlan=ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v7.schema.json'))
  const validateEvidence=ajv.compile(read('docs/qualification/brep-capability-evidence-v7.schema.json'))
  const index=read('docs/qualification/plans/g8-full-matrix-index-v7.json')
  for(const row of (index.capabilities as Row[]).filter(row=>row.plan.includes('exact-convex')||row.plan.includes('exact-valence3')||row.plan.includes('exact-variable'))){
   expect(validatePlan(read(row.plan)),JSON.stringify(validatePlan.errors)).toBe(true)
   expect(validateEvidence(read(row.evidence)),JSON.stringify(validateEvidence.errors)).toBe(true)
  }
 })

 it('publishes exact bridge/WASM certificates and refuses unsupported laws',()=>{
  const box=createBrepBox([0,0,0],[10,8,6])
  const longitudinal=box.edges.flatMap((edge,index)=>{
   const [a,b]=edge.vertices.map(vertex=>box.vertices[vertex].point)
   return Math.abs(a[0]-b[0])<1e-9&&Math.abs(a[1]-b[1])<1e-9?[index]:[]
  })
  const fillet=exactConvexPrismFillet(box,[longitudinal[0],longitudinal[2]],0.4)
  expect(fillet.certificate.capability).toBe('exact-convex-prism-edge-fillet/1')
  expect(fillet.audit.ok&&fillet.namingComplete).toBe(true)
  const connected=box.edges.findIndex((edge,index)=>index>0&&
   edge.vertices.some(vertex=>box.edges[0].vertices.includes(vertex)))
  const chamfer=exactConvexChamfer(box,[0,connected],0.5)
  expect(chamfer.certificate.capability).toBe('exact-convex-straight-edge-chamfer/1')
  expect(chamfer.audit.ok&&chamfer.namingComplete).toBe(true)
  expect(()=>exactVariableRadiusFillet(box,[0],[[0.4,0.6]])).toThrow(/Variable-radius fillet is unavailable/)
 })

 it('exposes only proven V7 cells as topology-authoring',()=>{
  for(const id of ['exact-convex-prism-edge-fillet/1','exact-convex-straight-edge-chamfer/1']){
   expect(BREP_CAPABILITY_MATRIX.find(row=>row.id===id)).toMatchObject({maturity:'Qualified',permitsTopologyChange:true})
   expect(()=>assertBrepCapabilityAllowsTopology(id)).not.toThrow()
  }
  for(const id of ['exact-valence3-corner-blend/1','exact-variable-radius-fillet/1']){
   expect(BREP_CAPABILITY_MATRIX.find(row=>row.id===id)).toMatchObject({maturity:'Unavailable',permitsTopologyChange:false})
   expect(()=>assertBrepCapabilityAllowsTopology(id)).toThrow()
  }
 })
})
