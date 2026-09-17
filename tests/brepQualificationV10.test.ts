import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import Ajv2020 from 'ajv/dist/2020.js'
import {describe,expect,it} from 'vitest'
import {
 analyzeCertifiedNurbsBrep,
 auditedBentRmfSweep,
 auditedMultiSectionLoft,
 booleanNurbsBrep,
 createBrepFrustum,
 createBrepSphere,
 createBrepTorus,
 createCanonicalMultispanGraphSolid,
 tessellateCertifiedNurbsBrep,
 transformNurbsBrep,
} from '../src/services/geometry/brep'

const root=resolve(import.meta.dirname,'..')
const read=(path:string)=>JSON.parse(readFileSync(resolve(root,path),'utf8')) as Record<string,unknown>

describe('V10 certified analysis successors',()=>{
 it('preserves V9 and strictly validates append-only V10 artifacts',()=>{
  const v9=read('docs/qualification/plans/g8-full-matrix-index-v9.json')
  const v10=read('docs/qualification/plans/g8-full-matrix-index-v10.json')
  expect((v10.capabilities as unknown[]).slice(0,(v9.capabilities as unknown[]).length)).toEqual(v9.capabilities)
  const ajv=new Ajv2020({strict:true,allErrors:true})
  const plan=ajv.compile(read('docs/qualification/plans/brep-capability-qualification-plan-v10.schema.json'))
  const evidence=ajv.compile(read('docs/qualification/brep-capability-evidence-v10.schema.json'))
  for(const slug of ['certified-brep-tessellation-2','certified-mass-properties-2']){
   expect(plan(read(`docs/qualification/plans/${slug}.json`)),JSON.stringify(plan.errors)).toBe(true)
   expect(evidence(read(`docs/qualification/${slug}-evidence-v10.json`)),JSON.stringify(evidence.errors)).toBe(true)
  }
 })

 it('certifies analytic curved tessellation with monotone adaptive budgets',()=>{
  for(const body of [createBrepSphere(3),createBrepFrustum(3,1,4),createBrepTorus(5,2)]){
   const coarse=tessellateCertifiedNurbsBrep(body,.5)
   const fine=tessellateCertifiedNurbsBrep(body,.1)
   expect(fine.capability).toBe('certified-brep-tessellation/2')
   expect(fine.surfaceToMeshDeviationMm).toBeLessThanOrEqual(.1)
   expect(fine.meshToSurfaceDeviationMm).toBe(fine.surfaceToMeshDeviationMm)
   expect(fine.resourceProof.subdivisionsPerPatch).toBeGreaterThanOrEqual(coarse.resourceProof.subdivisionsPerPatch)
   expect(fine.coverage).toMatchObject({noCracks:true,normalConsistency:true,poleDegeneracyHandled:true,periodicSeamsHandled:true})
   expect(fine.tessellation.report.closed).toBe(true)
  }
 })

 it('encloses analytic mass formulas and rigid transforms',()=>{
  const cases=[
   [createBrepSphere(3),36*Math.PI],
   [createBrepFrustum(3,1,4),52*Math.PI/3],
   [createBrepTorus(5,2),40*Math.PI*Math.PI],
  ] as const
  for(const [body,volume] of cases){
   const mass=analyzeCertifiedNurbsBrep(body)
   expect(mass.capability).toBe('certified-mass-properties/2')
   expect(mass.volumeMm3.lower).toBeLessThanOrEqual(volume)
   expect(mass.volumeMm3.upper).toBeGreaterThanOrEqual(volume)
   const placed=transformNurbsBrep(body,[[0,-1,0,7],[1,0,0,-3],[0,0,1,5],[0,0,0,1]])
   const transformed=analyzeCertifiedNurbsBrep(placed)
   expect(transformed.volumeMm3.lower).toBeLessThanOrEqual(volume)
   expect(transformed.volumeMm3.upper).toBeGreaterThanOrEqual(volume)
  }
  const cavity=booleanNurbsBrep(createBrepSphere(3),createBrepSphere(1),'difference')
  const hollow=analyzeCertifiedNurbsBrep(cavity)
  expect(hollow.composition).toEqual({componentCount:2,cavityCount:1,signedShellComposition:true})
  expect(hollow.volumeMm3.lower).toBeLessThanOrEqual(104*Math.PI/3)
  expect(hollow.volumeMm3.upper).toBeGreaterThanOrEqual(104*Math.PI/3)

  const moved=transformNurbsBrep(createBrepSphere(1),[[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
  const multiple=booleanNurbsBrep(createBrepSphere(2),moved,'union')
  const composed=analyzeCertifiedNurbsBrep(multiple)
  expect(composed.composition).toMatchObject({componentCount:2,cavityCount:0})
  expect(composed.volumeMm3.lower).toBeLessThanOrEqual(12*Math.PI)
  expect(composed.volumeMm3.upper).toBeGreaterThanOrEqual(12*Math.PI)
 })

 it('typed-refuses uncertified freeform, loft and bent-sweep cells',()=>{
  expect(()=>analyzeCertifiedNurbsBrep(createCanonicalMultispanGraphSolid(2,2))).toThrow(/generic rational|freeform|refused/i)
  const sections=[
   [[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],
   [[-2,-1,2],[2,-1,2],[2,1,2],[-2,1,2]],
   [[-1,-1,5],[1,-1,5],[1,1,5],[-1,1,5]],
  ] as [number,number,number][][]
  expect(()=>analyzeCertifiedNurbsBrep(auditedMultiSectionLoft(sections).model)).toThrow(/generic rational|freeform|refused/i)
  const sweep=auditedBentRmfSweep(
   [[-.2,-.2],[.2,-.2],[.2,.2],[-.2,.2]],
   [[0,0,0],[0,0,3],[0,1,6]],
   [0,.1,.2],[1,1.1,1.2],
  )
  expect(()=>analyzeCertifiedNurbsBrep(sweep.model)).toThrow(/generic rational|freeform|refused/i)
  expect(()=>tessellateCertifiedNurbsBrep(sweep.model,.1)).toThrow(/generic rational|freeform|refused/i)
 })
})
