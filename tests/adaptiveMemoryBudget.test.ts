import {expect,it} from 'vitest'
import {adaptiveCacheBytes} from '../src/services/adaptiveMemoryBudget'
import {SolidDisplayCache,type DisplayMesh} from '../src/services/solidDisplayCache'
it('scales capacity with bounded growth and a conservative fallback',()=>{
 expect(adaptiveCacheBytes(64_000_000,{})).toBe(64_000_000)
 expect(adaptiveCacheBytes(64_000_000,{deviceGiB:1})).toBe(32_000_000)
 expect(adaptiveCacheBytes(64_000_000,{deviceGiB:8})).toBe(256_000_000)
 expect(adaptiveCacheBytes(64_000_000,{deviceGiB:1024})).toBe(256_000_000)
 expect(adaptiveCacheBytes(64_000_000,{deviceGiB:NaN})).toBe(64_000_000)
})
it('shares remaining heap headroom proportionally between caches',()=>{
 const hints={heapLimit:1_000_000_000,heapUsed:700_000_000,deviceGiB:8}
 expect(adaptiveCacheBytes(64_000_000,hints)+adaptiveCacheBytes(8_000_000,hints)).toBeLessThanOrEqual(12_500_000)
 expect(adaptiveCacheBytes(64_000_000,{heapLimit:100,heapUsed:90})).toBe(1)
})
it('retains recently used geometry and shrinks on access under pressure',()=>{
 let budget=2000
 const cache=new SolidDisplayCache(2,2000,()=>budget)
 const value=():DisplayMesh=>({mesh:{positions:new Float64Array(9),indices:new Uint32Array([0,1,2])},normals:[],map:null})
 cache.set('a',value());cache.set('b',value());cache.get('a');cache.set('c',value())
 expect(cache.get('b')).toBeUndefined();expect(cache.get('a')).toBeDefined()
 budget=1;expect(cache.get('a')).toBeUndefined();expect(cache.retainedWeight).toBe(0)
 budget=2000;cache.set('d',value());expect(cache.get('d')).toBeDefined()
})
