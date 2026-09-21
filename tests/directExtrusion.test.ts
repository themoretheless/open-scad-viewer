import { describe,expect,it } from 'vitest'
import { applyDirectExtrusionProfile, extrudeSketchProfile, buildDirectExtrusion } from '../src/services/directExtrusion'
import { analyzeNurbsBrep, createBrepBox, tessellateNurbsBrep } from '../src/services/geometry/brep'
import { type DirectDocument, type DirectSketch } from '../src/services/directModeling'
import { facePlane, solidTopology } from '../src/services/directSolidTools'
const polygon=(id:string,points:[number,number][]):DirectSketch=>({id,name:id,closed:true,points})
const rectangle=(id:string,x:number,y:number,w:number,h=w)=>polygon(id,[[x,y],[x+w,y],[x+w,y+h],[x,y+h]])
const volume=(b:ReturnType<typeof extrudeSketchProfile>)=>{expect(tessellateNurbsBrep(b,4).report.closed).toBe(true);return analyzeNurbsBrep(b).signedVolumeMm3}
describe('complex sketch extrusion',()=>{
 it('extrudes concave contours with multiple holes and nested islands independent of winding',()=>{
  const contours=[polygon('outer',[[0,0],[8,0],[8,3],[4,3],[4,7],[0,7]]),rectangle('hole',1,4,2),rectangle('second',1,1,1),rectangle('island',1.5,4.5,.5)]
  contours[0].points.reverse()
  expect(volume(extrudeSketchProfile(contours,2))).toBeCloseTo((40-4-1+.25)*2)
 })
 it('keeps round holes analytic on a vertical plane with a negative height',()=>{
  const outer=rectangle('outer',0,0,10),hole:DirectSketch={id:'circle',name:'circle',closed:true,points:[],analytic:{kind:'circle',center:[5,5],radius:2,start:0,sweep:360}}
  const plane={origin:[20,0,0],u:[0,1,0],v:[0,0,1]} as const
  const model=extrudeSketchProfile([outer,hole].map(s=>({...s,plane:structuredClone(plane) as unknown as NonNullable<DirectSketch['plane']>})),-3)
  expect(volume(model)).toBeCloseTo((100-4*Math.PI)*3,5)
  expect(model.edges.some(e=>e.curve.degree===2)).toBe(true)
  expect(Math.min(...model.vertices.map(v=>v.point[0]))).toBeCloseTo(17)
  expect(Math.max(...model.vertices.map(v=>v.point[0]))).toBeCloseTo(20)
 })
 it('rejects crossing or noncoplanar profiles before changing a document',()=>{
  expect(()=>extrudeSketchProfile([rectangle('a',0,0,5),rectangle('b',2,2,5)],2)).toThrow()
  expect(()=>extrudeSketchProfile([rectangle('a',0,0,5),{...rectangle('b',1,1,1),plane:{origin:[0,0,1],u:[1,0,0],v:[0,1,0]}}],2)).toThrow(/same workplane/)
 })
 it.each([0,2])('adds and cuts a face sketch along world axis %i, with matching preview and commit',axis=>{
  const brep=createBrepBox([0,0,0],[10,10,10]),built=tessellateNurbsBrep(brep,2),body={id:'base',name:'Base',brep,mesh:{positions:built.positions,indices:built.indices}}
  const face=solidTopology(body.mesh).faces.find(f=>f.normal[axis]>.99)!,plane=facePlane(body,face)
  // Face coordinates vary by topology order; place a small rectangle around its center.
  const center=plane.u.map((_,i)=>face.center[i]-plane.origin[i]),u=center.reduce((s,v,i)=>s+v*plane.u[i],0),v=center.reduce((s,x,i)=>s+x*plane.v[i],0)
  const sketch={...rectangle('profile',u-1,v-1,2),plane,supportBodyId:'base'}
  const document:DirectDocument={version:1,sketches:[sketch],bodies:[body]},before=JSON.stringify(document)
  for(const [operation,height,expected] of [['union',3,1012],['difference',-3,988]] as const){
   const options={sketchIds:['profile'],height,offset:0,operation,targetId:'base',id:'new'}
   const preview=buildDirectExtrusion(document,options)!,result=applyDirectExtrusionProfile(document,options)
   expect(result.bodies).toHaveLength(1);expect(volume(result.bodies[0].brep!)).toBeCloseTo(expected,5)
   expect(result.bodies[0].mesh).toEqual(preview.mesh)
   expect(JSON.stringify(document)).toBe(before)
  }
 })
})
it('adds and cuts an annular face profile with an exact circular hole',()=>{
 const brep=createBrepBox([0,0,0],[10,10,10]),built=tessellateNurbsBrep(brep,2),body={id:'base',name:'Base',brep,mesh:{positions:built.positions,indices:built.indices}}
 const plane={origin:[0,0,10],u:[1,0,0],v:[0,1,0]} as NonNullable<DirectSketch['plane']>
 const sketches=[{...rectangle('outer',2,2,6),plane},{id:'hole',name:'Hole',closed:true,points:[],analytic:{kind:'circle' as const,center:[5,5] as [number,number],radius:1,start:0,sweep:360},plane}]
 const document:DirectDocument={version:1,sketches,bodies:[body]}
 for(const [operation,height,sign] of [['union',2,1],['difference',-2,-1]] as const){
  const result=applyDirectExtrusionProfile(document,{sketchIds:['outer','hole'],height,offset:0,operation,targetId:'base',id:'unused'})
  expect(result.bodies).toHaveLength(1)
  expect(volume(result.bodies[0].brep!)).toBeCloseTo(1000+sign*2*(36-Math.PI),5)
 }
})
