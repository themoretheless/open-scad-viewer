import {describe,expect,it} from 'vitest'
import {callGeometryRust,importCadMesh,withCadMesh,GeometryKernelError} from '../src/services/geometry/kernel'
import Module from '../src/services/geometry/module'
const cube=()=>callGeometryRust<number>('cad',{action:'cube',size:[2,3,4],center:false})
const remove=(ids:number[])=>callGeometryRust('cad',{action:'delete',ids})
describe('binary CAD transport',()=>{
 it('matches the value transport reference bit-for-bit and keeps renderer arrays owned by JS',async()=>{
  const id=cube()
  try{
   const reference=callGeometryRust<{positions:number[];indices:number[];faceIds:number[]}>('cad',{action:'export_mesh',id})
   withCadMesh(id,mesh=>{expect(Array.from(mesh.positions)).toEqual(reference.positions);expect(Array.from(mesh.indices)).toEqual(reference.indices);expect(Array.from(mesh.faceIds)).toEqual(reference.faceIds)})
   const wasm=await Module(),solid=wasm.Manifold.cube([2,3,4]),mesh=solid.getMesh();solid.delete()
   // The worker may transfer these buffers after the Rust snapshot/source is freed.
   const transferred=structuredClone(mesh.vertProperties,{transfer:[mesh.vertProperties.buffer]})
   expect(transferred.length).toBeGreaterThan(0);expect(Array.from(transferred).every(Number.isFinite)).toBe(true)
  }finally{remove([id])}
 })
 it('releases the lease on exceptions and rejects reentrant WASM calls',()=>{
  const id=cube();try{
   expect(()=>withCadMesh(id,()=>{throw new Error('reader failed')})).toThrow('reader failed')
   expect(()=>withCadMesh(id,()=>cube())).toThrow('borrowed CAD mesh')
   expect(()=>withCadMesh(id,async()=>1)).toThrow('synchronous')
   expect(withCadMesh(id,m=>m.indices.length)).toBe(36)
  }finally{remove([id])}
 })
 it('reacquires memory after growth instead of retaining detached views',()=>{
  const id=cube(),held:number[]=[]
  try{
   const original=withCadMesh(id,m=>m.positions.buffer)
   for(let i=0;i<32&&withCadMesh(id,m=>m.positions.buffer)===original;i++)held.push(callGeometryRust<number>('cad',{action:'sphere',radius:2,segments:128}))
   expect(withCadMesh(id,m=>m.positions.buffer)).not.toBe(original)
   expect(withCadMesh(id,m=>Math.max(...m.positions))).toBe(4)
  }finally{remove([id,...held])}
 })
 it('imports packed vertex data without borrowing JS input storage',()=>{
  const id=cube();let imported:number|undefined
  try{
   const packed=withCadMesh(id,m=>({vertices:new Float32Array(m.positions),indices:new Uint32Array(m.indices)}))
   imported=importCadMesh(3,packed.vertices,packed.indices)
   packed.vertices.fill(99);packed.indices.fill(0)
   expect(withCadMesh(imported,m=>Math.max(...m.positions))).toBe(4)
   expect(()=>importCadMesh(2,packed.vertices,packed.indices)).toThrow(GeometryKernelError)
  }finally{remove(imported===undefined?[id]:[id,imported])}
  expect(()=>withCadMesh(id,()=>0)).toThrow(/deleted/)
 })
})
