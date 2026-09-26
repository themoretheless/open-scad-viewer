import {it,expect} from 'vitest'
import {selectedBrepSupport,solidTopology} from '../src/services/directSolidTools'
import {createBrepBox,tessellateNurbsBrep,createBrepSphere} from '../src/services/geometry/brep'
it('resolves each planar display face to its authored support at different display detail',()=>{
 const brep=createBrepBox([2,3,4],[7,11,13])
 for(const detail of [1,3,8]){
  const mesh=tessellateNurbsBrep(brep,detail),body={id:'b',name:'Box',brep,mesh}
  const selected=solidTopology(mesh).faces.map(face=>selectedBrepSupport(body,face.triangles))
  expect(new Set(selected).size).toBe(6)
  const faceIds=mesh.faceIds
  solidTopology(mesh).faces.forEach((face,i)=>face.triangles.forEach(t=>expect(selected[i]).toBe(faceIds[t])))
 }
})
it('rejects nonplanar, absent, degenerate and stale display selection without mutating the body',()=>{
 const brep=createBrepBox([0,0,0],[10,10,10]),mesh=tessellateNurbsBrep(brep,1),body={id:'b',name:'Box',brep,mesh},before=JSON.stringify(body)
 const faces=solidTopology(mesh).faces
 expect(()=>selectedBrepSupport(body,[])).toThrow()
 expect(()=>selectedBrepSupport(body,[mesh.indices.length/3])).toThrow('triangle')
 expect(()=>selectedBrepSupport(body,[faces[0].triangles[0],faces[1].triangles[0]])).toThrow('not planar')
 const stale=structuredClone(body);stale.mesh.positions=stale.mesh.positions.map(x=>x+100)
 expect(()=>selectedBrepSupport(stale,faces[0].triangles)).toThrow('one authored')
 const broken=structuredClone(body);broken.mesh.indices[1]=broken.mesh.indices[0]
 expect(()=>selectedBrepSupport(broken,[0])).toThrow()
 expect(JSON.stringify(body)).toBe(before)
})
it('refuses a curved tessellation triangle as an authored planar support',()=>{
 const brep=createBrepSphere(2),mesh=tessellateNurbsBrep(brep,4)
 expect(()=>selectedBrepSupport({id:'s',name:'Sphere',brep,mesh},[0])).toThrow('one authored')
})
it('refuses multiple authored supports on one plane rather than guessing a face',async()=>{
 const {booleanNurbsBrep}=await import('../src/services/geometry/brep')
 const brep=booleanNurbsBrep(createBrepBox([0,0,0],[2,2,2]),createBrepBox([5,0,0],[7,2,2]),'union'),mesh=tessellateNurbsBrep(brep,1)
 const face=solidTopology(mesh).faces.find(f=>f.normal[2]>.99)!
 expect(()=>selectedBrepSupport({id:'pair',name:'Pair',brep,mesh},face.triangles)).toThrow('one authored')
})
it('resolves straight box edges in either direction and refuses diagonals and nonedges',async()=>{
 const {selectedBrepStraightEdge}=await import('../src/services/directSolidTools')
 const brep=createBrepBox([0,0,0],[10,10,10]),mesh=tessellateNurbsBrep(brep,1),body={id:'b',name:'Box',brep,mesh}
 const edges=solidTopology(mesh).edges,found=new Set<number>()
 for(const e of edges){const id=selectedBrepStraightEdge(body,[e.a,e.b]);found.add(id);expect(selectedBrepStraightEdge(body,[e.b,e.a])).toBe(id)}
 expect(found.size).toBe(12)
 const edgePairs=new Set(edges.flatMap(e=>[`${e.a},${e.b}`,`${e.b},${e.a}`]))
 const diagonal=Array.from({length:mesh.indices.length/3},(_,i)=>mesh.indices.slice(i*3,i*3+3)).flatMap(t=>Array.from(t).map((a,i)=>[a,t[(i+1)%3]] as [number,number])).find(pair=>!edgePairs.has(pair.join(',')))!
 expect(()=>selectedBrepStraightEdge(body,diagonal)).toThrow('one straight authored')
 expect(()=>selectedBrepStraightEdge(body,[0,0])).toThrow('vertices')
 expect(()=>selectedBrepStraightEdge(body,[0,mesh.positions.length/3])).toThrow('vertices')
})
it('refuses a tessellated chord even when its endpoints match a curved authored edge',async()=>{
 const {selectedBrepStraightEdge}=await import('../src/services/directSolidTools')
 const {createBrepCylinder}=await import('../src/services/geometry/brep')
 const brep=createBrepCylinder(3,5),mesh=tessellateNurbsBrep(brep,1),body={id:'c',name:'Cylinder',brep,mesh}
 const curved=brep.edges.find(e=>e.curve.controlPoints.length>2)!
 const endpoints=curved.vertices.map(i=>brep.vertices[i].point)
 const ids=endpoints.map(p=>{for(let i=0;i<mesh.positions.length/3;i++)if(p.every((x,k)=>Math.abs(x-mesh.positions[i*3+k])<1e-8))return i;return -1}) as [number,number]
 expect(ids.every(i=>i>=0)).toBe(true)
 expect(()=>selectedBrepStraightEdge(body,ids)).toThrow()
 const chord={...body,mesh:{positions:[...endpoints[0],...endpoints[1],0,0,2],indices:[0,1,2]}}
 expect(()=>selectedBrepStraightEdge(chord,[0,1])).toThrow('one straight authored')
})
