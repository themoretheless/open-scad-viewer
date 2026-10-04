import {expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {authorBrepProfile,transformBrepProfile} from '../src/services/geometry/brepProfile'
import {withRetainedProfile} from '../src/services/retainedSketchProfile'
import {applySolidRevolve,type SolidRevolveOptions} from '../src/services/solidRevolve'
import type {DirectDocument} from '../src/services/directModeling'
const options:SolidRevolveOptions={sketchId:'profile',geometry:'exact',operation:'new',targetId:'',id:'result',name:'Revolved',tessellation:2,axis:'y',offset:0,angle:360,segments:32}
it('revolves retained rational circular controls without replacing them by display chords',async()=>{
 await warmGeometryKernel()
 const profile=transformBrepProfile(authorBrepProfile({kind:'circle',radius:1}),[1,0,0,0,0,1,0,0,0,0,1,0,5,0,0,1])
 const sketch=withRetainedProfile({id:'profile',name:'Circle',points:[],closed:true},profile)
 const doc:DirectDocument={version:1,bodies:[],sketches:[sketch]},before=structuredClone(doc)
 const result=applySolidRevolve(doc,options)
 expect(result.bodies).toHaveLength(1)
 expect(result.bodies[0].brep!.faces.some(face=>face.surface.weights.flat().some(weight=>weight!==1))).toBe(true)
 expect(doc).toEqual(before)
 expect(result.sketches[0].retainedProfile).toEqual(profile)
})
it('retains inner contours on partial revolution caps and rejects faceted substitution',async()=>{
 await warmGeometryKernel()
 const profile=authorBrepProfile({kind:'polygon',rings:[[[3,0],[6,0],[6,4],[3,4]],[[4,1],[4,3],[5,3],[5,1]]]})
 const sketch=withRetainedProfile({id:'profile',name:'Holed',points:[],closed:true},profile)
 const doc:DirectDocument={version:1,bodies:[],sketches:[sketch]}
 const result=applySolidRevolve(doc,{...options,angle:90})
 expect(result.bodies[0].brep!.faces.filter(face=>face.holes.length===1)).toHaveLength(2)
 expect(result.sketches[0].retainedProfile).toEqual(profile)
 expect(()=>applySolidRevolve(doc,{...options,geometry:'faceted'})).toThrow(/retained curve profiles/)
})
it.each(['x','y'] as const)('matches polygon revolution for axis %s, offsets, both sides and tilted placement',async(axis)=>{
 await warmGeometryKernel()
 for(const side of [-1,1])for(const angle of [-90,90,360]){
  const points: [number,number][]=axis==='y'?[[2+side*3,0],[2+side*6,0],[2+side*6,4],[2+side*3,4]]:[[0,2+side*3],[4,2+side*3],[4,2+side*6],[0,2+side*6]]
  const plain={id:'profile',name:'Rectangle',points,closed:true,plane:{origin:[11,13,17] as [number,number,number],u:[0,1,0] as [number,number,number],v:[0,0,1] as [number,number,number]}}
  const retained=withRetainedProfile(plain,authorBrepProfile({kind:'polygon',rings:[points]}))
  const run=(sketch:typeof plain)=>applySolidRevolve({version:1,bodies:[],sketches:[sketch]},{...options,axis,offset:2,angle}).bodies[0].brep!
  const expected=run(plain),actual=run(retained)
  expect(actual.vertices).toHaveLength(expected.vertices.length)
  expect(actual.faces).toHaveLength(expected.faces.length)
  const sorted=(points:number[][])=>points.map(p=>p.map(x=>Number(x.toFixed(9)))).sort((a,b)=>a[0]-b[0]||a[1]-b[1]||a[2]-b[2])
  expect(sorted(actual.vertices.map(v=>v.point))).toEqual(sorted(expected.vertices.map(v=>v.point)))
 }
})
it('rejects a retained profile crossing the axis without modifying the document',async()=>{
 await warmGeometryKernel()
 const sketch=withRetainedProfile({id:'profile',name:'Crossing',closed:true,points:[]},authorBrepProfile({kind:'circle',radius:1}))
 const document:DirectDocument={version:1,bodies:[],sketches:[sketch]},before=structuredClone(document)
 expect(()=>applySolidRevolve(document,options)).toThrow(/one side of its axis/)
 expect(document).toEqual(before)
})
it('revolves disconnected retained regions as separate solids within one authored body',async()=>{
 await warmGeometryKernel()
 const profile=authorBrepProfile({kind:'polygon',rings:[[[3,0],[4,0],[4,1],[3,1]],[[5,2],[6,2],[6,3],[5,3]]]})
 const sketch=withRetainedProfile({id:'profile',name:'Two regions',closed:true,points:[]},profile)
 const document:DirectDocument={version:1,bodies:[],sketches:[sketch]},before=structuredClone(document)
 const result=applySolidRevolve(document,options)
 expect(result.bodies).toHaveLength(1)
 expect(result.bodies[0].brep!.bodies).toHaveLength(2)
 expect(result.sketches[0].retainedProfile!.loops).toHaveLength(2)
 expect(document).toEqual(before)
})
