import {extrudeSketchProfile} from '../src/services/directExtrusion'
import {createBrepBox,booleanNurbsBrep,transformNurbsBrep,tessellateNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
import type {DirectBody,DirectSketch} from '../src/services/directModeling'
const profile=(points:[number,number][]):DirectSketch=>({id:'profile',name:'Profile',closed:true,points})
const circle=(id:string,radius:number):DirectSketch=>({id,name:id,closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius,start:0,sweep:360}})
export function cadRoadmapParts():Array<{body:DirectBody;volume:number;bounds:number[][]}> {
 const definitions:Array<[string,NurbsBrep,number,number[][]]>=[
  ['bracket',extrudeSketchProfile([profile([[0,0],[40,0],[40,5],[5,5],[5,30],[0,30]])],20),6500,[[0,0,0],[40,30,20]]],
  ['flange',extrudeSketchProfile([circle('outer',20),circle('hole',5)],6),2250*Math.PI,[[-20,-20,0],[20,20,6]]],
  ['enclosure',booleanNurbsBrep(createBrepBox([0,0,0],[40,30,20]),createBrepBox([2,2,2],[38,28,22]),'difference'),7152,[[0,0,0],[40,30,20]]],
  ['edited-box',transformNurbsBrep(createBrepBox([0,0,0],[2,3,4]),[[2,0,0,5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]),48,[[5,0,0],[9,3,4]]],
 ]
 return definitions.map(([id,brep,volume,bounds])=>{const mesh=tessellateNurbsBrep(brep,4);return {body:{id,name:id,brep,mesh:{positions:mesh.positions,indices:mesh.indices}},volume,bounds}})
}
