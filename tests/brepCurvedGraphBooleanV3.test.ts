import {describe,expect,it} from 'vitest'
import {
 certifiedCurvedGraphBoolean,
 booleanNurbsBrep,
 createBrepBox,
 createCanonicalBezierGraphSolid,
} from '../src/services/geometry/brep'

describe('curved graph Boolean V3 WASM seam',()=>{
 it('serializes exact U/V degree 2/3 intersection and difference certificates',()=>{
  for(const [du,dv,axis] of [[2,2,'U'],[2,3,'V'],[3,2,'U'],[3,3,'V']] as const){
   const graph=createCanonicalBezierGraphSolid(du,dv)
   const cutter=axis==='U'
    ?createBrepBox([0.5,-1,-1],[2,2,3])
    :createBrepBox([-1,0.5,-1],[2,2,3])
   for(const operation of ['intersection','difference'] as const){
    const result=certifiedCurvedGraphBoolean(graph,cutter,operation)
    expect(result.certificate).toMatchObject({
     capability:'nurbs-boolean-bezier-le3/3',
     status:'Complete',
     operation,
     axis,
     tensorCells:3,
     exactCorrespondence:true,
     sew:{complete:true,displacementBudgetOk:true},
     audit:{ok:true,bodyCount:1,shellCount:1},
     namingComplete:true,
     noFallback:true,
     separationProof:true,
    })
    expect(result.certificate.lineage.generatedIntersectionEdge).not.toBe('')
    expect(result.model.faces).toHaveLength(6)
    expect(booleanNurbsBrep(graph,cutter,operation).faces).toHaveLength(6)
   }
  }
 })

 it('refuses union atomically at the certified product route',()=>{
  const graph=createCanonicalBezierGraphSolid(3,3)
  const cutter=createBrepBox([0.5,-1,-1],[2,2,3])
  expect(()=>certifiedCurvedGraphBoolean(
   graph,cutter,'union' as unknown as 'intersection',
  )).toThrow(/union lacks/i)
 })
})
