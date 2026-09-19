import {it,expect} from 'vitest'
import {solidTopology} from '../src/services/directSolidTools'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
it('groups a box into six oriented supports and twelve physical edges with stable triangle membership',()=>{
 const mesh=tessellateNurbsBrep(createBrepBox([0,0,0],[2,3,4]),3)
 const topology=solidTopology(mesh)
 expect(topology.faces).toHaveLength(6);expect(topology.edges).toHaveLength(12)
 const triangles=topology.faces.flatMap(f=>f.triangles).sort((a,b)=>a-b)
 expect(triangles).toEqual(Array.from({length:mesh.indices.length/3},(_,i)=>i))
 topology.faces.forEach(f=>{
  expect(Math.hypot(...f.normal)).toBeCloseTo(1,12)
  expect(new Set(f.vertices).size).toBe(f.vertices.length)
  f.vertices.forEach(i=>expect(f.normal.reduce((s,n,k)=>s+n*mesh.positions[i*3+k],0)).toBeCloseTo(f.offset,10))
  expect(f.normal.reduce((s,n,k)=>s+n*f.center[k],0)).toBeCloseTo(f.offset,10)
 })
 topology.edges.forEach(e=>{expect(e.faces[0]).not.toBe(e.faces[1]);e.faces.forEach(f=>expect(topology.faces[f]).toBeDefined())})
})
it('joins duplicated display seams without changing the source mesh',()=>{
 const original=tessellateNurbsBrep(createBrepBox([0,0,0],[2,3,4]),1)
 const mesh={positions:original.indices.flatMap(i=>original.positions.slice(i*3,i*3+3)),indices:original.indices.map((_,i)=>i)}
 const before=JSON.stringify(mesh),topology=solidTopology(mesh)
 expect(topology.faces).toHaveLength(6);expect(topology.edges).toHaveLength(12)
 expect(JSON.stringify(mesh)).toBe(before)
})
it('omits degenerate triangles without creating invalid face references',()=>{
 const mesh={positions:[0,0,0,1,0,0,0,1,0,0,0,1],indices:[0,1,2,0,3,1,0,0,1]}
 const result=solidTopology(mesh)
 expect(result.faces.map(f=>f.triangles)).toEqual([[0],[1]])
 expect(result.edges).toHaveLength(1)
 expect(result.edges[0].faces).toEqual([0,1])
 expect(()=>solidTopology({positions:[0,0],indices:[]})).toThrow()
 expect(()=>solidTopology({positions:[0,0,0],indices:[0,1,2]})).toThrow()
 expect(solidTopology({positions:[],indices:[]})).toEqual({faces:[],edges:[]})
})
it('keeps plane search linear on a body where every triangle is its own plane',()=>{
 // 6400 parallel planes used to cost 6400^2/2 comparisons and trip the 20M
 // budget; the grid lookup only compares neighbouring cells. A curved gear
 // in the Solid workspace is exactly this shape.
 const positions:number[]=[],indices:number[]=[]
 for(let i=0;i<6400;i++){positions.push(0,0,i,1,0,i,0,1,i);indices.push(i*3,i*3+1,i*3+2)}
 const started=performance.now()
 expect(solidTopology({positions,indices}).faces).toHaveLength(6400)
 expect(performance.now()-started).toBeLessThan(2000)
 // Planes closer than the match tolerance still merge through the grid.
 expect(solidTopology({positions:[0,0,0,1,0,0,0,1,0,0,0,5e-7,1,0,5e-7,0,1,5e-7],indices:[0,1,2,3,4,5]}).faces).toHaveLength(1)
})
it('preserves legacy seam grouping at exact decimal half ties',()=>{
 const x=1/256,y=0.0039063
 expect(x.toFixed(7)).toBe(y.toFixed(7))
 const mesh={positions:[x,0,0,x,1,0,x+1,0,0,y,0,0,y,1,0,y,0,1],indices:[0,1,2,4,3,5]}
 expect(solidTopology(mesh).edges).toHaveLength(1)
})
it('builds oriented orthonormal workplanes on tilted faces',async()=>{
 const {facePlane,transformBodies}=await import('../src/services/directSolidTools')
 const brep=createBrepBox([0,0,0],[2,3,4]),mesh=tessellateNurbsBrep(brep,1)
 const body=transformBodies([{id:'b',name:'Box',brep,mesh}],[7,-3,5],[1,2,3],37,1)[0]
 for(const face of solidTopology(body.mesh).faces){
  const plane=facePlane(body,face),dot=(a:number[],b:number[])=>a.reduce((s,x,k)=>s+x*b[k],0)
  expect(dot(plane.u,plane.u)).toBeCloseTo(1,12);expect(dot(plane.v,plane.v)).toBeCloseTo(1,12);expect(dot(plane.u,plane.v)).toBeCloseTo(0,12)
  const cross=[plane.u[1]*plane.v[2]-plane.u[2]*plane.v[1],plane.u[2]*plane.v[0]-plane.u[0]*plane.v[2],plane.u[0]*plane.v[1]-plane.u[1]*plane.v[0]]
  expect(dot(cross,face.normal)).toBeCloseTo(1,12)
  face.vertices.forEach(i=>{const p=body.mesh.positions.slice(i*3,i*3+3),d=p.map((x,k)=>x-plane.origin[k]),x=dot(d,plane.u),y=dot(d,plane.v);p.forEach((a,k)=>expect(plane.origin[k]+x*plane.u[k]+y*plane.v[k]).toBeCloseTo(a,10))})
 }
})
it('refuses malformed or inconsistent workplane selections',async()=>{
 const {facePlane}=await import('../src/services/directSolidTools')
 const mesh={positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},body={id:'b',name:'Triangle',mesh},face=solidTopology(mesh).faces[0]
 expect(()=>facePlane(body,{...face,vertices:[]})).toThrow('vertices')
 expect(()=>facePlane(body,{...face,vertices:[0,3]})).toThrow('vertices')
 expect(()=>facePlane(body,{...face,normal:[0,0,0]})).toThrow('Degenerate')
 expect(()=>facePlane(body,{...face,normal:[1,0,0]})).toThrow('tangent')
 expect(()=>facePlane(body,{...face,vertices:[0,0]})).toThrow('Degenerate')
})
