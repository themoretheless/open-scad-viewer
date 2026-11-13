import {expect,it,vi} from 'vitest'
import {NativePickingCache} from '../src/render/nativePickingCache'
import * as meshAnalysis from '../src/kernel/geometry/meshAnalysis'
import {callGeometryRust} from '../src/kernel/geometry/kernel'

it('reuses uploads, evicts bounded cache entries and disposes every owned handle',()=>{
 const upload=vi.spyOn(meshAnalysis,'createPickingSnapshotInKernel')
 const cache=new NativePickingCache(1),a={},b={}
 const vertices=new Float32Array([-1,-1,0,1,-1,0,0,1,0]),indices=new Uint32Array([0,1,2])
 const ray={origin:[0,0,2],direction:[0,0,-1]} as const
 const query=(key:object)=>cache.query(key,vertices,indices,3,1,ray)
 const probe=(handle:string)=>callGeometryRust('mesh_picking',{action:'query',handle,...ray,excludedTriangles:[]})
 try{
  expect(query(a)?.t).toBe(2);expect(query(a)?.t).toBe(2)
  expect(upload).toHaveBeenCalledTimes(1)
  const first=upload.mock.results[0].value
  expect(query(b)?.t).toBe(2)
  expect(()=>probe(first)).toThrow()
  const second=upload.mock.results[1].value
  cache.release(a);cache.clear();cache.clear()
  expect(()=>probe(second)).toThrow()
 }finally{cache.clear();upload.mockRestore()}
})

it('answers box, radius and nearest queries from one cached snapshot',()=>{
 const upload=vi.spyOn(meshAnalysis,'createPickingSnapshotInKernel')
 const cache=new NativePickingCache(2),key={}
 // Two unit triangles: one at x in [0,1], one shifted to x in [10,11].
 const vertices=new Float32Array([0,0,0,1,0,0,0,1,0, 10,0,0,11,0,0,10,1,0]),indices=new Uint32Array([0,1,2,3,4,5])
 try{
  expect(cache.overlap(key,vertices,indices,3,1,[-1,-1,-1],[2,2,1])).toEqual([0])
  expect(cache.overlap(key,vertices,indices,3,1,[-1,-1,-1],[20,2,1]).sort()).toEqual([0,1])
  const near=cache.withinRadius(key,vertices,indices,3,1,[0.2,0.2,1],1.5)
  expect(near.map(n=>n.triangleIndex)).toEqual([0])
  expect(near[0].distance).toBeCloseTo(1,6)
  const nearest=cache.nearest(key,vertices,indices,3,1,[9,0,0],2)
  expect(nearest.map(n=>n.triangleIndex)).toEqual([1,0])
  expect(upload).toHaveBeenCalledTimes(1)
  expect(()=>cache.nearest(key,vertices,indices,3,1,[0,0,0],0)).toThrow(RangeError)
  expect(()=>cache.overlap(key,vertices,indices,3,1,[0,0],[1,1,1])).toThrow(RangeError)
 }finally{cache.clear();upload.mockRestore()}
})
