import {expect,it} from 'vitest'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {createPickingSnapshotInKernel} from '../src/services/geometry/meshAnalysis'
import type {MeshBvhHit} from '../src/services/meshBvh'
import reference from '../crates/polygon-core/tests/fixtures/bvh-query-parity-v1.json'

it('matches the frozen independent ray-query corpus through the shipped WASM boundary',()=>{
 const handle=createPickingSnapshotInKernel(new Float32Array(reference.vertices),new Uint32Array(reference.indices),3,2)
 let hits=0
 try{
  for(const [index,test] of reference.cases.entries()){
   const hit=callGeometryRust<MeshBvhHit|null>('mesh_picking',{
    action:'query',handle,origin:test.origin,direction:test.direction,
    minT:test.minT,maxT:test.maxT,excludedTriangles:test.excluded,localFromWorld:test.matrix,
   })
   if(test.expected===null){expect(hit,`case ${index}`).toBeNull();continue}
   expect(hit,`case ${index}`).not.toBeNull()
   const expected=test.expected
   expect(hit!.triangleIndex).toBe(expected.triangleIndex)
   expect(hit!.triangleVertexIndices).toEqual(expected.triangleVertexIndices)
   expect(hit!.frontFace).toBe(expected.frontFace)
   expect(Math.abs(hit!.t-expected.t)).toBeLessThanOrEqual(1e-12)
   for(const field of ['barycentric','localPoint','worldPoint','localNormal','worldNormal'] as const){
    for(let axis=0;axis<3;axis++)expect(Math.abs(hit![field][axis]-expected[field][axis]),`case ${index}: ${field}[${axis}]`).toBeLessThanOrEqual(1e-12)
   }
   hits++
  }
  expect(reference.cases).toHaveLength(256)
  expect(hits).toBe(125)
 }finally{callGeometryRust('mesh_picking',{action:'dispose',handle})}
})

it('uploads raw subarray views and retains an independent native snapshot',()=>{
 const storage=new Float32Array([99,99,99,-1,-1,0,1,-1,0,0,1,0,99])
 const vertices=storage.subarray(3,12)
 const indexStorage=new Uint32Array([99,0,1,2,99])
 const handle=createPickingSnapshotInKernel(vertices,indexStorage.subarray(1,4),3,1)
 try{
  vertices.fill(100)
  indexStorage.fill(99)
  expect(callGeometryRust('mesh_picking',{action:'query',handle,origin:[0,0,2],direction:[0,0,-1],excludedTriangles:[]})).toMatchObject({triangleIndex:0,t:2})
 }finally{callGeometryRust('mesh_picking',{action:'dispose',handle})}
 const empty=createPickingSnapshotInKernel(new Float32Array(),new Uint32Array(),3,1)
 try{expect(callGeometryRust('mesh_picking',{action:'query',handle:empty,origin:[0,0,2],direction:[0,0,-1],excludedTriangles:[]})).toBeNull()}
 finally{callGeometryRust('mesh_picking',{action:'dispose',handle:empty})}
})

it('uploads a picking snapshot once and queries it until explicit disposal',()=>{
 const created=callGeometryRust<{handle:string}>('mesh_picking',{action:'create',vertices:[-1,-1,0,1,-1,0,0,1,0],indices:[0,1,2],stride:3,leafSize:1})
 const query={action:'query',handle:created.handle,origin:[0,0,2],direction:[0,0,-1],excludedTriangles:[]}
 try{
  for(let i=0;i<20;i++)expect(callGeometryRust('mesh_picking',query)).toMatchObject({triangleIndex:0,t:2,barycentric:[0.25,0.25,0.5]})
  expect(callGeometryRust('mesh_picking',{...query,excludedTriangles:[0]})).toBeNull()
  expect(()=>callGeometryRust('mesh_picking',{...query,maxT:'invalid'})).toThrow()
  expect(callGeometryRust('mesh_picking',query)).toMatchObject({t:2})
 }finally{callGeometryRust('mesh_picking',{action:'dispose',handle:created.handle})}
 expect(()=>callGeometryRust('mesh_picking',query)).toThrow()
 const next=callGeometryRust<{handle:string}>('mesh_picking',{action:'create',vertices:[],indices:[],stride:3,leafSize:1})
 try{expect(BigInt(next.handle)).toBeGreaterThan(BigInt(created.handle))}
 finally{callGeometryRust('mesh_picking',{action:'dispose',handle:next.handle})}
})
