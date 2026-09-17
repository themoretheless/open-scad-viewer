import {describe,expect,it} from 'vitest'
import {
 createBrepBox,
 createCanonicalMultispanGraphSolid,
 generalNurbsBoolean,
 generalNurbsBooleanModel,
} from '../src/services/geometry/brep'

describe('general multispan NURBS Boolean product',()=>{
 it('authors all exact-region operations in the 2x1, 1x2, and 2x2 cell',()=>{
  for(const [spansU,spansV] of [[2,1],[1,2],[2,2]] as const){
   const graph=createCanonicalMultispanGraphSolid(spansU,spansV)
   const cutter=createBrepBox([0.25,-1,-1],[0.75,2,3])
   const snapshots=[structuredClone(graph),structuredClone(cutter)]
   for(const [operation,components,faces] of [
    ['intersection',1,6],
    ['difference',2,12],
    ['union',3,18],
   ] as const){
    const result=generalNurbsBoolean(graph,cutter,operation)
    expect(result.certificate).toMatchObject({
     capability:'nurbs-boolean-bezier-le3/8',
     authority:'author-general-nurbs-boolean',
     status:'Complete',
     operation,
     operandOrder:'source-tool',
     exactRegionMembership:true,
     branchGraph:{components:2,complete:true},
     uv:{branches:2,complete:true,holeCells:0},
     sew:{complete:true,displacementBudgetOk:true},
     audit:{ok:true,bodyCount:components,shellCount:components},
     resultComponents:components,
     resultFaces:faces,
     noFallback:true,
    })
    expect(result.certificate.exactCurvePcurveCount).toBeGreaterThanOrEqual(4)
    expect(result.certificate.naming).toMatchObject({
     split:1,
     operationStable:true,
    })
    expect(generalNurbsBooleanModel(graph,cutter,operation)).toEqual(result.model)
   }
   const reversed=generalNurbsBoolean(cutter,graph,'difference')
   expect(reversed.certificate).toMatchObject({
    capability:'nurbs-boolean-bezier-le3/8',
    operation:'difference',
    operandOrder:'tool-source',
    exactRegionMembership:true,
    partitionCells:4,
    resultComponents:4,
    resultFaces:24,
    noFallback:true,
   })
   expect(graph).toEqual(snapshots[0])
   expect(cutter).toEqual(snapshots[1])
  }
 })

 it('atomically refuses excluded operations, contacts, seams, poles, degree and rational boundaries',()=>{
  const graph=createCanonicalMultispanGraphSolid(2,2)
  const cutter=createBrepBox([0.25,-1,-1],[0.75,2,3])
  const expectAtomicRefusal=(a:typeof graph,b:typeof cutter,operation:'union'|'intersection'|'difference')=>{
   const before=[structuredClone(a),structuredClone(b)]
   expect(()=>generalNurbsBoolean(a,b,operation)).toThrow()
   expect(a).toEqual(before[0])
   expect(b).toEqual(before[1])
  }
  const snapshots=[structuredClone(graph),structuredClone(cutter)]
  expect(()=>generalNurbsBoolean(cutter,graph,'union')).toThrow(/operand order/i)
  expect(graph).toEqual(snapshots[0])
  expect(cutter).toEqual(snapshots[1])
  const tangent=createBrepBox([1,-1,-1],[2,2,3])
  const coincident=createBrepBox([0,-1,-1],[1,2,3])
  for(const contact of [tangent,coincident]){
   expectAtomicRefusal(graph,contact,'intersection')
   expectAtomicRefusal(graph,contact,'union')
   expectAtomicRefusal(contact,graph,'difference')
  }
  const periodic=structuredClone(graph)
  periodic.faces[1]!.surface.periodicU=true
  expectAtomicRefusal(periodic,cutter,'intersection')
  const highDegree=structuredClone(graph)
  highDegree.faces[1]!.surface.degreeU=4
  expectAtomicRefusal(highDegree,cutter,'intersection')
  const pole=structuredClone(graph)
  pole.faces[1]!.surface.controlPoints[0]![0]=[0,0,0]
  pole.faces[1]!.surface.controlPoints[0]![1]=[0,0,0]
  expectAtomicRefusal(pole,cutter,'intersection')
  const rationalBoundary=structuredClone(cutter)
  rationalBoundary.faces[0]!.surface.weights[0]![0]=2
  expectAtomicRefusal(graph,rationalBoundary,'intersection')
 })
})
