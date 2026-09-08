import {describe,it,expect} from 'vitest'
import {extrudePolygonProfile,revolvePolygonProfile,extrudePolygonFaces,deformPolygonMesh} from '../src/services/polygonKernel'
import {extrudeSubdivision,sweepSubdivision,tessellateSubdivision} from '../src/services/subdivisionKernel'
import {evaluateSdf,sculptSdfSphere,deformSdf,type SdfField} from '../src/services/sdfKernel'
import {solveNativeSketch} from '../src/services/sketchKernel'
const square=[[0,0],[2,0],[2,2],[0,2]]
describe('native modeling through WASM',()=>{
 it('extrudes holes and revolves a profile touching the axis',()=>{
  const mesh=extrudePolygonProfile({outer:square,holes:[[[0.5,0.5],[0.5,1.5],[1.5,1.5],[1.5,0.5]]]},[0,0,3])
  expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeCloseTo(9)
  const cylinder=revolvePolygonProfile([[0,0],[2,0],[2,3],[0,3]])
  expect(cylinder.report.closed).toBe(true);expect(cylinder.report.degenerateTriangles).toBe(0)
  expect(cylinder.report.signedVolumeMm3).toBeCloseTo(32*Math.sin(Math.PI/16)*6,5)
 })
 it('extrudes selected mesh faces and preserves closure under twist',()=>{
  const mesh=extrudePolygonProfile({outer:square},[0,0,2])
  const top=[];for(let i=0;i<mesh.indices.length;i+=3)if(mesh.indices.slice(i,i+3).every(v=>mesh.positions[v*3+2]===2))top.push(i/3)
  const edited=extrudePolygonFaces(mesh,top,[0,0,1]);expect(edited.report.closed).toBe(true);expect(edited.report.signedVolumeMm3).toBeCloseTo(12)
  expect(deformPolygonMesh(edited,{kind:'twist',origin:[0,0,0],radians_per_unit:0.1}).report.closed).toBe(true)
 })
 it('constructs native quad cages before subdivision',()=>{
  const cage=extrudeSubdivision(square.map(p=>[...p,0]),[0,0,3]);expect(cage.faces).toHaveLength(6);expect(cage.faces.every(f=>f.length===4)).toBe(true)
  expect(tessellateSubdivision(cage,1).report.closed).toBe(true)
  expect(sweepSubdivision(square,[[0,0,0],[0,0,2],[1,0,4]]).faces).toHaveLength(10)
 })
 it('extrudes, revolves, deforms and sculpts fields without a mesh',()=>{
  const field:SdfField={kind:'extrude',profile:{outer:square,holes:[]},half_height:2}
  expect(evaluateSdf(field,[1,1,0])).toBe(-1);expect(evaluateSdf(field,[1,1,3])).toBe(1)
  expect(evaluateSdf({kind:'revolve',profile:{outer:[[1,-1],[2,-1],[2,1],[1,1]],holes:[]}},[1.5,0,0])).toBe(-0.5)
  expect(evaluateSdf(sculptSdfSphere(field,[1,1,0],0.5,true),[1,1,0])).toBe(0.5)
  expect(evaluateSdf(deformSdf(field,{kind:'twist',origin:[0,0,0],radians_per_unit:1}),[1,1,0])).toBe(-1)
  expect(()=>deformSdf(field,{kind:'bend',origin:[0,0,0],radius:5})).toThrow()
 })
 it('reports solved and conflicting sketch constraints',()=>{
  const fixed={kind:'fix' as const,point:0,at:[0,0]}
  expect(solveNativeSketch({points:[[0,0]],constraints:[fixed]}).status).toBe('solved')
  expect(solveNativeSketch({points:[[0,0]],constraints:[fixed,{...fixed,at:[1,0]}]}).status).toBe('not_converged')
 })
})
