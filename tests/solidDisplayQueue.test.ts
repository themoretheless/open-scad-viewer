import {expect,it,vi} from 'vitest'
import {SolidDisplayQueue} from '../src/services/solidDisplayQueue'
import {SolidDisplayCache,solidDisplayKey} from '../src/services/solidDisplayCache'
import type {DirectBody} from '../src/services/directModeling'
const body=(id:string)=>({id,name:id,mesh:{positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])},brep:{tag:id}} as unknown as DirectBody)
const display=(b:DirectBody)=>({mesh:b.mesh,map:[0],normals:[[0,0,1]]})
it('runs one request at a time, reuses exact cached results and continues after failure',async()=>{
 const a=body('a'),b=body('b'),cache=new SolidDisplayCache(),requests:Array<{job:any;resolve:(v:any)=>void;reject:(e:Error)=>void}>=[]
 const port={cancel:vi.fn(),run:vi.fn(job=>new Promise<any>((resolve,reject)=>requests.push({job,resolve,reject})))},queue=new SolidDisplayQueue(port,cache)
 const pending=queue.prepare([a,b]);expect(requests).toHaveLength(1)
 requests[0].reject(Error('refinement failed'));await Promise.resolve();expect(requests).toHaveLength(2)
 requests[1].resolve(display(b));expect(await pending).toBe(true)
 expect(cache.get(solidDisplayKey(a))).toBeUndefined();expect(cache.get(solidDisplayKey(b))).toBeDefined()
 expect(await queue.prepare([b])).toBe(false);expect(requests).toHaveLength(2)
})
it('drops old responses after cancellation, a newer scene, or in-place geometry changes',async()=>{
 const a=body('a'),b=body('b'),cache=new SolidDisplayCache(),resolve:Array<(v:any)=>void>=[]
 const queue=new SolidDisplayQueue({cancel:vi.fn(),run:()=>new Promise<any>(done=>resolve.push(done))},cache)
 const old=queue.prepare([a]);const current=queue.prepare([b]);resolve[0](display(a));expect(await old).toBe(false)
 expect(cache.get(solidDisplayKey(a))).toBeUndefined()
 b.mesh.positions[0]=2;resolve[1](display(b));expect(await current).toBe(false);expect(cache.size).toBe(0)
 const cancelled=queue.prepare([a]);queue.cancel();resolve[2](display(a));expect(await cancelled).toBe(false);expect(cache.size).toBe(0)
})
