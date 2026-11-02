import {Worker} from 'node:worker_threads'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {afterEach,expect,it} from 'vitest'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest,MainSolidJob} from '../src/services/mainSolidProtocol'
import identity from '../src/generated/geometry-kernels/identity'
const bytes=readFileSync('public/wasm/geometry-kernel.wasm')
const expectedArtifact={sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length}
const workers:Worker[]=[],clients:MainSolidWorkerClient[]=[]
class Port implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 constructor(readonly worker:Worker){
  worker.on('message',data=>{if(data?.__webWorkerHarness==='artifact'){expect(data.identity).toEqual(expectedArtifact);return}this.onmessage?.({data} as MessageEvent)})
  worker.on('error',error=>this.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
 }
 postMessage(message:MainSolidRequest){this.worker.postMessage(message)}
 terminate(){void this.worker.terminate()}
}
function client(){expect(identity).toEqual(expectedArtifact);const c=new MainSolidWorkerClient(()=>{
 const w=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false,expectedArtifact}})
 workers.push(w);return new Port(w)
});clients.push(c);return c}
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})
function job():Extract<MainSolidJob,{kind:'sourceGraphG2Preview'}>{return {
 kind:'sourceGraphG2Preview',sources:[0,1].map(side=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[0,1].map(x=>[[x,0,0],[x,side===0?-1:1,-1/1024]]),weights:[[1,1],[1,1]]})) as any,
 width:{degree:1,knots:[0,0,1,1],controlPoints:[[1/1024,0,0],[1/1024,0,0]],weights:[1,1]},options:{radiusMm:1,radiusToleranceMm:.001,maxWork:1000000,maxCells:1000}
}}
it('preserves full radius refusal through a separate real worker realm',async()=>{
 const c=client(),request=job(),before=structuredClone(request),r=await c.run(request)
 expect(r.report.curvatureDecision).toBe('within');expect(r.report.radiusDecision).toBe('exceeds')
 expect(r.report.applicationAdmitted).toBe(false);expect(r.report.radiusDomain).toEqual([[0,1],[0,1]])
 expect(request).toEqual(before)
},60000)
it('terminates a cancelled realm and retries in a replacement',async()=>{
 const c=client();const pending=c.run(job());const rejection=expect(pending).rejects.toThrow()
 c.cancel();await rejection
 const r=await c.run(job());expect(r.report.applicationAdmitted).toBe(false)
 expect(r.report.radiusDecision).toBe('exceeds');expect(workers.length).toBeGreaterThanOrEqual(2)
},60000)
