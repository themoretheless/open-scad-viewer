import {expect,it} from 'vitest'
import {createBrepBox,createBrepSphere,tessellateNurbsBrep} from '../src/services/geometry/brep'
import type {PolygonMesh} from '../src/services/geometry/polygon'
import {prepareSolidDisplay} from '../src/services/solidDisplayPreparation'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
// Independent compatibility oracle: the previous DirectModeler display algorithm.
function stats(mesh:PolygonMesh){
 const centers:number[][]=[],normals:number[][]=[]
 for(let t=0;t<mesh.indices.length;t+=3){
  const [a,b,c]=Array.from(mesh.indices.slice(t,t+3),i=>Array.from(mesh.positions.slice(i*3,i*3+3)))
  centers.push([0,1,2].map(i=>(a[i]+b[i]+c[i])/3))
  const u=b.map((v,i)=>v-a[i]),w=c.map((v,i)=>v-a[i])
  const n=[u[1]*w[2]-u[2]*w[1],u[2]*w[0]-u[0]*w[2],u[0]*w[1]-u[1]*w[0]],len=Math.hypot(...n)||1
  normals.push(n.map(v=>v/len))
 }
 return {centers,normals}
}
function reference(mesh:PolygonMesh,dense:PolygonMesh){
 const work=stats(mesh),view=stats(dense)
 const map=view.centers.map((c,t)=>{
  let best=0,score=Infinity
  work.centers.forEach((w,i)=>{
   const facing=1-view.normals[t].reduce((s,n,k)=>s+n*work.normals[i][k],0)
   const candidate=Math.hypot(...c.map((x,k)=>x-w[k]))*(1+facing*4)
   if(candidate<score){score=candidate;best=i}
  });return best
 })
 const sums=new Map<string,number[]>(),key=(i:number)=>Array.from(dense.positions.slice(i*3,i*3+3),x=>x.toFixed(5)).join(',')
 view.normals.forEach((n,t)=>{for(let k=0;k<3;k++){const id=key(dense.indices[t*3+k]),sum=sums.get(id)??[0,0,0];sums.set(id,sum.map((v,j)=>v+n[j]))}})
 const normals=view.normals.map((n,t)=>{
  const sum=[0,1,2].map(k=>sums.get(key(dense.indices[t*3+k]))!).reduce((a,b)=>a.map((x,j)=>x+b[j]),[0,0,0]),len=Math.hypot(...sum)
  return len>1e-9?sum.map(x=>x/len):n
 })
 return {map,normals}
}
it.each(['box','sphere'])('matches the previous display and picking result for %s',kind=>{
 const brep=kind==='box'?createBrepBox([0,0,0],[2,3,4]):createBrepSphere(5)
 const mesh=tessellateNurbsBrep(brep,3),dense=tessellateNurbsBrep(brep,12),expected=reference(mesh,dense),actual=prepareSolidDisplay(mesh,brep)
 expect(Array.from(actual.mesh.positions)).toEqual(Array.from(dense.positions))
 expect(Array.from(actual.mesh.indices)).toEqual(Array.from(dense.indices))
 expect(actual.map).toEqual(expected.map)
 actual.normals.forEach((n,t)=>n.forEach((x,k)=>expect(x).toBeCloseTo(expected.normals[t][k],12)))
})
it('rejects malformed display results before the client applies them',()=>{
 const mesh={positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])}
 const expectation=mainSolidExpectation({kind:'displayMesh',mesh,segments:12})
 const result={mesh,map:[0],normals:[[0,0,1]]}
 expect(mainSolidResult(expectation,result)).toBe(true)
 expect(mainSolidResult(expectation,{...result,map:[1]})).toBe(false)
 expect(mainSolidResult(expectation,{...result,normals:[[NaN,0,1]]})).toBe(false)
 expect(mainSolidResult(expectation,{...result,mesh:{...mesh,uv:new Float64Array([1])}})).toBe(false)
})
