import {it,expect} from 'vitest'
import {cadOperation,bounds,type CadOptions} from '../src/services/cadWorkbench'
import {createBrepBox,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
const options:CadOptions={action:'align',ids:['1','0','2'],sketches:[],axis:[1,0,0],origin:[0,0,0],amount:0,count:3,width:1,height:1,depth:1,pitch:1,secondary:1,mode:'min',pathId:'',profileIds:[]}
const document=()=>({version:1 as const,sketches:[],bodies:[0,11,40].map((x,i)=>{const brep=createBrepBox([x,0,0],[x+2+i,3,4]);return {id:String(i),name:`Part ${i}`,brep,mesh:tessellateNurbsBrep(brep,1)}})})
it('aligns to the first selected body and keeps B-rep and mesh synchronized',()=>{
 const input=document(),before=JSON.stringify(input)
 for(const mode of ['min','max','center']){
  const result=cadOperation(input,{...options,mode})
  const key=(b:typeof result.bodies[number])=>{const a=bounds([b]);return mode==='min'?a.min[0]:mode==='max'?a.max[0]:(a.min[0]+a.max[0])/2}
  result.bodies.forEach((b,i)=>{
   expect(key(b)).toBe(key(input.bodies[1]))
   expect(bounds([{...b,mesh:{positions:b.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([b]))
   expect(analyzeNurbsBrep(b.brep!).signedVolumeMm3).toBeCloseTo((2+i)*12,8)
   expect(b.brep!.loops).toEqual(input.bodies[i].brep.loops)
  })
 }
 expect(JSON.stringify(input)).toBe(before)
})
it('distributes unequal bodies by bounds centers regardless of selection order',()=>{
 const input=document(),result=cadOperation(input,{...options,action:'distribute',mode:'center',axis:[Number.MAX_VALUE,0,0]})
 const centers=result.bodies.map(b=>{const a=bounds([b]);return (a.min[0]+a.max[0])/2})
 expect(centers).toEqual([1,21.5,42])
 result.bodies.forEach(b=>expect(bounds([{...b,mesh:{positions:b.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([b])))
 expect(cadOperation(input,{...options,action:'distribute',mode:'center',axis:[Number.MIN_VALUE,0,0]})).toEqual(result)
})
it('refuses invalid later geometry without partial document changes',()=>{
 const input=document();input.bodies[2].brep.vertices[0].point[0]+=1
 const before=JSON.stringify(input)
 expect(()=>cadOperation(input,options)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 expect(()=>cadOperation(document(),{...options,axis:[1,1,0]})).toThrow('world axis')
 expect(()=>cadOperation(document(),{...options,ids:['0']})).toThrow('at least two')
 expect(cadOperation(document(),options).bodies).toHaveLength(3)
})
it('moves only the joint child and rotates its mesh and B-rep about an offset pivot',()=>{
 const input=document(),before=JSON.stringify(input)
 const result=cadOperation(input,{...options,action:'joint',ids:['0','1'],mode:'hinge',axis:[0,0,1],origin:[10,2,0],amount:90})
 expect(result.bodies[0]).toEqual(input.bodies[0]);expect(result.bodies[2]).toEqual(input.bodies[2])
 const child=result.bodies[1]
 child.brep!.vertices.forEach((v,i)=>{const p=input.bodies[1].brep.vertices[i].point;[12-p[1],p[0]-8,p[2]].forEach((x,k)=>expect(v.point[k]).toBeCloseTo(x,11))})
 expect(bounds([{...child,mesh:{positions:child.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([child]))
 expect(analyzeNurbsBrep(child.brep!).signedVolumeMm3).toBeCloseTo(36,9)
 expect(JSON.stringify(input)).toBe(before)
})
it('slides retained bodies along the normalized axis and refuses an invalid child',()=>{
 const input=document(),operation={...options,action:'joint' as const,ids:['0','1'],mode:'slider',axis:[0,0,Number.MAX_VALUE] as [number,number,number],amount:-5}
 const result=cadOperation(input,operation),child=result.bodies[1]
 expect(bounds([child])).toEqual({min:[11,0,-5],max:[14,3,-1]})
 expect(bounds([{...child,mesh:{positions:child.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([child]))
 input.bodies[1].brep.vertices[0].point[0]+=1;const before=JSON.stringify(input)
 expect(()=>cadOperation(input,operation)).toThrow();expect(JSON.stringify(input)).toBe(before)
 expect(()=>cadOperation(document(),{...operation,ids:['0']})).toThrow('parent')
 expect(()=>cadOperation(document(),{...operation,axis:[0,0,0]})).toThrow('Zero direction')
})
it('patterns retained bodies along a direction with new IDs and synchronized topology',()=>{
 const input=document(),before=JSON.stringify(input)
 const result=cadOperation(input,{...options,action:'pattern',ids:['0','1'],axis:[0,0,2],amount:7,count:3})
 expect(result.bodies).toHaveLength(7)
 expect(new Set(result.bodies.map(b=>b.id)).size).toBe(7)
 expect(result.bodies[2]).toEqual(input.bodies[2])
 for(const [i,z] of [[0,0],[1,0],[3,7],[4,7],[5,14],[6,14]]){
  const body=result.bodies[i]
  expect(bounds([body]).min[2]).toBe(z)
  expect(bounds([{...body,mesh:{positions:body.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([body]))
  expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(i===0||i===3||i===5?24:36,8)
 }
 expect(JSON.stringify(input)).toBe(before)
})
it('places the group center at equal arc-length stations of a placed sketch path',()=>{
 const input=document()
 const path={id:'path',name:'Path',closed:false,points:[[0,0],[0,0],[6,0],[6,8]] as [number,number][],plane:{origin:[10,20,30] as [number,number,number],u:[0,1,0] as [number,number,number],v:[0,0,1] as [number,number,number]}}
 const result=cadOperation(input,{...options,action:'pattern',ids:['0','1'],sketches:[path],pathId:'path',count:3})
 for(const [ids,expected] of [[[0,1],[10,20,30]],[[3,4],[10,26,31]],[[5,6],[10,26,38]]] as [number[],number[]][]){
  const b=bounds(ids.map(i=>result.bodies[i]))
  b.min.map((x,k)=>(x+b.max[k])/2).forEach((x,k)=>expect(x).toBeCloseTo(expected[k],10))
  ids.forEach(i=>{const body=result.bodies[i];expect(bounds([{...body,mesh:{positions:body.brep!.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([body]))})
 }
})
it('rejects invalid pattern inputs and late body failures without changing the source',()=>{
 const input=document();input.bodies[1].brep.vertices[0].point[0]+=1;const before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,action:'pattern',ids:['0','1']})).toThrow();expect(JSON.stringify(input)).toBe(before)
 for(const count of [1,101,2.5])expect(()=>cadOperation(document(),{...options,action:'pattern',count})).toThrow()
 const path={id:'p',name:'Zero',closed:false,points:[[0,0],[0,0]] as [number,number][]}
 expect(()=>cadOperation(document(),{...options,action:'pattern',sketches:[path],pathId:'p'})).toThrow('Zero-length')
})
it('refuses oversized pattern expansion before constructing the full group and recovers',()=>{
 const input=document()
 // Unreferenced positions are still transformed and retained in exchange meshes.
 const extra=Array.from({length:30000},(_,i)=>[i/1000,0,0]).flat();input.bodies[0].mesh.positions=Float64Array.from([...input.bodies[0].mesh.positions,...extra])
 const before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,action:'pattern',ids:['0'],count:100})).toThrow('transport capacity')
 expect(JSON.stringify(input)).toBe(before)
 expect(cadOperation(document(),{...options,action:'pattern',ids:['0'],count:3}).bodies).toHaveLength(5)
})
