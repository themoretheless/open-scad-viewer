import {expect,it,vi} from 'vitest'
import {NativePickingCache} from '../src/services/nativePickingCache'
import * as meshAnalysis from '../src/services/geometry/meshAnalysis'
import {callGeometryRust} from '../src/services/geometry/kernel'

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
