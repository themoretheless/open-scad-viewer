import {expect,it,vi} from 'vitest'
import {SolidGeometryDisplayQueue} from '../src/services/solidGeometryDisplayQueue'

it('allows cancellation during scene key collection before sending worker jobs',async()=>{
 let clock=0
 const now=vi.spyOn(performance,'now').mockImplementation(()=>clock+=4)
 const run=vi.fn(async()=>[1]),cancel=vi.fn()
 const queue=new SolidGeometryDisplayQueue({run,cancel},item=>item.id,item=>item,()=>8)
 try{
  const pending=queue.prepare(Array.from({length:1000},(_,i)=>({id:String(i)})))
  expect(run).not.toHaveBeenCalled()
  queue.cancel()
  expect(await pending).toEqual({changed:false,errors:[]})
  expect(run).not.toHaveBeenCalled();expect(queue.size).toBe(0)
 }finally{now.mockRestore()}
})

it('allows cancellation during the final cache availability scan',async()=>{
 let clock=0,reads=0
 const now=vi.spyOn(performance,'now').mockImplementation(()=>clock)
 const run=vi.fn(async()=>[1])
 const queue=new SolidGeometryDisplayQueue({run,cancel:vi.fn()},item=>{if(++reads>5)clock+=4;return item.id},item=>item,()=>8)
 try{
  // One deduplicated job: three collection keys, one response check, then scan.
  const pending=queue.prepare([{id:'a'},{id:'a'},{id:'a'}])
  await Promise.resolve()
  expect(run).toHaveBeenCalledTimes(1)
  queue.cancel()
  expect(await pending).toEqual({changed:false,errors:[]})
  expect(queue.size).toBe(1)
 }finally{now.mockRestore()}
})
