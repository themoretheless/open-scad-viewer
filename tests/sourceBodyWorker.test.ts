import {createSourceBodyRecord} from '../src/services/sourceBodyArchive'
import {emptyDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {afterEach,expect,it} from 'vitest'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {gunzipSync} from 'node:zlib'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest} from '../src/services/mainSolidProtocol'
import {sourceBodyExpectation,validSourceBody,type SourceBodyOptions,type SourceBodyResult} from '../src/services/sourceBody'
const request=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-request.json.gz',import.meta.url))).toString())
const options=():SourceBodyOptions=>{const {op:_op,...value}=structuredClone(request);return value}
class Port implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 requests:MainSolidRequest[]=[];terminated=false
 postMessage(request:MainSolidRequest){this.requests.push(structuredClone(request))}
 terminate(){this.terminated=true}
}
const timings:unknown[]=[]
class NodePort implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 constructor(readonly worker:Worker){
  worker.on('message',data=>{if(data.kind==='sourceBodyRestore'&&data.timing)timings.push({id:data.id,ok:data.ok,admitted:data.result?.admitted,...data.timing});this.onmessage?.({data} as MessageEvent)})
  worker.on('error',e=>this.onerror?.({message:e.message} as ErrorEvent))
  worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
 }
 postMessage(value:MainSolidRequest){this.worker.postMessage({...value,traceTiming:true})}
 terminate(){void this.worker.terminate()}
}
const clients:MainSolidWorkerClient[]=[],workers:Worker[]=[]
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})
function realClient(){
 const client=new MainSolidWorkerClient(()=>{
  const worker=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{
   workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false}})
  workers.push(worker);return new NodePort(worker)
 });clients.push(client);return client
}
const denied:SourceBodyResult={admitted:false,sourceBody:null,edges:[],diagnostics:{reason:'source-volume-initial-work-limit',incidence:{},embedding:{},volume:{}}}
function reply(port:Port,result:SourceBodyResult){port.onmessage?.({data:{version:1,id:port.requests.at(-1)!.id,kind:'sourceBodyRestore',ok:true,result}} as MessageEvent)}
it('restores original source definitions through the real WASM CAD worker',async()=>{
 const o={...options(),displaySegments:8,faceDisplay:{divisions:4,toleranceUv:1e-8,domainCellsPerFace:10000}},before=JSON.stringify(o),client=realClient()
 const start=performance.now()
 const r=await client.run({kind:'sourceBodyRestore',options:o})
 const restoreElapsedMs=performance.now()-start
 expect(r.admitted).toBe(true);expect(validSourceBody(sourceBodyExpectation(o),r)).toBe(true)
 expect(r.sourceBody!.shell).toEqual(o.definition.shell)
 expect(r.edges.map(e=>e.definition)).toEqual(o.definition.shell.pairs.map(p=>p.edge))
 expect(r.edges.every(edge=>edge.displaySegments?.length===8)).toBe(true)
 expect(r.displayFaces).toHaveLength(r.faceCount!)
 expect(r.displayFaces!.every(face=>face.tiles.length===16&&face.unresolved.length===0&&face.unresolvedBoxes.length===0)).toBe(true)
 const incomplete=structuredClone(r);incomplete.displayFaces![0]!.tiles.pop()
 expect(validSourceBody(sourceBodyExpectation(o),incomplete)).toBe(false)

 const broken=structuredClone(r);broken.edges[0]!.displaySegments![0]![0][0]+=1
 expect(validSourceBody(sourceBodyExpectation(o),broken)).toBe(false)
 const missing=structuredClone(r);delete missing.edges[0]!.displaySegments
 expect(validSourceBody(sourceBodyExpectation(o),missing)).toBe(false)
 expect(()=>sourceBodyExpectation({...o,displaySegments:4097})).toThrow()

 const oracle=2*Math.PI/3;expect(r.volume![0]).toBeLessThanOrEqual(oracle);expect(r.volume![1]).toBeGreaterThanOrEqual(oracle)
 const changed=structuredClone(r);changed.edges[0]!.definition={changed:true}
 expect(validSourceBody(sourceBodyExpectation(o),changed)).toBe(false)
 const signedZero=structuredClone(r),points=(signedZero.edges[0]!.definition as {world:{controlPoints:number[][]}}).world.controlPoints
 let changedZero=false
 for(const point of points){const axis=point.findIndex(n=>Object.is(n,0));if(axis>=0){point[axis]=-0;changedZero=true;break}}
 expect(changedZero).toBe(true);expect(validSourceBody(sourceBodyExpectation(o),signedZero)).toBe(false)
 const low=options();low.limits.volume.cells=1
 const refusal=await client.run({kind:'sourceBodyRestore',options:low})
 expect(refusal.admitted).toBe(false);expect(refusal.sourceBody).toBeNull();expect(refusal.edges).toEqual([])
 expect(JSON.stringify(o)).toBe(before)
 expect(timings).toHaveLength(2)
 if(process.env.CAD_SOURCE_BODY_WORKER_REPORT){
  const wasm=readFileSync(new URL('../public/wasm/geometry-kernel.wasm',import.meta.url))
  writeFileSync(process.env.CAD_SOURCE_BODY_WORKER_REPORT,JSON.stringify({schema:'source-body-worker/1',passed:true,wasmBytes:wasm.length,wasmSha256:createHash('sha256').update(wasm).digest('hex'),restoreElapsedMs,timings,volume:r.volume,edgeCount:r.edges.length,faceCount:r.faceCount,scope:'Native-authored equal-radius capped canal through the shipped worker and WASM; executeMs includes the adapter and native gates, and does not isolate Rust or rendered UI latency.'},null,2)+'\n')
 }
},120000)
it('supersedes a source restore and ignores its late response',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client)
 const first=client.run({kind:'sourceBodyRestore',options:options()}).catch(e=>e.code)
 const late=ports[0]!.onmessage!
 const second=client.run({kind:'sourceBodyRestore',options:options()})
 expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 late({data:{version:1,id:1,kind:'sourceBodyRestore',ok:true,result:denied}} as MessageEvent)
 reply(ports[1]!,denied);expect(await second).toEqual(denied)
})
it('aborts a source restore and permits retry in a fresh worker',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client)
 const controller=new AbortController(),first=client.run({kind:'sourceBodyRestore',options:options()},{signal:controller.signal}).catch(e=>e.code)
 controller.abort();expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 const retried=client.run({kind:'sourceBodyRestore',options:options()});reply(ports[1]!,denied);expect(await retried).toEqual(denied)
})

it('loads a source archive through the real document worker and rejects saved false admission',async()=>{
 const record=createSourceBodyRecord('source-archive','Source archive',options())
 const document={...emptyDirectDocument(),sourceBodies:[record]},client=realClient()
 const loaded=await client.run({kind:'restoreDocument',text:serializeDirectDocument(document)})
 expect(loaded.sourceBodies).toEqual([record])
 const low=options();low.limits.volume.cells=1
 const invalid={...emptyDirectDocument(),sourceBodies:[{...createSourceBodyRecord('bad-source','Bad source',low),admitted:true}]}
 await expect(client.run({kind:'restoreDocument',text:serializeDirectDocument(invalid)})).rejects.toMatchObject({code:'CAD_SOURCE_BODY_RESTORE',message:expect.stringContaining('bad-source')})
},30000)
