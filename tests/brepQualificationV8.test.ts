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
 createBrepCylinder,
 exactAnalyticShell,
} from '../src/services/geometry/brep'

type Row={id:string;plan:string;evidence:string;maturity:string;releaseState:'shipped'|'candidate'}
const root=resolve(import.meta.dirname,'..')
const read=(path:string)=>JSON.parse(readFileSync(resolve(root,path),'utf8')) as Record<string,unknown>

describe('V8 exact analytic shell qualification',()=>{
 it('appends one qualified successor without rewriting V7',()=>{
  const v7=read('docs/qualification/plans/g8-full-matrix-index-v7.json')
  const v8=read('docs/qualification/plans/g8-full-matrix-index-v8.json')
  const oldRows=v7.capabilities as Row[]
  const rows=v8.capabilities as Row[]
  expect(rows.slice(0,oldRows.length)).toEqual(oldRows)
  expect(rows.at(-1)).toMatchObject({
   id:'analytic-shell/2',maturity:'Qualified',releaseState:'shipped',
  })
 })

 it('validates the V8 plan and evidence with strict AJV',()=>{
  const ajv=new Ajv2020({strict:true,allErrors:true})
  const validatePlan=ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v8.schema.json'))
  const validateEvidence=ajv.compile(read('docs/qualification/brep-capability-evidence-v8.schema.json'))
  expect(validatePlan(read('docs/qualification/plans/analytic-shell-2.json')),JSON.stringify(validatePlan.errors)).toBe(true)
  expect(validateEvidence(read('docs/qualification/analytic-shell-2-evidence-v8.json')),JSON.stringify(validateEvidence.errors)).toBe(true)
 })

 it('serializes audited shell certificates and exact refusal boundaries',()=>{
  const box=createBrepBox([0,0,0],[10,8,6])
  const before=JSON.stringify(box)
  const shell=exactAnalyticShell(box,[0,1],0.5,'inward')
  expect(shell.certificate.capability).toBe('analytic-shell/2')
  expect(shell.audit.ok&&shell.namingComplete).toBe(true)
  expect(shell.audit.selfIntersectionPairsChecked).toBeGreaterThan(0)
  expect(shell.changeSet.changes.length).toBeGreaterThan(0)
  expect(JSON.stringify(box)).toBe(before)
  expect(()=>exactAnalyticShell(box,[0,2],0.5,'inward')).toThrow(/Adjacent planar openings/)

  const cylinder=createBrepCylinder(4,6)
  const caps=cylinder.faces.flatMap((face,index)=>
   face.surface.degreeU===1&&face.surface.degreeV===1?[index]:[])
  expect(exactAnalyticShell(cylinder,caps,0.5,'outward').model.faces).toHaveLength(10)
  expect(()=>exactAnalyticShell(cylinder,caps.slice(0,1),0.5,'inward')).toThrow(/Single-cap cylinder/)
 })

 it('exposes only the qualified successor as topology-authoring',()=>{
  expect(BREP_CAPABILITY_MATRIX.find(row=>row.id==='analytic-shell/2')).toMatchObject({
   maturity:'Qualified',permitsTopologyChange:true,
  })
  expect(()=>assertBrepCapabilityAllowsTopology('analytic-shell/2')).not.toThrow()
  expect(BREP_CAPABILITY_MATRIX.find(row=>row.id==='analytic-shell/1')).toMatchObject({
   maturity:'ResearchOnly',permitsTopologyChange:false,
  })
 })
})
