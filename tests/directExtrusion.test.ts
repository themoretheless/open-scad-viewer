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

import {combineSketchProfiles,unionSketchProfiles,withRetainedProfile,sketchProfile} from '../src/services/retainedSketchProfile'
import {parseDirectDocument,extrudeSketchBrep,DirectHistory} from '../src/services/directModeling'
import {transformSketch} from '../src/services/directSketchGeometry'
import {transformSelection} from '../src/services/directSolidTools'
import {booleanBrepProfiles} from '../src/services/geometry/brepProfile'
describe('retained compound sketch profiles',()=>{
 const inputs=():DirectSketch[]=>[rectangle('rect',-4,-3,7,6),{id:'circle',name:'circle',closed:true,points:[],analytic:{kind:'circle',center:[3,0],radius:2,start:0,sweep:360}}]
 it('retains mixed line/arc definitions through union, document reload and exact extrusion',()=>{
  const source=inputs(),before=structuredClone(source),sketch=unionSketchProfiles(source)
  expect(source).toEqual(before);expect(sketch.id).toBe('rect');expect(sketch.retainedProfile!.areaMm2).toBeCloseTo(42+2*Math.PI,8)
  expect(sketch.retainedProfile!.loops.flat().some(c=>c.degree===2)).toBe(true)
  const document=parseDirectDocument(JSON.stringify({version:1,sketches:[sketch],bodies:[]}))
  expect(document.sketches[0].retainedProfile).toEqual(sketch.retainedProfile)
  const body=extrudeSketchBrep(document.sketches[0],5)
  expect(volume(body)).toBeCloseTo((42+2*Math.PI)*5,5)
  expect(body.faces.some(f=>f.surface.degreeU===2||f.surface.degreeV===2)).toBe(true)
 })
 it('transforms retained controls and preserves holes without sampling the extrusion source',()=>{
  const outer=inputs()[0],hole={...inputs()[1],analytic:{kind:'circle' as const,center:[0,0] as [number,number],radius:1,start:0,sweep:360}}
  const original=withRetainedProfile(outer,booleanBrepProfiles(sketchProfile(outer),sketchProfile(hole),'difference'))
  const transformed=transformSketch(original,[20,30],30,2,[0,0])
  expect(transformed.retainedProfile!.loops).toHaveLength(2)
  expect(transformed.retainedProfile!.areaMm2).toBeCloseTo((42-Math.PI)*4,6)
  const document={version:1 as const,sketches:[transformed],bodies:[]}
  const moved=transformSelection(document,['rect'],[0,0,10],[1,0,0],90,1)
  expect(volume(extrudeSketchProfile(moved.sketches,-3))).toBeCloseTo((42-Math.PI)*12,4)
  const history=new DirectHistory(document),before=history.document;history.commit(moved);history.undo();expect(history.document).toEqual(before)
 })
 it('rebuilds display samples from retained curves instead of trusting stale polygon points',()=>{
  const sketch=unionSketchProfiles(inputs());sketch.points=[[100,100],[101,100],[100,101]]
  const parsed=parseDirectDocument(JSON.stringify({version:1,sketches:[sketch],bodies:[]}))
  expect(parsed.sketches[0].points).not.toEqual(sketch.points)
  expect(parsed.sketches[0].retainedProfile).toEqual(sketch.retainedProfile)
 })
})

import {sketchSnapGeometry,resolveModelingSnap} from '../src/services/modelingSnaps'
import {offsetSketch,bakeSketch} from '../src/services/directSketchGeometry'
it('does not expose sampled arc chords as exact snap edges or silently bake unsupported edits',()=>{
 const sketch=unionSketchProfiles([rectangle('r',-4,-3,7,6),{id:'c',name:'Circle',closed:true,points:[],analytic:{kind:'circle',center:[3,0],radius:2,start:0,sweep:360}}])
 const snaps=sketchSnapGeometry([sketch])
 expect(snaps.segments.filter(s=>!s.nurbs).length).toBe(sketch.retainedProfile!.loops.flat().filter(c=>c.degree===1).length)
 const edges=snaps.segments.filter(s=>s.nurbs);expect(edges.length).toBeGreaterThan(0)
 for(const edge of edges){
  const chord=edge.a.map((v,i)=>(v+edge.b[i])/2) as [number,number,number]
  const hit=resolveModelingSnap(chord,{points:[],segments:[edge]},{project:p=>[p[0]*100,p[1]*100],geometry:true,grid:0})
  expect(hit.kind).toBe('edge');expect(Math.hypot(hit.point[0]-3,hit.point[1])).toBeCloseTo(2,9)
  expect(Math.hypot(chord[0]-3,chord[1])).toBeLessThan(2-1e-7)
 }
 expect(snaps.points.length).toBe(sketch.retainedProfile!.loops.flat().length*3)
 expect(()=>bakeSketch(sketch)).toThrow('retained curve profiles')
})

describe('retained profile region operations',()=>{
 const inputs=():DirectSketch[]=>[rectangle('plate',-4,-3,8,6),{id:'circle',name:'Circle cutter',closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius:2,start:0,sweep:360}}]
 it('subtracts a circular hole and intersects an analytic region through JSON and extrusion',()=>{
  for(const operation of ['difference','intersection'] as const){
   const source=inputs(),before=structuredClone(source),result=combineSketchProfiles(source,operation)
   expect(source).toEqual(before);expect(result.id).toBe('plate')
   const area=operation==='difference'?48-4*Math.PI:4*Math.PI
   expect(result.retainedProfile!.areaMm2).toBeCloseTo(area,8)
   expect(result.retainedProfile!.loops).toHaveLength(operation==='difference'?2:1)
   const restored=parseDirectDocument(JSON.stringify({version:1,sketches:[result],bodies:[]})).sketches[0]
   expect(restored.retainedProfile).toEqual(result.retainedProfile)
   expect(volume(extrudeSketchBrep(restored,3))).toBeCloseTo(area*3,5)
  }
 })
 it('subtracts every cutter and refuses empty results without altering the inputs',()=>{
  const source=inputs();source[1].analytic!.radius=.5;source[1].analytic!.center=[-2,0]
  source.push({...structuredClone(source[1]),id:'other',analytic:{...source[1].analytic!,center:[2,0]}})
  const result=combineSketchProfiles(source,'difference')
  expect(result.retainedProfile!.loops).toHaveLength(3);expect(result.retainedProfile!.areaMm2).toBeCloseTo(48-Math.PI/2,8)
  const reversed=inputs().reverse(),before=structuredClone(reversed)
  expect(()=>combineSketchProfiles(reversed,'difference')).toThrow('contains no material');expect(reversed).toEqual(before)
  const disjoint=inputs();disjoint[1].analytic!.center=[20,0]
  expect(()=>combineSketchProfiles(disjoint,'intersection')).toThrow('contains no material')
 })
})

describe('retained region offset',()=>{
 const source=()=>combineSketchProfiles([rectangle('plate',0,0,8,6),{id:'hole',name:'Hole',closed:true,points:[],analytic:{kind:'circle',center:[4,3],radius:1,start:0,sweep:360}}],'difference')
 it('changes both outer boundaries and holes without replacing arcs by display chords',()=>{
  for(const [distance,area] of [[.5,62],[-.5,35-2.25*Math.PI]]){
   const original=source(),before=structuredClone(original),result=offsetSketch(original,distance)
   expect(original).toEqual(before);expect(result.id).toBe(original.id)
   expect(result.retainedProfile!.loops).toHaveLength(2);expect(result.retainedProfile!.areaMm2).toBeCloseTo(area,7)
   expect(result.retainedProfile!.loops.flat().some(c=>c.degree===2)).toBe(true)
   const restored=parseDirectDocument(JSON.stringify({version:1,sketches:[result],bodies:[]})).sketches[0]
   expect(restored.retainedProfile).toEqual(result.retainedProfile)
   expect(volume(extrudeSketchBrep(restored,2))).toBeCloseTo(2*area,5)
  }
 })
 it('refuses complete erosion without deleting the source document',()=>{
  const original=source(),before=structuredClone(original)
  expect(()=>offsetSketch(original,-10)).toThrow('contains no material');expect(original).toEqual(before)
 })
})
