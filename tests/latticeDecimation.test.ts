import {describe,it,expect} from 'vitest'
import {decimateLattice} from '../src/services/latticeDecimation'
import {inspectPolygonMesh,type PolygonMesh} from '../src/services/geometry/polygon'
/** Closed oriented box with subdivided top/bottom grids and quad strip walls. */
function gridBox(n:number,size=10,height=4):PolygonMesh{
 const d=size/n,row=n+1,positions:number[]=[],indices:number[]=[]
 for(const z of [height,0])for(let iy=0;iy<row;iy++)for(let ix=0;ix<row;ix++)positions.push(ix*d,iy*d,z)
 const t=(ix:number,iy:number)=>iy*row+ix,b=(ix:number,iy:number)=>row*row+iy*row+ix
 for(let iy=0;iy<n;iy++)for(let ix=0;ix<n;ix++){
  indices.push(t(ix,iy),t(ix+1,iy),t(ix+1,iy+1),t(ix,iy),t(ix+1,iy+1),t(ix,iy+1))
  indices.push(b(ix,iy),b(ix+1,iy+1),b(ix+1,iy),b(ix,iy),b(ix,iy+1),b(ix+1,iy+1))
 }
 for(let i=0;i<n;i++){
  indices.push(t(i,0),b(i,0),b(i+1,0),t(i,0),b(i+1,0),t(i+1,0))
  indices.push(t(i,n),b(i+1,n),b(i,n),t(i,n),t(i+1,n),b(i+1,n))
  indices.push(t(0,i),t(0,i+1),b(0,i+1),t(0,i),b(0,i+1),b(0,i))
  indices.push(t(n,i),b(n,i+1),t(n,i+1),t(n,i),b(n,i),b(n,i+1))
 }
 return {positions:Float64Array.from(positions),indices:Uint32Array.from(indices)}
}
describe('lattice decimation',()=>{
 const mesh=gridBox(28),target=1200,tolerance=1.5
 it('reduces a subdivided closed box toward the target and keeps it closed, oriented and valid',()=>{
  expect(inspectPolygonMesh(mesh).closed).toBe(true)
  const before=JSON.stringify(mesh)
  const out=decimateLattice(mesh,tolerance,target)
  expect(JSON.stringify(mesh)).toBe(before)
  const triangles=out.indices.length/3
  expect(triangles).toBeLessThan(mesh.indices.length/3)
  expect(triangles).toBeLessThanOrEqual(target)
  expect(out.indices.length%3).toBe(0)
  expect(out.positions.length%3).toBe(0)
  expect(out.positions.every(Number.isFinite)).toBe(true)
  expect(Math.min(...out.indices)).toBeGreaterThanOrEqual(0)
  expect(Math.max(...out.indices)).toBeLessThan(out.positions.length/3)
  const report=inspectPolygonMesh(out)
  expect(report.closed).toBe(true)
  expect(report.signedVolumeMm3).toBeGreaterThan(0)
 })
 it('is deterministic for a fixed input',()=>{
  const a=decimateLattice(mesh,tolerance,target),b=decimateLattice(mesh,tolerance,target)
  expect(a.positions).toEqual(b.positions)
  expect(a.indices).toEqual(b.indices)
  expect(a).toEqual(b)
 })
 it('applies the default target and lets the tolerance guard stop collapse early',()=>{
  const relaxed=decimateLattice(mesh,2)
  expect(relaxed.indices.length/3).toBeLessThanOrEqual(2600)
  expect(relaxed.indices.length/3).toBeLessThan(mesh.indices.length/3)
  const tight=decimateLattice(mesh,0.02,100)
  expect(tight.indices.length/3).toBeGreaterThan(100)
  expect(inspectPolygonMesh(tight).closed).toBe(true)
 })
 it('keeps an empty mesh empty',()=>{
  const out=decimateLattice({positions:new Float64Array(0),indices:new Uint32Array(0)},1);expect(Array.from(out.positions)).toEqual([]);expect(Array.from(out.indices)).toEqual([])
 })
 it('refuses malformed meshes and out-of-bounds admission',()=>{
  const m=gridBox(4),snapshot=JSON.stringify(m)
  expect(()=>decimateLattice({positions:[0,0,NaN,1,0,0,0,1,0],indices:[0,1,2]},1)).toThrow()
  expect(()=>decimateLattice({positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,3]},1)).toThrow()
  expect(()=>decimateLattice({positions:[0,0,0,1,0,0],indices:[0,1,2]},1)).toThrow()
  expect(()=>decimateLattice(m,-1)).toThrow()
  expect(()=>decimateLattice(m,NaN)).toThrow()
  expect(()=>decimateLattice(m,1,-4)).toThrow()
  expect(()=>decimateLattice(m,1,NaN)).toThrow()
  expect(JSON.stringify(m)).toBe(snapshot)
 })
})
