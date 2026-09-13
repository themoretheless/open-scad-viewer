import {it,expect} from 'vitest'
import {cadOperation,type CadOptions} from '../src/services/cadWorkbench'
import {createBrepBox,createBrepSphere,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const options:CadOptions={action:'union',ids:['a','b'],sketches:[],axis:[0,0,1],origin:[0,0,0],amount:0,count:3,width:1,height:1,depth:1,pitch:1,secondary:1,mode:'min',pathId:'',profileIds:[]}
const body=(id:string,min:number[],max:number[])=>{const brep=createBrepBox(min,max);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,1)}}
const stock=()=>({version:1 as const,sketches:[],bodies:[body('a',[0,0,0],[10,10,10]),body('b',[5,0,0],[15,10,10]),body('unselected',[30,0,0],[31,1,1])]})
it('applies retained Boolean operations with matching B-rep and mesh and preserves source bodies',()=>{
 const input=stock(),before=JSON.stringify(input)
 for(const [action,volume] of [['union',1500],['difference',500],['intersection',500]] as const){
  const result=cadOperation(input,{...options,action})
  expect(result.bodies).toHaveLength(2);expect(result.bodies[0].id).toBe('a');expect(result.bodies[0].name).toBe('a')
  expect(analyzeNurbsBrep(result.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(volume,7)
  expect(inspectPolygonMesh(result.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(volume,7)
  expect(result.bodies[1]).toEqual(input.bodies[2])
 }
 expect(JSON.stringify(input)).toBe(before)
})
it('keeps operand order and supports repeated retained Boolean operations',()=>{
 const input=stock(),reverse=cadOperation(input,{...options,action:'difference',ids:['b','a']})
 expect(reverse.bodies[0].id).toBe('b')
 expect(Math.min(...reverse.bodies[0].brep!.vertices.map(v=>v.point[0]))).toBe(10)
 const union=cadOperation(input,options)
 union.bodies.push(body('c',[0,0,0],[2,10,10]))
 const cut=cadOperation(union,{...options,action:'difference',ids:['a','c']})
 expect(analyzeNurbsBrep(cut.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(1300,7)
})
it('removes selected bodies for regularized empty results but preserves unrelated bodies',()=>{
 const input=stock();input.bodies[1]=body('b',[0,0,0],[10,10,10])
 expect(cadOperation(input,{...options,action:'difference'}).bodies).toEqual([input.bodies[2]])
 input.bodies[1]=body('b',[20,0,0],[21,1,1])
 expect(cadOperation(input,{...options,action:'intersection'}).bodies).toEqual([input.bodies[2]])
})
it('refuses invalid later operands, mixed representations and unsupported curved pairs atomically',()=>{
 const input=stock();input.bodies[2].brep.vertices[0].point[0]+=0.5;const before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,ids:['a','b','unselected']})).toThrow();expect(JSON.stringify(input)).toBe(before)
 const mixed=stock();delete (mixed.bodies[1] as {brep?:unknown}).brep
 expect(()=>cadOperation(mixed,options)).toThrow('all retain B-rep')
 const brep=createBrepSphere(4),sphere={id:'b',name:'sphere',brep,mesh:tessellateNurbsBrep(brep,4)},curved={...stock(),bodies:[stock().bodies[0],sphere]},snapshot=JSON.stringify(curved)
 expect(()=>cadOperation(curved,options)).toThrow();expect(JSON.stringify(curved)).toBe(snapshot)
})
it('retains rational cylindrical walls after an admitted curved Boolean',async()=>{
 const {createBrepCylinder}=await import('../src/services/geometry/brep')
 const bodies=[4,2].map((radius,i)=>{const brep=createBrepCylinder(radius,5);return {id:i?'b':'a',name:'Cylinder',brep,mesh:tessellateNurbsBrep(brep,4)}})
 const result=cadOperation({version:1,sketches:[],bodies},{...options,action:'difference'}).bodies[0]
 expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(60*Math.PI,6)
 expect(result.brep!.faces.some(f=>f.surface.weights.flat().some(w=>w!==1))).toBe(true)
 expect(inspectPolygonMesh(result.mesh).closed).toBe(true)
})
