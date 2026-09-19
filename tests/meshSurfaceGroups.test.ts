import {expect,it} from 'vitest'
import {selectionSurfaceIds,withSelectionSurfaces} from '../src/services/meshSurfaceGroups'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {buildFaceOverlayGeometry,buildFaceTriangleIndex} from '../src/services/meshSelectionOverlay'
import {identity} from '../src/services/math3d'
it('selects a complete cylinder wall but keeps the two caps separate',async()=>{
 const {meshes}=await parseOpenSCAD('cylinder(r=10,h=5,$fn=48);')
 const mesh=withSelectionSurfaces(meshes[0]);expect(new Set(mesh.faceIds).size).toBe(3)
 const t=Array.from({length:mesh.indices.length/3},(_,t)=>t).find(t=>{const a=mesh.indices[t*3],b=mesh.indices[t*3+1],c=mesh.indices[t*3+2];return new Set([a,b,c].map(i=>mesh.vertices[i*6+2])).size>1})!
 const overlay=buildFaceOverlayGeometry(mesh.vertices,mesh.indices,mesh.faceIds,identity(),t,mesh.faceIds[t],20000,buildFaceTriangleIndex(mesh.faceIds,mesh.indices.length/3))
 expect(overlay.triangles.length/9).toBe(96)
 expect(overlay.boundaryLines.length/6).toBe(96)
 expect(withSelectionSurfaces({...meshes[0],faceIdsAuthoritative:true} as typeof mesh).faceIds).toBe(meshes[0].faceIds)
})
it('does not cross cube creases or join disconnected coplanar triangles',async()=>{
 const {meshes}=await parseOpenSCAD('cube(5);');expect(new Set(withSelectionSurfaces(meshes[0]).faceIds).size).toBe(6)
 const vertices=new Float32Array([0,0,0,0,0,1,1,0,0,0,0,1,0,1,0,0,0,1,3,0,0,0,0,1,4,0,0,0,0,1,3,1,0,0,0,1])
 expect(Array.from(selectionSurfaceIds(vertices,new Uint32Array([0,1,2,3,4,5])))).toEqual([0,1])
})
it('groups the original spinner bore around the complete ring',async()=>{
 const {readFileSync}=await import('node:fs')
 const {meshes}=await parseOpenSCAD(readFileSync('examples/modelgraph-text/planetary-spinner.mg','utf8'))
 let found=false
 for(const raw of meshes){const mesh=withSelectionSurfaces(raw);for(let t=0;t<mesh.indices.length/3;t++){
  const points=Array.from(mesh.indices.subarray(t*3,t*3+3),i=>Array.from(mesh.vertices.subarray(i*6,i*6+3)))
  if(!points.every(p=>Math.abs(Math.hypot(p[0],p[1])-22)<0.01)||new Set(points.map(p=>p[2])).size<2)continue
  const id=mesh.faceIds[t],xs:number[]=[],ys:number[]=[]
  for(let i=0;i<mesh.indices.length/3;i++)if(mesh.faceIds[i]===id)for(const v of mesh.indices.subarray(i*3,i*3+3)){xs.push(mesh.vertices[v*6]);ys.push(mesh.vertices[v*6+1])}
  expect(Math.min(...xs)).toBeLessThan(-21);expect(Math.max(...xs)).toBeGreaterThan(21)
  expect(Math.min(...ys)).toBeLessThan(-21);expect(Math.max(...ys)).toBeGreaterThan(21)
  found=true;break
 }if(found)break}
 expect(found).toBe(true)
},15000)
it('does not merge across a non-manifold shared edge',()=>{
 const p=[[0,0,0],[1,0,0],[0,1,0],[0,-1,0],[1,1,0]]
 const vertices=new Float32Array(p.flatMap(v=>[...v,0,0,1]))
 const ids=selectionSurfaceIds(vertices,new Uint32Array([0,1,2,1,0,3,0,1,4]))
 expect(new Set(ids).size).toBe(3)
})
it('does not recompute transported kernel surface ids',()=>{
 const faceIds=new Uint32Array([7,7])
 const mesh={
  vertices:new Float32Array(4*6),
  indices:new Uint32Array([0,1,2,0,2,3]),
  bvh:{version:1,vertexStride:6,leafSize:1,nodeCount:0,bounds:new Float32Array(),nodes:new Uint32Array(),triangles:new Uint32Array()},
  edgeIndices:new Uint32Array(),
  color:[1,1,1,1] as [number,number,number,number],
  transform:new Float32Array(16),
  faceIds,
  faceIdsSurfaceGroups:true,
  provenance:[],
  topology:{boundary:0,crease:0,nonManifold:0,degenerate:0},
 }
 expect(withSelectionSurfaces(mesh).faceIds).toBe(faceIds)
})
it('does not trust legacy zero-filled non-authoritative ids by length alone',()=>{
 const vertices=new Float32Array([0,0,0,0,0,1,1,0,0,0,0,1,0,1,0,0,0,1,3,0,0,0,0,1,4,0,0,0,0,1,3,1,0,0,0,1])
 const mesh={
  vertices,
  indices:new Uint32Array([0,1,2,3,4,5]),
  bvh:{version:1,vertexStride:6,leafSize:1,nodeCount:0,bounds:new Float32Array(),nodes:new Uint32Array(),triangles:new Uint32Array()},
  edgeIndices:new Uint32Array(),
  color:[1,1,1,1] as [number,number,number,number],
  transform:new Float32Array(16),
  faceIds:new Uint32Array(2),
  provenance:[],
  topology:{boundary:0,crease:0,nonManifold:0,degenerate:0},
 }
 expect(Array.from(withSelectionSurfaces(mesh).faceIds)).toEqual([0,1])
})
