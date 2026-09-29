import {expect,it} from 'vitest'
import {SolidDisplayCache,solidDisplayKey,type DisplayMesh} from '../src/services/solidDisplayCache'
const value=():DisplayMesh=>({mesh:{positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])},map:[0],normals:[[0,0,1]]})
it('retains a complete 1000-instance scan and keys retained surfaces independently of display mesh',()=>{
 const cache=new SolidDisplayCache(),keys=[]
 for(let i=0;i<1001;i++){const mesh=value();mesh.mesh.positions[0]=i;const key=solidDisplayKey({mesh:mesh.mesh,brep:{controlPoint:i}});keys.push(key);cache.set(key,mesh)}
 for(const key of keys)expect(cache.get(key)).toBeDefined()
 const body={mesh:value().mesh,brep:{controlPoint:1}}
 expect(solidDisplayKey(structuredClone(body))).toBe(solidDisplayKey(body))
 expect(solidDisplayKey({...body,brep:{controlPoint:2}})).not.toBe(solidDisplayKey(body))
})
it('bounds entry count and accounting weight including reserved flat GPU buffers',()=>{
 const cache=new SolidDisplayCache(2,2000)
 cache.set('a',value());cache.set('b',value());cache.set('c',value())
 expect(cache.get('a')).toBeUndefined();expect(cache.size).toBe(2);expect(cache.retainedWeight).toBeLessThanOrEqual(2000)
 const weight=cache.retainedWeight;cache.set('c',value());expect(cache.retainedWeight).toBe(weight)
 const huge=value();huge.mesh.positions=new Float64Array(1000);cache.set('c',huge)
 expect(cache.get('c')).toBeUndefined();expect(cache.size).toBe(1)
 cache.clear();expect(cache.size).toBe(0);expect(cache.retainedWeight).toBe(0)
 const bounded=new SolidDisplayCache(1200,900);bounded.set('a',value());bounded.set('b',value())
 expect(bounded.size).toBe(1);expect(bounded.get('a')).toBeUndefined()
})
