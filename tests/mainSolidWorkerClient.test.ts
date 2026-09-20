import {afterEach, expect, it, vi} from 'vitest'
import {MainSolidWorkerClient, type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest} from '../src/services/mainSolidProtocol'
import type {TrussModel, TrussResponse} from '../src/services/trussAnalysis'

class Port implements MainSolidPort {
  onmessage:MainSolidPort['onmessage']=null
  onerror:MainSolidPort['onerror']=null
  onmessageerror:MainSolidPort['onmessageerror']=null
  terminate=vi.fn()
  postMessage=vi.fn<(message:MainSolidRequest)=>void>()
  reply(value:object){const {version,id,job}=this.postMessage.mock.lastCall![0];this.onmessage?.({data:{version,id,kind:job.kind,...value}} as MessageEvent)}
}
const clients:MainSolidWorkerClient[]=[]
function setup(){const ports:Port[]=[];const client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client);return {client,ports}}
const model=():TrussModel=>({nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],youngMpa:2000,areaMm2:2}],restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]})
const response=():TrussResponse=>({displacementsMm:[[0,0,0],[0.25,0,0]],reactionsN:[[-100,0,0],[0,0,0]],axialForcesN:[100],axialStressesMpa:[50],maxDeflectionMm:0.25,maxRelativeResidual:0,freeDofs:1})
afterEach(()=>{clients.splice(0).forEach(c=>c.dispose());vi.useRealTimers()})

it('reuses the CAD realm for truss and inspection and retains typed failures',async()=>{
  const {client,ports}=setup()
  const first=client.run({kind:'truss',model:model()})
  ports[0].reply({ok:false,error:{name:'GeometryKernelError',code:'TRUSS_SINGULAR',message:'Unrestrained mode'}})
  await expect(first).rejects.toMatchObject({name:'GeometryKernelError',code:'TRUSS_SINGULAR'})
  const second=client.run({kind:'inspect',bodies:[]})
  ports[0].reply({ok:true,result:[]})
  await expect(second).resolves.toEqual([])
  expect(ports).toHaveLength(1);expect(ports[0].terminate).not.toHaveBeenCalled()
})

it('settles superseded work once and ignores captured late callbacks',async()=>{
  const {client,ports}=setup()
  const first=client.run({kind:'truss',model:model()}), rejected=expect(first).rejects.toMatchObject({name:'AbortError'})
  const stale=ports[0].onmessage!
  const next=client.run({kind:'truss',model:model()})
  await rejected;expect(ports[0].terminate).toHaveBeenCalledTimes(1)
  stale({data:{version:1,id:1,kind:'truss',ok:true,result:response()}} as MessageEvent)
  ports[1].reply({ok:true,result:response()})
  await expect(next).resolves.toEqual(response())
})

it('ignores old IDs on a warm realm and snapshots response dimensions',async()=>{
  const {client,ports}=setup(), input=model()
  const first=client.run({kind:'truss',model:input})
  ports[0].reply({ok:true,result:response()});await first
  const next=client.run({kind:'truss',model:input}), resolved=vi.fn()
  void next.then(resolved)
  input.nodesMm.push([1,1,1]);input.members.length=0
  ports[0].reply({id:1,ok:true,result:response()})
  await Promise.resolve();expect(resolved).not.toHaveBeenCalled()
  ports[0].reply({ok:true,result:response()});await expect(next).resolves.toEqual(response())
})

it.each([
  {version:2}, {id:99}, {kind:'inspect'}, {result:{}},
  {result:{...response(),axialForcesN:[NaN]}},
  {result:{...response(),axialForcesN:new Array(1)}},
  {result:{...response(),displacementsMm:[]}},
  {result:{...response(),maxRelativeResidual:1}},
  {ok:false,error:{name:'Error',message:'bad',code:42}},
])('discards malformed or mismatched responses: %j',async patch=>{
  const {client,ports}=setup(), task=client.run({kind:'truss',model:model()})
  ports[0].reply({ok:true,result:response(),...patch})
  await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
  expect(ports[0].terminate).toHaveBeenCalledTimes(1)
  expect(ports[0].onmessage).toBeNull()
})

it('aborts noncooperative work, cleans listeners and starts a fresh realm',async()=>{
  const {client,ports}=setup(), controller=new AbortController()
  const remove=vi.spyOn(controller.signal,'removeEventListener')
  const options={signal:controller.signal}
  const task=client.run({kind:'truss',model:model()},options)
  options.signal=new AbortController().signal
  controller.abort()
  await expect(task).rejects.toMatchObject({name:'AbortError'})
  expect(remove).toHaveBeenCalledWith('abort',expect.any(Function))
  expect(ports[0].terminate).toHaveBeenCalledTimes(1)
  const next=client.run({kind:'truss',model:model()})
  ports[1].reply({ok:true,result:response()});await next
})

it('times out, clears timers and does not time out a later successful job',async()=>{
  vi.useFakeTimers()
  const {client,ports}=setup(), task=client.run({kind:'truss',model:model()},{timeoutMs:25})
  const rejected=expect(task).rejects.toMatchObject({code:'CAD_TIMEOUT'})
  await vi.advanceTimersByTimeAsync(25);await rejected
  expect(ports[0].terminate).toHaveBeenCalledTimes(1)
  const next=client.run({kind:'truss',model:model()})
  ports[1].reply({ok:true,result:response()});await next
  expect(vi.getTimerCount()).toBe(0)
})

it('rejects pre-aborted and invalid deadline calls without superseding active work',async()=>{
  const {client,ports}=setup(), task=client.run({kind:'truss',model:model()})
  const controller=new AbortController();controller.abort()
  await expect(client.run({kind:'truss',model:model()},{signal:controller.signal})).rejects.toMatchObject({name:'AbortError'})
  for(const timeoutMs of [0,NaN,Infinity,120001])await expect(client.run({kind:'truss',model:model()},{timeoutMs})).rejects.toThrow(RangeError)
  expect(ports[0].terminate).not.toHaveBeenCalled()
  ports[0].reply({ok:true,result:response()});await task
})

it.each(['onerror','onmessageerror'] as const)('discards crashed or undecodable workers: %s',async event=>{
  const {client,ports}=setup(), task=client.run({kind:'truss',model:model()})
  ports[0][event]!({message:'crash'} as ErrorEvent & MessageEvent)
  await expect(task).rejects.toMatchObject({code:event==='onerror'?'CAD_CRASH':'CAD_TRANSPORT'})
  expect(ports[0].terminate).toHaveBeenCalledTimes(1)
})

it('recovers from constructor and postMessage failures and closes deterministically',async()=>{
  let count=0
  const port=new Port(), factory=()=>{if(count++===0)throw Error('startup');return port}
  const client=new MainSolidWorkerClient(factory);clients.push(client)
  await expect(client.run({kind:'truss',model:model()})).rejects.toMatchObject({code:'CAD_STARTUP'})
  port.postMessage.mockImplementationOnce(()=>{throw new DOMException('clone','DataCloneError')})
  await expect(client.run({kind:'truss',model:model()})).rejects.toMatchObject({code:'CAD_TRANSPORT'})
  expect(port.terminate).toHaveBeenCalledTimes(1)
  const task=client.run({kind:'truss',model:model()})
  const rejected=expect(task).rejects.toMatchObject({name:'AbortError'})
  client.dispose();await rejected
  await expect(client.run({kind:'truss',model:model()})).rejects.toMatchObject({code:'CAD_DISPOSED'})
})
