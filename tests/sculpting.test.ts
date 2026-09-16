import {describe,it,expect} from 'vitest'
import {extrudePolygonProfile,brushPolygonMesh,inspectPolygonMesh} from '../src/services/geometry/polygon'
import {extrudeSubdivision,brushSubdivision,tessellateSubdivision} from '../src/services/geometry/subdivision'
import {evaluateSdf,sculptSdfSphere,tessellateSdf,type SdfField} from '../src/services/geometry/sdf'
import {brushNurbsCurve,brushNurbsSurface,extrudeNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {brushDisplace} from '../src/services/meshEditing'
import type {GeometryBrush} from '../src/services/geometryEditing'

const square=[[-1,-1],[1,-1],[1,1],[-1,1]]
const cube=()=>extrudePolygonProfile({outer:square,holes:[]},[0,0,2])
const circle:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1]}
const points=(positions:number[])=>{const out:number[][]=[];for(let i=0;i<positions.length;i+=3)out.push(positions.slice(i,i+3));return out}
const invalidBrushes:GeometryBrush[]=[
 {center:[0,0,0],radius:0,displacement:[0,0,1]},
 {center:[0,0,0],radius:-1,displacement:[0,0,1]},
 {center:[0,0,0],radius:1e7,displacement:[0,0,1]},
 {center:[0,0,0],radius:1,displacement:[1e7,0,0]},
]

describe('sculpting through WASM',()=>{
 it('polygon brush moves only vertices inside the radius with smooth falloff',()=>{
  const mesh=cube()
  const out=brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,1]})
  expect(out.indices).toEqual(mesh.indices)
  expect(out.report.closed).toBe(true);expect(out.report.degenerateTriangles).toBe(0)
  expect(out.report.signedVolumeMm3).toBeGreaterThan(mesh.report.signedVolumeMm3)
  const before=points(mesh.positions),after=points(out.positions)
  const moved=before.map((p,i)=>[p,after[i]] as const).filter(([a,b])=>a.some((v,k)=>v!==b[k]))
  expect(moved.length).toBeGreaterThanOrEqual(1);expect(moved.length).toBeLessThan(before.length)
  for(const [a,b] of moved){expect(b[0]).toBe(a[0]);expect(b[1]).toBe(a[1]);expect(b[2]-a[2]).toBeGreaterThan(0);expect(b[2]-a[2]).toBeLessThanOrEqual(1)}
  const half=brushPolygonMesh(mesh,{center:[1,1,2.5],radius:1,displacement:[0,0,4]})
  const corner=points(half.positions).find(p=>p[0]===1&&p[1]===1&&p[2]>2)!
  expect(corner[2]).toBeCloseTo(4,12)
 })
 it('polygon brush carves with negative displacement and is idempotent outside the radius',()=>{
  const mesh=cube()
  const carved=brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,-0.5]})
  expect(carved.report.closed).toBe(true);expect(carved.report.signedVolumeMm3).toBeLessThan(mesh.report.signedVolumeMm3)
  const miss=brushPolygonMesh(mesh,{center:[0.5,0.5,2],radius:0.1,displacement:[0,0,-0.5]})
  expect(miss.positions).toEqual(mesh.positions)
  expect(inspectPolygonMesh(carved).closed).toBe(true)
 })
 it('brushDisplace validates inputs before calling the kernel',()=>{
  const mesh=cube()
  expect(brushDisplace(mesh,[1,1,2],0.5,[0,0,1]).positions).toEqual(brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,1]}).positions)
  expect(()=>brushDisplace(mesh,[1,1,2],0,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[1,1,2],-1,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[1,1,2],Number.NaN,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[Number.POSITIVE_INFINITY,1,2],1,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[1,1,2],1,[0,Number.NaN,1])).toThrow('Invalid brush.')
 })
 it('kernel rejects invalid brushes and degenerate results for every representation',()=>{
  const mesh=cube(),cage=extrudeSubdivision(square.map(p=>[...p,0]),[0,0,2]),surface=extrudeNurbsCurve(circle,[0,0,3])
  for(const brush of invalidBrushes){
   expect(()=>brushPolygonMesh(mesh,brush)).toThrow()
   expect(()=>brushSubdivision(cage,brush)).toThrow()
   expect(()=>brushNurbsCurve(circle,brush)).toThrow()
   expect(()=>brushNurbsSurface(surface,brush)).toThrow()
  }
  expect(()=>brushPolygonMesh({positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},{center:[0,0,0],radius:3,displacement:[0,0,0]})).not.toThrow()
  expect(()=>brushPolygonMesh({positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},{center:[0,0,0],radius:0.1,displacement:[1,0,0]})).toThrow()
 })
 it('subdivision brush edits the cage and keeps it subdividable',()=>{
  const cage=extrudeSubdivision(square.map(p=>[...p,0]),[0,0,2])
  const out=brushSubdivision(cage,{center:[1,1,2],radius:0.5,displacement:[0,0,1]})
  expect(out.faces).toEqual(cage.faces)
  const moved=cage.vertices.map((p,i)=>[p,out.vertices[i]] as const).filter(([a,b])=>a.some((v,k)=>v!==b[k]))
  expect(moved).toHaveLength(1);expect(moved[0][0]).toEqual([1,1,2]);expect(moved[0][1]).toEqual([1,1,3])
  const tess=tessellateSubdivision(out,2)
  expect(tess.report.closed).toBe(true)
  expect(tess.report.signedVolumeMm3).toBeGreaterThan(tessellateSubdivision(cage,2).report.signedVolumeMm3)
 })
 it('nurbs brush edits control points and preserves knots, weights and untouched ends',()=>{
  const curve=brushNurbsCurve(circle,{center:[1,1,0],radius:0.5,displacement:[0,0,2]})
  expect(curve.knots).toEqual(circle.knots);expect(curve.weights).toEqual(circle.weights);expect(curve.degree).toBe(2)
  expect(curve.controlPoints[0]).toEqual([1,0,0]);expect(curve.controlPoints[2]).toEqual([0,1,0])
  expect(curve.controlPoints[1][2]).toBeCloseTo(2,12)
  expect(evaluateNurbsCurve(curve,0).point).toEqual([1,0,0])
  expect(evaluateNurbsCurve(curve,0.5).point[2]).toBeGreaterThan(0)
  expect(evaluateNurbsCurve(circle,0.5).point[2]).toBe(0)
  const surface=extrudeNurbsCurve(circle,[0,0,3])
  const sculpted=brushNurbsSurface(surface,{center:[1,1,3],radius:0.5,displacement:[1,0,0]})
  expect(sculpted.knotsU).toEqual(surface.knotsU);expect(sculpted.knotsV).toEqual(surface.knotsV);expect(sculpted.weights).toEqual(surface.weights)
  const moved=surface.controlPoints.flat().map((p,i)=>[p,sculpted.controlPoints.flat()[i]] as const).filter(([a,b])=>a.some((v,k)=>v!==b[k]))
  expect(moved).toHaveLength(1);expect(moved[0][1]).toEqual([2,1,3])
  expect(evaluateNurbsSurface(sculpted,0,0).point).toEqual(evaluateNurbsSurface(surface,0,0).point)
 })
 it('sdf sculpt strokes add and remove material and remain polygonizable',()=>{
  const base:SdfField={kind:'sphere',center:[0,0,0],radius:1}
  const added=sculptSdfSphere(base,[1.5,0,0],0.5)
  expect(added.kind).toBe('union');expect(evaluateSdf(base,[1.5,0,0])).toBe(0.5);expect(evaluateSdf(added,[1.5,0,0])).toBe(-0.5)
  const removed=sculptSdfSphere(base,[0,0,0],0.5,true)
  expect(removed.kind).toBe('difference');expect(evaluateSdf(removed,[0,0,0])).toBe(0.5);expect(evaluateSdf(removed,[0.75,0,0])).toBeLessThan(0)
  const strokes=sculptSdfSphere(sculptSdfSphere(removed,[0,1,0],0.3),[0,-1,0],0.3,true)
  expect(evaluateSdf(strokes,[0,1,0])).toBeLessThan(0);expect(evaluateSdf(strokes,[0,-1,0])).toBeGreaterThan(0)
  const mesh=tessellateSdf(strokes,{min:[-2,-2,-2],max:[2,2,2],cells:[16,16,16]})
  expect(mesh.report.closed).toBe(true);expect(mesh.report.degenerateTriangles).toBe(0)
  for(const radius of [0,-1,Number.NaN,Number.POSITIVE_INFINITY])expect(()=>sculptSdfSphere(base,[0,0,0],radius)).toThrow()
  expect(()=>sculptSdfSphere(base,[Number.NaN,0,0],1)).toThrow()
  expect(()=>sculptSdfSphere({kind:'sphere',center:[0,0,0],radius:-1},[0,0,0],1)).toThrow()
 })
})
