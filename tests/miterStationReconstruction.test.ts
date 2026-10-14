import {expect,it} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import type {NurbsBrep,SmoothStationWallCandidate} from '../src/services/geometry/brep'
import {createPeriodicBrepSectionLoft,createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {callGeometryRust} from '../src/services/geometry/kernel'
const signs=[[1,0],[1,1],[0,1],[-1,1],[-1,0],[-1,-1],[0,-1],[1,-1],[1,0]]
const section=(x:number,z:number):NurbsCurve[][]=>[[{
 degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],
 controlPoints:signs.map(([a,b])=>[x+a!,b!,z]),
 weights:signs.map((_,i)=>i%2===0?1:Math.SQRT1_2),periodic:false,
}]]
const reconstruct=(model:NurbsBrep,sections:NurbsCurve[][][],closed=false,maxWork=10000,tolerance=10)=>
 callGeometryRust<{model:NurbsBrep|null;candidate:SmoothStationWallCandidate}>('brep_miter_reconstruct_stations',{
  model,sections,sharp:[],closed,quantum:.125,tolerance,maxWork,
 })
it('owns complete source correspondence and preserves caps through the native reconstruction opcode',()=>{
 const sections=[section(0,0),section(1,5),section(0,10)]
 const source=createRationalBrepSectionLoft(sections)
 const result=reconstruct(source,sections)
 expect(result.model).not.toBeNull()
 expect(result.candidate.wallDisplacementUpper).toBeGreaterThan(0)
 for(const index of [source.faces.length-2,source.faces.length-1]){
  expect(result.model!.faces[index]).toEqual(source.faces[index])
  const face=source.faces[index]!
  for(const loop of [face.outer,...face.holes]){
   expect(result.model!.loops[loop]).toEqual(source.loops[loop])
   for(const use of source.loops[loop]!.coedges){
    expect(result.model!.edges[use.edge]).toEqual(source.edges[use.edge])
    for(const vertex of source.edges[use.edge]!.vertices)expect(result.model!.vertices[vertex]).toEqual(source.vertices[vertex])
   }
  }
 }
 const wrong=structuredClone(sections)
 for(const pole of wrong[1]![0]![0]!.controlPoints)pole[0]!+=.125
 expect(()=>reconstruct(source,wrong)).toThrow(/reproduce/)
 for(const [work,tolerance] of [[0,10],[10000,0]]){
  const refusal=reconstruct(source,sections,false,work,tolerance)
  expect(refusal.model).toBeNull()
  expect(refusal.candidate.sides).toBeNull()
 }
})
it('reconstructs periodic source stations and refuses an open source in closed mode',()=>{
 const sections=[section(0,0),section(1,5),section(0,10),section(0,0)]
 const source=createPeriodicBrepSectionLoft(sections)
 const result=reconstruct(source,sections,true)
 expect(result.model).not.toBeNull()
 expect(result.model!.faces).toHaveLength(source.faces.length)
 const open=createRationalBrepSectionLoft([section(0,0),section(1,5),section(0,10)])
 expect(()=>reconstruct(open,sections,true)).toThrow(/reproduce/)
})
