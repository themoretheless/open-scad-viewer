import {authorBrepProfile,booleanBrepProfiles} from '../src/services/geometry/brepProfile'
import {describe,it,expect} from 'vitest'
import {cleanSketchContours} from '../src/services/editableSketchOperations'
import {xyPlane} from '../src/services/directSketchGeometry'
import type {DirectSketch} from '../src/services/directModeling'
const sketch:DirectSketch={id:'source',name:'Source',plane:xyPlane(),closed:true,points:[[0,0],[10,0],[10,10],[5,10],[0,10]]}
describe('native contour cleanup',()=>{
 it('removes redundant edges and creates a retained filled profile without modifying the source',()=>{
  const cleaned=cleanSketchContours(sketch,.01,'clean')
  expect(cleaned.id).toBe('clean');expect(cleaned.retainedProfile?.areaMm2).toBeCloseTo(100);expect(cleaned.points).toHaveLength(4);expect(sketch.points).toHaveLength(5)
 })
 it('cleans open paths while preserving endpoints and rejects invalid tolerances',()=>{
  const open={...sketch,closed:false};const cleaned=cleanSketchContours(open,.01,'clean')
  expect(cleaned.closed).toBe(false);expect(cleaned.points[0]).toEqual(open.points[0]);expect(cleaned.points.at(-1)).toEqual(open.points.at(-1));expect(cleaned.points.length).toBeLessThan(open.points.length)
  expect(()=>cleanSketchContours(sketch,Infinity,'clean')).toThrow(/tolerance/)
 })
})
it('cleans an already retained ring without filling its hole or modifying source curves',()=>{
 const source:DirectSketch={...sketch,retainedProfile:authorBrepProfile({kind:'circle',radius:5})}
 const inner=authorBrepProfile({kind:'circle',radius:2});source.retainedProfile=booleanBrepProfiles(source.retainedProfile!,inner,'difference')
 const before=JSON.stringify(source),cleaned=cleanSketchContours(source,.05,'clean')
 expect(cleaned.retainedProfile?.loops).toHaveLength(2);expect(cleaned.retainedProfile!.loops.flat().every(c=>c.degree===1)).toBe(true)
 expect(cleaned.retainedProfile!.areaMm2).toBeCloseTo(21*Math.PI,0);expect(JSON.stringify(source)).toBe(before)
})
