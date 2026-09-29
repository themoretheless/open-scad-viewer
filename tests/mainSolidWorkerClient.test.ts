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

it.each([[[0,0,0],[1,0,NaN]],[[0,0],[1,0,0]],[[0,0,0]],'invalid'].map(closestPoints=>({closestPoints})))('rejects malformed closest-point coordinates: %j',async ({closestPoints})=>{
 const {client,ports}=setup(),task=client.run({kind:'inspect',bodies:[]})
 ports[0].reply({ok:true,result:[{a:'A',b:'B',gapMm:1,overlapMm3:0,closestPoints,displayMeshOnly:true}]})
 await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
})
it('preserves closest-point witnesses through the worker response',async()=>{
 const {client,ports}=setup(),task=client.run({kind:'inspect',bodies:[]})
 const result=[{a:'A',b:'B',gapMm:1,overlapMm3:0,closestPoints:[[0,0,0],[1,0,0]],displayMeshOnly:true}]
 ports[0].reply({ok:true,result});await expect(task).resolves.toEqual(result)
})

it('checks mesh-contact indices against the requested mesh',async()=>{
 const {client,ports}=setup()
 const mesh={positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])}
 const first=client.run({kind:'meshContacts',mesh})
 const clean={scope:'display-mesh-all-contacts',relativeTolerance:1e-9,contact:null,contacts:[],triangleIds:[],lines:[],complete:true,stopReason:null,work:1,maxWork:200_000,maxContacts:10_000}
 ports[0].reply({ok:true,result:clean})
 await expect(first).resolves.toMatchObject({contact:null})
 const bad=client.run({kind:'meshContacts',mesh})
 const invalid={triangles:[0,9],point:[0,0,0],sharedVertices:0,toleranceMm:1e-9}
 ports[0].reply({ok:true,result:{...clean,contact:invalid,contacts:[invalid]}})
 await expect(bad).rejects.toMatchObject({code:'CAD_PROTOCOL'})
})

it('rejects malformed profile diagnostics instead of exposing them to the scene',async()=>{
 const {client,ports}=setup()
 const document={version:1 as const,sketches:[],bodies:[]}
 const pending=client.run({kind:'profilePrepare',document,ids:[],tolerance:.01})
 ports[0].reply({ok:true,result:{document,id:'a',plane:{origin:[0,0,0],u:[1,0,0],v:[0,1,0]},report:{accepted:false,reason:'endpoint-topology',points:[],connectors:[],defects:[{chain:0,end:'start',point:[Infinity,0],kind:'gap',candidates:[]}]}}})
 await expect(pending).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 expect(ports[0].terminate).toHaveBeenCalledOnce()
})

it('refuses a contradictory NURBS certificate before it reaches Apply',async()=>{
 const {client,ports}=setup(),document={version:1 as const,sketches:[],bodies:[]}
 const pending=client.run({kind:'nurbsRefit',document,options:{operation:'nurbs-reduce',id:'curve',axis:'u',degree:1,controlCount:2,maxError:.01}})
 ports[0].reply({ok:true,result:{document,certificate:{version:'nurbs-foundation/1',accepted:true,rolledBack:true,evidence:{toleranceIdentity:{canonical:'test'}}}}})
 await expect(pending).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 expect(ports[0].terminate).toHaveBeenCalledOnce()
})

it('preserves incomplete mesh inspection and rejects contradictory completion claims',async()=>{
 const mesh={positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])}
 const partial={scope:'display-mesh-all-contacts',relativeTolerance:1e-9,contact:null,contacts:[],triangleIds:[],lines:[],complete:false,stopReason:'work-limit',work:1,maxWork:1,maxContacts:10_000}
 const {client,ports}=setup(),first=client.run({kind:'meshContacts',mesh,maxWork:1})
 ports[0].reply({ok:true,result:partial});await expect(first).resolves.toMatchObject({complete:false,contacts:[]})
 const second=client.run({kind:'meshContacts',mesh,maxWork:1})
 ports[0].reply({ok:true,result:{...partial,complete:true}});await expect(second).rejects.toMatchObject({code:'CAD_PROTOCOL'})
})

it('terminates a curve-distance request and ignores its late reply after restart',async()=>{
 const {client,ports}=setup(),curve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0],[1,0]],weights:[1,1]}
 const job={kind:'curveDistance' as const,a:curve,b:curve,toleranceMm:.001,maxCells:100}
 const first=client.run(job),old=ports[0].onmessage
 const rejected=expect(first).rejects.toMatchObject({name:'AbortError'})
 client.cancel();await rejected
 expect(ports[0].terminate).toHaveBeenCalledOnce()
 const second=client.run(job),result={method:'interval-de-boor-pair-subdivision',distanceIntervalMm:[0,1e-12],parameters:[0,0],points:[[0,0],[0,0]],pointEnclosures:[[[0,0],[0,0]],[[0,0],[0,0]]],converged:true,reason:'tolerance',cells:1,maxCells:100,toleranceMm:.001}
 old?.({data:{version:1,id:1,kind:'curveDistance',ok:true,result:{...result,maxCells:1}}} as MessageEvent)
 ports[1].reply({ok:true,result})
 await expect(second).resolves.toEqual(result)
})
