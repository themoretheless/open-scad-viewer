import {describe,it,expect} from 'vitest'
import {paintCpuOrbit} from '../src/services/cpuOrbitPaint'
describe('CPU orbit painter',()=>{
 it('clears the previous camera frame and preserves ordered alpha polygons and opaque strokes',()=>{
  const events:unknown[][]=[]
  const ctx=new Proxy({}, {set(target,key,value){events.push([key,value]);return Reflect.set(target,key,value)},get(target,key){return Reflect.get(target,key)??((...args:unknown[])=>events.push([key,...args]))}}) as CanvasRenderingContext2D
  const triangles=[{vertices:[[1,2],[3,4],[5,6]],color:'red',opacity:1,stroke:true},{vertices:[[7,8],[9,10],[11,12]],color:'blue',opacity:.3,stroke:false}]
  const before=JSON.stringify(triangles)
  paintCpuOrbit(ctx,triangles,800,800,100,-50,-25,400)
  expect(events.slice(0,5)).toEqual([['setTransform',1,0,0,1,0,0],['clearRect',0,0,800,800],['setTransform',8,0,0,8,400,200],['lineWidth',.25],['lineJoin','round']])
  expect(events.filter(e=>e[0]==='globalAlpha')).toEqual([['globalAlpha',1],['globalAlpha',.3],['globalAlpha',1]])
  expect(events.filter(e=>['fill','stroke'].includes(String(e[0])))).toEqual([['fill'],['stroke'],['fill']])
  expect(events.filter(e=>e[0]==='moveTo')).toEqual([['moveTo',1,2],['moveTo',7,8]])
  expect(JSON.stringify(triangles)).toBe(before)
 })
 it('clears an empty replacement scene',()=>{
  const calls:string[]=[]
  const ctx=new Proxy({}, {get:(_,key)=>(..._args:unknown[])=>calls.push(String(key)),set:()=>true}) as CanvasRenderingContext2D
  paintCpuOrbit(ctx,[],100,100,10,0,0,100)
  expect(calls).toEqual(['setTransform','clearRect','setTransform'])
 })
})
