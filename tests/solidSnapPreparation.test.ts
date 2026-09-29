import {expect,it,vi} from 'vitest'
import {SolidSnapPreparation} from '../src/services/solidSnapPreparation'
import type {DirectBody} from '../src/services/directModeling'
import type {SnapGeometry} from '../src/services/modelingSnaps'
const body=(id:string):DirectBody=>({id,name:id,mesh:{positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])}})
const geometry:SnapGeometry={points:[{point:[0,0,0],kind:'vertex'}],segments:[]}
it('serializes preparation, reuses immutable bodies and releases departed scene entries',async()=>{
 const a=body('a'),b=body('b');b.mesh.positions[0]=3;const run=vi.fn(async()=>structuredClone(geometry)),queue=new SolidSnapPreparation({run,cancel:vi.fn()})
 expect((await queue.prepare([a,a,b])).changed).toBe(true);expect(run).toHaveBeenCalledTimes(2);expect(queue.size).toBe(2)
 await queue.prepare([b]);expect(run).toHaveBeenCalledTimes(2);expect(queue.size).toBe(1);expect(queue.get(a)).toBeUndefined()
 b.mesh.positions[0]=2;await queue.prepare([b]);expect(run).toHaveBeenCalledTimes(3)
 queue.clear();expect(queue.size).toBe(0)
})
it('drops cancelled and mutated results and isolates per-body failures',async()=>{
 const pending:{resolve:(g:SnapGeometry)=>void;reject:(e:Error)=>void}[]=[]
 const queue=new SolidSnapPreparation({run:()=>new Promise((resolve,reject)=>pending.push({resolve,reject})),cancel:vi.fn()}),a=body('a'),b=body('b');b.mesh.positions[0]=3
 const old=queue.prepare([a]);queue.cancel();pending[0].resolve(geometry);await old;expect(queue.get(a)).toBeUndefined()
 const changed=queue.prepare([a]);a.mesh.positions[0]=2;pending[1].resolve(geometry)
 expect((await changed).errors).toHaveLength(1);expect(queue.get(a)).toBeUndefined()
 const retry=queue.prepare([a,b]);expect(pending).toHaveLength(3)
 pending[2].reject(Error('bad input'));await Promise.resolve();expect(pending).toHaveLength(4)
 pending[3].resolve(geometry);expect((await retry).errors).toEqual([{id:'a',message:'bad input'}]);expect(queue.get(b)).toEqual(geometry)
})
