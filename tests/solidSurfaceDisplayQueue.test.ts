import {expect,it,vi} from 'vitest'
import {SolidSurfaceDisplayQueue} from '../src/services/solidSurfaceDisplayQueue'
import {createSolidNurbsSurface,tessellateSolidNurbsSurface} from '../src/services/solidNurbs'
it('deduplicates geometry, retains bounded results and reports unavailable surfaces',async()=>{
 const a=createSolidNurbsSurface('a'),b=structuredClone(a);b.id='b';b.surface.controlPoints[0][0][2]+=1
 const run=vi.fn(async({item})=>tessellateSolidNurbsSurface(item)),queue=new SolidSurfaceDisplayQueue({run,cancel:vi.fn()},1)
 expect((await queue.prepare([a,{...a,id:'same'}])).errors).toEqual([]);expect(run).toHaveBeenCalledTimes(1)
 expect((await queue.prepare([a])).changed).toBe(false)
 const result=await queue.prepare([a,b]);expect(queue.size).toBe(1);expect(queue.get(a)).toBeUndefined();expect(queue.get(b)).toBeDefined()
 expect(result.errors.map(e=>e.id)).toEqual(['a'])
 const tiny=new SolidSurfaceDisplayQueue({run,cancel:vi.fn()},2,1)
 expect((await tiny.prepare([a])).errors[0].message).toContain('budget');expect(tiny.retainedWeight).toBe(0)
})
it('drops cancelled and mutated responses and continues other surfaces after a failure',async()=>{
 const a=createSolidNurbsSurface('a'),b=structuredClone(a);b.id='b';b.segmentsU=4
 const requests:Array<{job:any;resolve:(v:any)=>void;reject:(e:Error)=>void}>=[]
 const queue=new SolidSurfaceDisplayQueue({run:job=>new Promise((resolve,reject)=>requests.push({job,resolve,reject})),cancel:vi.fn()})
 const old=queue.prepare([a]);queue.cancel();requests[0].resolve(tessellateSolidNurbsSurface(a));expect((await old).changed).toBe(false);expect(queue.size).toBe(0)
 const pending=queue.prepare([a,b]);requests[1].reject(Error('bad surface'));await Promise.resolve();expect(requests).toHaveLength(3)
 const mesh=tessellateSolidNurbsSurface(b);b.surface.controlPoints[0][0][2]+=2;requests[2].resolve(mesh)
 const result=await pending;expect(result.changed).toBe(false);expect(queue.size).toBe(0);expect(result.errors[0]).toMatchObject({id:'a',message:'bad surface'})
})
