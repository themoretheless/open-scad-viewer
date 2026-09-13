import {it,expect} from 'vitest'
import {cadOperation,type CadOptions} from '../src/services/cadWorkbench'
import {createBrepBox,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const options:CadOptions={action:'hole',ids:['a'],sketches:[],axis:[0,0,1],origin:[5,5,0],amount:0,count:3,width:2,height:3,depth:12,pitch:1,secondary:4,mode:'plain',pathId:'',profileIds:[]}
const stock=()=>{const brep=createBrepBox([0,0,0],[10,10,10]);return {version:1 as const,sketches:[],bodies:[{id:'a',name:'Stock',brep,mesh:tessellateNurbsBrep(brep,1)}]}}
it('cuts through and blind holes while retaining rational cylindrical walls',()=>{
 const input=stock(),before=JSON.stringify(input)
 for(const depth of [6,12]){
  const result=cadOperation(input,{...options,depth}).bodies[0]
  expect(result.brep).toBeDefined();expect(result.id).toBe('a');expect(result.name).toBe('Stock')
  expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(1000-Math.PI*Math.min(depth,10),6)
  expect(result.brep!.faces.some(f=>f.surface.weights.flat().some(w=>w!==1))).toBe(true)
  expect(inspectPolygonMesh(result.mesh).closed).toBe(true)
 }
 expect(JSON.stringify(input)).toBe(before)
})
it('cuts a retained counterbore and places a hole along a non-Z axis',()=>{
 const input=stock()
 const counter=cadOperation(input,{...options,mode:'counterbore'}).bodies[0]
 expect(analyzeNurbsBrep(counter.brep!).signedVolumeMm3).toBeCloseTo(1000-19*Math.PI,6)
 expect(inspectPolygonMesh(counter.mesh).closed).toBe(true)
 const sideways=cadOperation(input,{...options,axis:[Number.MAX_VALUE,0,0],origin:[0,5,5]}).bodies[0]
 expect(analyzeNurbsBrep(sideways.brep!).signedVolumeMm3).toBeCloseTo(1000-10*Math.PI,6)
})
it('supports repeated holes and rejects invalid requests without changing the input',()=>{
 const first=cadOperation(stock(),{...options,origin:[3,3,0]}),before=JSON.stringify(first)
 const second=cadOperation(first,{...options,origin:[7,7,0]}).bodies[0]
 expect(analyzeNurbsBrep(second.brep!).signedVolumeMm3).toBeCloseTo(1000-20*Math.PI,6)
 for(const patch of [{width:0},{axis:[0,0,0] as [number,number,number]},{mode:'counterbore',secondary:1},{mode:'counterbore',height:20}])expect(()=>cadOperation(first,{...options,...patch})).toThrow()
 expect(JSON.stringify(first)).toBe(before)
})
it('refuses unsupported countersink Boolean geometry without degrading retained surfaces',()=>{
 const input=stock(),before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,mode:'countersink'})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})
