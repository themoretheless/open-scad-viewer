import {afterEach, expect, it, vi} from 'vitest'
import {MainSolidWorkerClient, type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidJob,MainSolidRequest} from '../src/services/mainSolidProtocol'
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

it('publishes validated progress without letting observers change settlement',async()=>{
 const {client,ports}=setup(),onProgress=vi.fn(()=>{throw Error('observer failure')})
 const task=client.run({kind:'inspect',bodies:[]},{onProgress}),resolved=vi.fn()
 void task.then(resolved)
 ports[0].reply({progress:{phase:'check',completed:0,total:3}})
 expect(onProgress).toHaveBeenCalledWith({phase:'check',completed:0,total:3})
 await Promise.resolve();expect(resolved).not.toHaveBeenCalled();expect(ports[0].terminate).not.toHaveBeenCalled()
 ports[0].reply({progress:{phase:'repair',completed:1,total:3}})
 ports[0].reply({ok:true,result:[]});await expect(task).resolves.toEqual([])
 expect(onProgress).toHaveBeenCalledTimes(2)
})

it.each([null,{phase:'',completed:0,total:3},{phase:'check',completed:-1,total:3},{phase:'check',completed:4,total:3},{phase:'check',completed:0,total:0}])('rejects malformed worker progress: %j',async progress=>{
 const {client,ports}=setup(),task=client.run({kind:'inspect',bodies:[]})
 ports[0].reply({progress})
 await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 expect(ports[0].terminate).toHaveBeenCalledOnce()
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

const manifoldCancellationJobs=():MainSolidJob[]=>{
  const mesh={positions:new Float64Array([0,0,0,1,0,0,0,1,0,0,0,1]),indices:new Uint32Array([0,2,1,0,1,3,1,2,3,2,0,3])}
  return [
    {kind:'manifoldMetrics',mesh},
    {kind:'meshManifold',objects:[{id:'a',mesh}]},
    {kind:'meshRepair',mesh},
    {kind:'meshBoolean',a:mesh,b:mesh,operation:'union'},
  ]
}
it.each(manifoldCancellationJobs())('cancels $kind without retaining callbacks or blocking its retry',async job=>{
  const {client,ports}=setup()
  const first=client.run(job as never),rejected=expect(first).rejects.toMatchObject({code:'CAD_CANCELLED'})
  const stale=ports[0].onmessage!
  client.cancel();await rejected
  expect(ports[0].terminate).toHaveBeenCalledOnce()
  expect(ports[0].onmessage).toBeNull();expect(ports[0].onerror).toBeNull();expect(ports[0].onmessageerror).toBeNull()
  const retry=client.run(job as never),retryRejected=expect(retry).rejects.toMatchObject({code:'CAD_CANCELLED'})
  expect(ports).toHaveLength(2)
  expect(ports.map(port=>port.postMessage.mock.calls[0][0].id)).toEqual([1,2])
  stale({data:{version:1,id:1,kind:job.kind,ok:true,result:{}}} as MessageEvent)
  client.cancel();await retryRejected
  expect(ports[1].terminate).toHaveBeenCalledOnce()
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

it('observes stage timings only for the accepted response without changing settlement',async()=>{
  const {client,ports}=setup(),onTiming=vi.fn((_timing:import('../src/services/mainSolidWorkerClient').MainSolidTiming)=>{throw new Error('observer failure')})
  const task=client.run({kind:'inspect',bodies:[]},{onTiming})
  expect(ports[0].postMessage.mock.lastCall![0].traceTiming).toBe(true)
  ports[0].reply({id:0,ok:true,result:[],timing:{warmupMs:1,executeMs:2,prepareMs:3}})
  await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
  expect(onTiming).not.toHaveBeenCalled()
  const retry=client.run({kind:'inspect',bodies:[]},{onTiming})
  ports[1].reply({ok:true,result:[],timing:{warmupMs:1,executeMs:2,prepareMs:3}})
  await expect(retry).resolves.toEqual([])
  expect(onTiming).toHaveBeenCalledTimes(1)
  expect(onTiming.mock.calls[0][0]).toMatchObject({warmupMs:1,executeMs:2,prepareMs:3})
})
it('keeps timing optional and ignores malformed diagnostic values',async()=>{
  const {client,ports}=setup(),onTiming=vi.fn()
  const first=client.run({kind:'inspect',bodies:[]})
  expect(ports[0].postMessage.mock.lastCall![0].traceTiming).toBeUndefined()
  ports[0].reply({ok:true,result:[]});await first
  const next=client.run({kind:'inspect',bodies:[]},{onTiming})
  ports[0].reply({id:1,ok:true,result:[],timing:{warmupMs:1,executeMs:2,prepareMs:3}})
  expect(onTiming).not.toHaveBeenCalled()
  ports[0].reply({ok:true,result:[],timing:{warmupMs:NaN,executeMs:2,prepareMs:3}})
  await expect(next).resolves.toEqual([]);expect(onTiming).not.toHaveBeenCalled()
})
it('reports a typed operation failure but never reports a late cancelled response',async()=>{
  const {client,ports}=setup(),onTiming=vi.fn(),timing={warmupMs:1,executeMs:2,prepareMs:0}
  const failure=client.run({kind:'inspect',bodies:[]},{onTiming})
  ports[0].reply({ok:false,error:{name:'Error',code:'CAD_OPERATION',message:'known failure'},timing})
  await expect(failure).rejects.toMatchObject({code:'CAD_OPERATION'})
  expect(onTiming).toHaveBeenCalledTimes(1)
  expect(onTiming.mock.calls[0][0].clientElapsedMs).toBeGreaterThanOrEqual(0)
  const controller=new AbortController()
  const pending=client.run({kind:'inspect',bodies:[]},{onTiming,signal:controller.signal})
  const rejected=expect(pending).rejects.toMatchObject({code:'CAD_CANCELLED'})
  const late=ports[0].onmessage!,request=ports[0].postMessage.mock.lastCall![0]
  controller.abort();await rejected
  late({data:{version:1,id:request.id,kind:'inspect',ok:true,result:[],timing}} as MessageEvent)
  expect(onTiming).toHaveBeenCalledTimes(1)
})

const topologyResult=()=>({densities:[.4],areasMm2:[40],volumeMm3:400000,targetVolumeMm3:400000,
 compliancesNmm:[125],initialComplianceNmm:50,complianceNmm:125,iterations:1,converged:true,
 maxDensityChange:.6,activeMembers:[],axialForcesN:[[1000]]})
const topologyJob=()=>{const {forcesN,...structure}=model();return {kind:'trussTopology' as const,model:structure,
 cases:[{weight:1,forcesN:[[0,0,0],[1000,0,0]] as [number,number,number][]}],options:{volumeFraction:.4,penalty:1}}}

it('accepts a complete SIMP response with request-sized arrays',async()=>{
 const {client,ports}=setup(),task=client.run(topologyJob())
 ports[0].reply({ok:true,result:topologyResult()})
 await expect(task).resolves.toEqual(topologyResult())
})

it.each([{densities:[NaN]},{densities:[]},{axialForcesN:[[NaN]]},{axialForcesN:[[]]}])('rejects malformed SIMP arrays: %j',async patch=>{
 const {client,ports}=setup(),task=client.run(topologyJob())
 ports[0].reply({ok:true,result:{...topologyResult(),...patch}})
 await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
})

const profile=()=>({designation:'IPE 200',family:'ipe' as const,heightMm:200,flangeWidthMm:100,
 webThicknessMm:5.6,flangeThicknessMm:8.5,areaMm2:2800,massKgM:22,iStrongMm4:19400000,
 iWeakMm4:1400000,wStrongMm3:194000,wWeakMm3:28000,jMm4:70000,cwMm6:13000000000})

it('accepts one complete catalogue and rejects duplicates or non-finite properties',async()=>{
 const {client,ports}=setup()
 const valid=client.run({kind:'structuralProfiles'})
 ports[0].reply({ok:true,result:[profile()]});await expect(valid).resolves.toEqual([profile()])
 for(const result of [[profile(),profile()],[{...profile(),areaMm2:NaN}]]){
  const task=client.run({kind:'structuralProfiles'})
  ports.at(-1)!.reply({ok:true,result})
  await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 }
})

it('accepts complete field meshes and rejects duplicate force groups',async()=>{
 const {trussFieldMeshes}=await import('../src/services/trussFieldMeshes')
 const {client,ports}=setup(),job={kind:'trussField' as const,model:model(),result:response(),markerMm:1}
 const meshes=trussFieldMeshes(job.model,job.result,job.markerMm)
 const valid=client.run(job);ports[0].reply({ok:true,result:meshes});await expect(valid).resolves.toEqual(meshes)
 const invalid=client.run(job);ports[0].reply({ok:true,result:[meshes[0],meshes[0]]})
 await expect(invalid).rejects.toMatchObject({code:'CAD_PROTOCOL'})
})

const screeningJob=()=>({kind:'trussScreening' as const,model:model(),responses:[response()],limits:{tensionMpa:80,compressionMpa:40,safetyFactor:2}})
const screeningRows=()=>[{memberIndex:0,utilization:1.25,governingResponse:0,stressMpa:50,status:'overloaded' as const}]
it('accepts ranked axial screening and rejects inconsistent rows',async()=>{
 const {client,ports}=setup()
 const valid=client.run(screeningJob());ports[0].reply({ok:true,result:screeningRows()});await expect(valid).resolves.toEqual(screeningRows())
 for(const result of [[{...screeningRows()[0],utilization:NaN}],[{...screeningRows()[0],governingResponse:1}],[{...screeningRows()[0],status:'low-demand'}]]){
  const task=client.run(screeningJob());ports.at(-1)!.reply({ok:true,result})
  await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 }
})

const printProfile=()=>({material:'PETG',grade:'Synthetic specimen',propertySource:'Regression only',nozzleMm:.4,lineWidthMm:.45,layerHeightMm:.2,nozzleTempC:240,bedTempC:80})
const validatedPrintProfile=()=>({profile:printProfile(),warnings:[] as string[],propertyModel:'user-supplied-isotropic-axial'})
it('binds validated print conditions to their request and rejects malformed warnings',async()=>{
 const {client,ports}=setup(),job={kind:'printStrengthProfile' as const,profile:printProfile()}
 const valid=client.run(job);ports[0].reply({ok:true,result:validatedPrintProfile()});await expect(valid).resolves.toEqual(validatedPrintProfile())
 for(const result of [
  {...validatedPrintProfile(),profile:{...printProfile(),nozzleTempC:250}},
  {...validatedPrintProfile(),warnings:['layer-above-80-percent-nozzle','layer-above-80-percent-nozzle']},
  {...validatedPrintProfile(),warnings:['unknown-warning']},
 ]){
  const task=client.run(job);ports.at(-1)!.reply({ok:true,result})
  await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 }
})

const thermalResult=()=>({modelKind:'measured-bilinear-isotropic-axial-v1',youngMpa:1500,tensionMpa:30,compressionMpa:45,nozzleBracketC:[240,240],serviceBracketC:[20,60]})
it('accepts measured thermal properties and rejects inconsistent results',async()=>{
 const {client,ports}=setup(),profile=printProfile(),job={kind:'thermalStrength' as const,request:{profile,calibrationProfile:{...profile},serviceTempC:40,samples:[
  {nozzleTempC:240,serviceTempC:20,youngMpa:2000,tensionMpa:40,compressionMpa:60},
  {nozzleTempC:240,serviceTempC:60,youngMpa:1000,tensionMpa:20,compressionMpa:30},
 ]}}
 const valid=client.run(job);ports[0].reply({ok:true,result:thermalResult()});await expect(valid).resolves.toEqual(thermalResult())
 for(const result of [
  {...thermalResult(),youngMpa:NaN},
  {...thermalResult(),modelKind:'unmeasured-model'},
  {...thermalResult(),serviceBracketC:[60,20]},
  {...thermalResult(),nozzleBracketC:[NaN,240]},
 ]){
  const task=client.run(job);ports.at(-1)!.reply({ok:true,result})
  await expect(task).rejects.toMatchObject({code:'CAD_PROTOCOL'})
 }
})
