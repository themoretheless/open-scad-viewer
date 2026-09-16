import {describe,expect,it} from 'vitest'
import {
 booleanNurbsBrep,
 certifiedContainedGraphBoolean,
 certifiedRationalGraphBoolean,
 certifiedUnequalSpanGraphBoolean,
 createBrepBox,
 createCanonicalBezierGraphSolid,
 createCanonicalRationalGraphSolid,
} from '../src/services/geometry/brep'

describe('append-only NURBS Boolean successor cells',()=>{
 it('certifies V4 unequal spans and strict containment per operation',()=>{
  const graph=createCanonicalBezierGraphSolid(3,3)
  const unequal=createBrepBox([0.5,-2,-3],[2,3,4])
  for(const operation of ['intersection','difference'] as const){
   const result=certifiedUnequalSpanGraphBoolean(graph,unequal,operation)
   expect(result.certificate.capability).toBe('nurbs-boolean-bezier-le3/4')
   expect(result.certificate.operation).toBe(operation)
   expect(result.certificate.separationProof).toBe(true)
   expect(result.certificate.noFallback).toBe(true)
   expect(result.certificate.changeSet.changes.length).toBeGreaterThan(0)
  }
  const inner=createBrepBox([0.2,0.2,0.2],[0.8,0.8,0.8])
  for(const operation of ['union','intersection','difference'] as const){
   const result=certifiedContainedGraphBoolean(graph,inner,operation)
   expect(result.certificate).toMatchObject({
    capability:'nurbs-boolean-bezier-le3/4',
    status:'Complete',
    operation,
    relation:'graph-strictly-contains-affine-cutter',
    separationProof:true,
    namingComplete:true,
    noFallback:true,
   })
   expect(result.certificate.strictUvMargin).toBeGreaterThan(0)
   expect(result.certificate.floorClearance).toBeGreaterThan(0)
   expect(result.certificate.roofClearance).toBeGreaterThan(0)
   expect(result.certificate.cavityProof).toBe(true)
  }
  expect(()=>certifiedContainedGraphBoolean(
   graph,createBrepBox([0,0.2,0.2],[0.8,0.8,0.8]),'difference',
  )).toThrow(/strict containment/i)
 })

 it('certifies V5 bounded rational roots and keeps excluded contacts refused',()=>{
  const graph=createCanonicalRationalGraphSolid(3,3)
  const cutter=createBrepBox([0.5,-1,-1],[2,2,3])
  for(const operation of ['intersection','difference'] as const){
   const result=certifiedRationalGraphBoolean(graph,cutter,operation)
   expect(result.certificate).toMatchObject({
    capability:'nurbs-boolean-bezier-le3/5',
    status:'Complete',
    operation,
    homogeneousRootProof:true,
    exactCorrespondence:true,
    noFallback:true,
   })
   expect(result.certificate.denominatorLowerBound).toBeGreaterThanOrEqual(0.25)
   expect(result.certificate.weightConditionNumber).toBeLessThanOrEqual(8)
   expect(result.certificate.resourceBound).toBeLessThanOrEqual(16)
   expect(booleanNurbsBrep(graph,cutter,operation).faces).toHaveLength(6)
  }
  expect(()=>certifiedRationalGraphBoolean(
   graph,createBrepBox([0,-1,-1],[2,2,3]),'intersection',
  )).toThrow(/strict-interior|cutting face/i)
  expect(()=>certifiedRationalGraphBoolean(
   graph,cutter,'union' as unknown as 'intersection',
  )).toThrow(/union lacks/i)
 })
})
