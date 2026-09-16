import {describe,expect,it} from 'vitest'
import {
  authorizedHealNurbsBrep,
  createBrepBox,
  type AuthorizedHealOperation,
} from '../src/services/geometry/brep'
import {callGeometryRust} from '../src/services/geometry/kernel'

describe('authorized heal v2 WASM product seam',()=>{
 it('executes a nonzero native endpoint snap without mutating its source',()=>{
  const source=createBrepBox([0,0,0],[2,2,2])
  const before=JSON.stringify(source)
  const vertex=source.edges[0].vertices[0]
  const incident=source.edges.filter(edge=>edge.vertices.includes(vertex)).length
  const to=[...source.vertices[vertex].point] as [number,number,number]
  to[1]+=source.toleranceMm/(incident+1)*0.25
  const result=authorizedHealNurbsBrep(source,{kind:'endpointSnap',vertex,to})
  expect(JSON.stringify(source)).toBe(before)
  expect(result.certificate).toMatchObject({
   capability:'authorized-heal-gap-le1/2',
   status:'Complete',
   namingComplete:true,
   sew:{complete:true,displacementBudgetOk:true},
   audit:{ok:true},
  })
  expect(result.certificate.displacementLedger[0].actualMm).toBeGreaterThan(0)
  expect(result.certificate.cumulativeDisplacementMm).toBeLessThanOrEqual(source.toleranceMm)
  expect(result.model.vertices[vertex].point).toEqual(to)
 })

 it('refuses caller evidence and out-of-cell displacement',()=>{
  const source=createBrepBox([0,0,0],[2,2,2])
  const vertex=source.edges[0].vertices[0]
  const to=[...source.vertices[vertex].point] as [number,number,number]
  to[1]+=source.toleranceMm*2
  expect(()=>authorizedHealNurbsBrep(source,{kind:'endpointSnap',vertex,to}))
   .toThrow(/budget/i)
  const operation={
   kind:'endpointSnap',vertex,to:source.vertices[vertex].point,evidence:{complete:true},
  } as unknown as AuthorizedHealOperation
  expect(()=>callGeometryRust('brep_nurbs_authorized_heal_v2',{model:source,operation}))
   .toThrow(/unauthorized fields/i)
 })
})
