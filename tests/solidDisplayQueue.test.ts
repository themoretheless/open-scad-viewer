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

it('shares prepared keys with rendering and invalidates them on scene replacement or cancellation',async()=>{
 const a=body('a'),cache=new SolidDisplayCache(),result=display(a)
 cache.set(solidDisplayKey(a),result)
 const port={cancel:vi.fn(),run:vi.fn()},queue=new SolidDisplayQueue(port,cache)
 let reads=0;const mesh=a.mesh
 Object.defineProperty(a,'mesh',{get(){reads++;return mesh},configurable:true})
 expect(queue.get(a)).toBeUndefined()
 expect(await queue.prepare([a])).toBe(false);const preparedReads=reads
 for(let i=0;i<100;i++)expect(queue.get(a)).toBe(result)
 expect(reads).toBe(preparedReads);expect(port.run).not.toHaveBeenCalled()
 await queue.prepare([]);expect(queue.get(a)).toBeUndefined()
 await queue.prepare([a]);queue.cancel();expect(queue.get(a)).toBeUndefined()
 await queue.prepare([a]);cache.clear();expect(queue.get(a)).toBeUndefined()
})

it('allows cancellation during a large key scan without starting obsolete workers',async()=>{
 const bodies=Array.from({length:100},(_,i)=>body(String(i))),cache=new SolidDisplayCache()
 let release!:()=>void
 const port={cancel:vi.fn(),run:vi.fn()},yieldScan=vi.fn(()=>new Promise<void>(resolve=>{release=resolve}))
 const queue=new SolidDisplayQueue(port,cache,yieldScan),pending=queue.prepare(bodies)
 expect(yieldScan).toHaveBeenCalledTimes(1)
 expect(port.run).not.toHaveBeenCalled()
 queue.cancel();release();expect(await pending).toBe(false)
 expect(queue.get(bodies[0])).toBeUndefined();expect(cache.size).toBe(0)
})

it('publishes all cached keys after a large scan yields and never requests refinement',async()=>{
 const bodies=Array.from({length:100},(_,i)=>body(String(i))),cache=new SolidDisplayCache()
 for(const b of bodies)cache.set(solidDisplayKey(b),display(b))
 const port={cancel:vi.fn(),run:vi.fn()},yieldScan=vi.fn(async()=>{})
 const queue=new SolidDisplayQueue(port,cache,yieldScan)
 expect(await queue.prepare(bodies)).toBe(true)
 expect(yieldScan).toHaveBeenCalledTimes(3)
 expect(port.run).not.toHaveBeenCalled()
 for(const b of bodies)expect(queue.get(b)).toBe(cache.get(solidDisplayKey(b)))
})

it('an old suspended scan cannot erase keys published by its replacement scene',async()=>{
 const old=Array.from({length:100},(_,i)=>body('old'+i)),fresh=body('fresh'),cache=new SolidDisplayCache()
 cache.set(solidDisplayKey(fresh),display(fresh))
 let release!:()=>void
 const port={cancel:vi.fn(),run:vi.fn()},queue=new SolidDisplayQueue(port,cache,()=>new Promise<void>(resolve=>{release=resolve}))
 const pending=queue.prepare(old)
 expect(await queue.prepare([fresh])).toBe(false)
 const result=queue.get(fresh);expect(result).toBeDefined()
 release();expect(await pending).toBe(false)
 expect(queue.get(fresh)).toBe(result)
 expect(queue.get(old[0])).toBeUndefined();expect(port.run).not.toHaveBeenCalled()
})


it('publishes cached keys atomically after an interruptible scan',async()=>{
 const bodies=Array.from({length:33},(_,i)=>body(String(i))),cache=new SolidDisplayCache()
 for(const b of bodies)cache.set(solidDisplayKey(b),display(b))
 let release!:()=>void
 const queue=new SolidDisplayQueue({cancel:vi.fn(),run:vi.fn()},cache,()=>new Promise<void>(resolve=>{release=resolve}))
 const pending=queue.prepare(bodies)
 for(const b of bodies)expect(queue.get(b)).toBeUndefined()
 release();expect(await pending).toBe(true)
 for(const b of bodies)expect(queue.get(b)).toBeDefined()
})

it('the default event-loop yield permits cancelling before any refinement request',async()=>{
 const bodies=Array.from({length:33},(_,i)=>body(String(i))),port={cancel:vi.fn(),run:vi.fn()},queue=new SolidDisplayQueue(port,new SolidDisplayCache())
 const pending=queue.prepare(bodies)
 // prepare has yielded to its MessageChannel rather than running all jobs.
 expect(port.run).not.toHaveBeenCalled()
 queue.cancel()
 expect(await pending).toBe(false)
 expect(port.run).not.toHaveBeenCalled()
})

it('yields for expensive keys before reaching the 32-body batch limit',async()=>{
 let clock=0,release!:()=>void
 const now=vi.spyOn(performance,'now').mockImplementation(()=>clock+=5)
 const port={cancel:vi.fn(),run:vi.fn()},yieldScan=vi.fn(()=>new Promise<void>(resolve=>release=resolve))
 const queue=new SolidDisplayQueue(port,new SolidDisplayCache(),yieldScan)
 try{
  const pending=queue.prepare([body('a'),body('b'),body('c')])
  expect(yieldScan).toHaveBeenCalledTimes(1);expect(port.run).not.toHaveBeenCalled()
  queue.cancel();release();expect(await pending).toBe(false)
  expect(port.run).not.toHaveBeenCalled();expect(queue.get(body('a'))).toBeUndefined()
 }finally{now.mockRestore()}
})
