import {expect,it} from 'vitest'
import {extrudeSketchProfile} from '../src/services/directExtrusion'
import {analyzeNurbsBrep,createBrepBox,booleanNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import type {DirectSketch} from '../src/services/directModeling'
import {inspectSolidDisplay} from '../src/services/solidDiagnostics'
const profile=(points:[number,number][]):DirectSketch=>({id:'profile',name:'Profile',closed:true,points})
it('qualifies bracket, flange and enclosure against independent dimensions and volumes',()=>{
 const bracket=extrudeSketchProfile([profile([[0,0],[40,0],[40,5],[5,5],[5,30],[0,30]])],20)
 const circle=(id:string,radius:number):DirectSketch=>({id,name:id,closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius,start:0,sweep:360}})
 const flange=extrudeSketchProfile([circle('outer',20),circle('hole',5)],6)
 const enclosure=booleanNurbsBrep(createBrepBox([0,0,0],[40,30,20]),createBrepBox([2,2,2],[38,28,22]),'difference')
 for(const [brep,expected,bounds] of [[bracket,6500,[[0,0,0],[40,30,20]]],[flange,2250*Math.PI,[[-20,-20,0],[20,20,6]]],[enclosure,7152,[[0,0,0],[40,30,20]]]] as const){
  expect(analyzeNurbsBrep(brep).signedVolumeMm3).toBeCloseTo(expected,5)
  for(let axis=0;axis<3;axis++){
   const values=brep.vertices.map(v=>v.point[axis]);expect(Math.min(...values)).toBeCloseTo(bounds[0][axis],6);expect(Math.max(...values)).toBeCloseTo(bounds[1][axis],6)
  }
  expect(tessellateNurbsBrep(brep,4).report.closed).toBe(true)
 }
})
it('locates the open boundary of a known defect and sections a closed control body without edits',()=>{
 const mesh=tessellateNurbsBrep(createBrepBox([0,0,0],[10,10,10]),1)
 const before=Array.from(mesh.positions),closed=inspectSolidDisplay(mesh,5)
 expect(closed.report.boundaryEdges).toBe(0);expect(closed.section.contours.length).toBeGreaterThan(0)
 const broken={positions:mesh.positions,indices:mesh.indices.slice(3)}
 const defect=inspectSolidDisplay(broken,5)
 expect(defect.report.boundaryEdges).toBe(3);expect(defect.boundaries.length).toBeGreaterThan(0)
 expect(defect.boundaries.flat().every(p=>p.length===3&&p.every(Number.isFinite))).toBe(true)
 expect(Array.from(mesh.positions)).toEqual(before)
})
