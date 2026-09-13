import {it,expect} from 'vitest'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {extrudePolygonProfile,inspectPolygonMesh} from '../src/services/geometry/polygon'
import {pushPullFace,bevelSolidEdge,shellSolid,solidTopology} from '../src/services/directSolidTools'
const body=()=>({id:'mesh',name:'Stock',mesh:tessellateNurbsBrep(createBrepBox([0,0,0],[10,10,10]),1)})
const volume=(b:ReturnType<typeof body>)=>{const r=inspectPolygonMesh(b.mesh);expect(r.closed).toBe(true);expect(r.degenerateTriangles).toBe(0);return r.signedVolumeMm3}
it('pushes native mesh supports with independent prism volumes and no source mutation',()=>{
 const input=body(),before=JSON.stringify(input),faces=solidTopology(input.mesh).faces
 for(const axis of [0,1,2]){
  const index=faces.findIndex(f=>f.normal[axis]>.99)
  expect(volume(pushPullFace(input,index,3))).toBeCloseTo(1300,7)
  expect(volume(pushPullFace(input,index,-3))).toBeCloseTo(700,7)
  expect(()=>pushPullFace(input,index,-11)).toThrow()
 }
 expect(()=>pushPullFace(input,99,1)).toThrow()
 expect(()=>pushPullFace(input,0,Infinity)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})
it('constructs native mesh chamfers and faceted fillets while preserving adjacent supports',()=>{
 const input=body(),before=JSON.stringify(input),topology=solidTopology(input.mesh)
 for(const index of [0,5,11]){
  expect(volume(bevelSolidEdge(input,index,2,'chamfer'))).toBeCloseTo(980,6)
  const rounded=bevelSolidEdge(input,index,2,'fillet')
  expect(volume(rounded)).toBeCloseTo(1000-10*(4-Math.PI),0)
  const result=solidTopology(rounded.mesh)
  for(const face of topology.edges[index].faces){
   const f=topology.faces[face]
   expect(result.faces.some(g=>g.normal.reduce((s,x,k)=>s+x*f.normal[k],0)>1-1e-6&&Math.abs(g.offset-f.offset)<1e-5)).toBe(true)
  }
 }
 for(const kind of ['chamfer','fillet'] as const)expect(()=>bevelSolidEdge(input,0,50,kind)).toThrow()
 expect(()=>bevelSolidEdge(input,99,1,'chamfer')).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})
it('shells one and two opposite mesh openings and refuses a collapsed cavity atomically',()=>{
 const input=body(),before=JSON.stringify(input),faces=solidTopology(input.mesh).faces
 const top=faces.findIndex(f=>f.normal[2]>.99),bottom=faces.findIndex(f=>f.normal[2]<-.99)
 expect(volume(shellSolid(input,[top],1))).toBeCloseTo(424,6)
 expect(volume(shellSolid(input,[top,bottom],1))).toBeCloseTo(360,6)
 expect(volume(shellSolid(input,[top,top],1))).toBeCloseTo(424,6)
 for(const ids of [[],faces.map((_,i)=>i),[99]])expect(()=>shellSolid(input,ids,1)).toThrow()
 for(const amount of [0,6,Infinity])expect(()=>shellSolid(input,[top],amount)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 expect(volume(shellSolid(input,[top],1))).toBeCloseTo(424,6)
})
it('refuses concave planar edits without silently changing representation',()=>{
 const mesh=extrudePolygonProfile({outer:[[0,0],[20,0],[20,10],[10,10],[10,20],[0,20]]},[0,0,5])
 const input={id:'l',name:'L',mesh},before=JSON.stringify(input)
 expect(()=>pushPullFace(input,0,1)).toThrow('convex')
 expect(()=>shellSolid(input,[0],1)).toThrow('convex')
 expect(()=>bevelSolidEdge(input,0,1,'chamfer')).toThrow('convex')
 expect(JSON.stringify(input)).toBe(before)
})
