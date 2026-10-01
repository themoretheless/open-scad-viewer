import {readFileSync} from 'node:fs'
import {parseDirectDocument} from '../src/services/directModeling'
import {expect,it} from 'vitest'
import {createBrepBox,createBrepSphere,tessellateNurbsBrep} from '../src/services/geometry/brep'
import type {PolygonMesh} from '../src/services/geometry/polygon'
import {prepareSolidDisplay} from '../src/services/solidDisplayPreparation'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
// Independent JS reference for picking correspondence and crease-aware normal averaging.
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
 const key=(i:number)=>Array.from(dense.positions.slice(i*3,i*3+3),x=>x.toFixed(5)).join(',')
 const incident=new Map<string,number[][]>()
 view.normals.forEach((n,t)=>{for(let k=0;k<3;k++){const id=key(dense.indices[t*3+k]);incident.set(id,[...(incident.get(id)??[]),n])}})
 const normals=view.normals.map((n,t)=>{
  const eligible=[0,1,2].flatMap(k=>incident.get(key(dense.indices[t*3+k]))!).filter(other=>other.reduce((dot,v,i)=>dot+v*n[i],0)>.5)
  const sum=eligible.reduce((a,b)=>a.map((x,j)=>x+b[j]),[0,0,0]),len=Math.hypot(...sum)
  return len>1e-9?sum.map(x=>x/len):n
 })
 return {map,normals}
}
it.each(['box','sphere'])('preserves display geometry and picking with crease-aware shading for %s',kind=>{
 const brep=kind==='box'?createBrepBox([0,0,0],[2,3,4]):createBrepSphere(5)
 const segments=kind==='sphere'?8:12
 const mesh=tessellateNurbsBrep(brep,3),dense=tessellateNurbsBrep(brep,segments),expected=reference(mesh,dense),actual=prepareSolidDisplay(mesh,brep,segments)
 expect(dense.indices.length/3).toBeLessThanOrEqual(4000)
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
 expect(mainSolidResult(expectation,{...result,closed:[true],workClosed:[false]})).toBe(true)
 for(const key of ['closed','workClosed'])for(const value of [[],[true,false],[1]])expect(mainSolidResult(expectation,{...result,[key]:value})).toBe(false)
 expect(mainSolidResult(expectation,{...result,map:[1]})).toBe(false)
 expect(mainSolidResult(expectation,{...result,normals:[[NaN,0,1]]})).toBe(false)
 expect(mainSolidResult(expectation,{...result,mesh:{...mesh,uv:new Float64Array([1])}})).toBe(false)
})

it('keeps all planar box normals perpendicular to their faces',()=>{
 const brep=createBrepBox([0,0,0],[2,3,4]),mesh=tessellateNurbsBrep(brep,1)
 const display=prepareSolidDisplay(mesh,brep)
 const geometric=stats(display.mesh).normals
 display.normals.forEach((normal,i)=>normal.forEach((value,axis)=>expect(value===geometric[i][axis]).toBe(true)))
})

it('keeps the edited flange caps flat without changing its document or picking map',()=>{
 const text=readFileSync('docs/qualification/cad-roadmap-2026-09-28/history20-parts-2026-09-30/browser/flange-reload.json','utf8')
 const doc=parseDirectDocument(text),body=doc.bodies[0],before=JSON.stringify(doc)
 const display=prepareSolidDisplay(body.mesh,body.brep),geometric=stats(display.mesh).normals
 let caps=0,walls=0
 geometric.forEach((flat,i)=>{
  const n=display.normals[i]
  if(Math.abs(flat[2])>.999){caps++;expect(n[0]).toBeCloseTo(0,12);expect(n[1]).toBeCloseTo(0,12);expect(n[2]).toBeCloseTo(flat[2],12)}
  else{walls++;expect(n[2]).toBeCloseTo(0,12)}
 })
 expect(caps).toBeGreaterThan(0);expect(walls).toBeGreaterThan(0)
 expect(display.map?.every(i=>Number.isInteger(i)&&i>=0&&i<body.mesh.indices.length/3)).toBe(true)
 expect(JSON.stringify(doc)).toBe(before)
})

it('preserves exact closed-shell membership for dense and working mixed meshes',()=>{
 const doc=parseDirectDocument(readFileSync('docs/qualification/cad-roadmap-2026-09-28/component-copy-2026-09-30/fixture.json','utf8'))
 const body=doc.bodies[0],before=JSON.stringify(doc),display=prepareSolidDisplay(body.mesh,body.brep)
 for(const [mesh,closed] of [[display.mesh,display.closed],[body.mesh,display.workClosed]] as const){
  expect(closed).toHaveLength(mesh.indices.length/3)
  for(let i=0;i<mesh.indices.length/3;i++){
   const x=mesh.positions[mesh.indices[i*3]*3]
   expect(closed![i]).toBe(x<20)
  }
 }
 expect(JSON.stringify(doc)).toBe(before)
 const changed=structuredClone(body.mesh);changed.positions[0]+=.123
 expect(prepareSolidDisplay(changed,body.brep).workClosed).toBeNull()
})

it('keeps the sphere working mesh and picking when requested refinement exceeds its budget',()=>{
 const brep=createBrepSphere(5),mesh=tessellateNurbsBrep(brep,3),before=structuredClone(mesh)
 const dense=tessellateNurbsBrep(brep,12)
 expect(dense.indices.length/3).toBeGreaterThan(4000)
 const actual=prepareSolidDisplay(mesh,brep,12)
 expect(Array.from(actual.mesh.positions)).toEqual(Array.from(mesh.positions))
 expect(Array.from(actual.mesh.indices)).toEqual(Array.from(mesh.indices))
 expect(actual.map).toBeNull()
 const expected=stats(mesh).normals
 actual.normals.forEach((normal,i)=>normal.forEach((x,k)=>expect(x).toBeCloseTo(expected[i][k],12)))
 expect(mesh).toEqual(before)
})
