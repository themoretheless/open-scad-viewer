import {Worker} from 'node:worker_threads'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {afterEach,expect,it} from 'vitest'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest,MainSolidJob} from '../src/services/mainSolidProtocol'
import type {DirectDocument} from '../src/services/directModeling'
import identity from '../src/generated/geometry-kernels/identity'
const bytes=readFileSync('public/wasm/geometry-kernel.wasm')
const expectedArtifact={sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length}
expect(identity).toEqual(expectedArtifact)
const workers:Worker[]=[],clients:MainSolidWorkerClient[]=[]
class NodePort implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 constructor(readonly worker:Worker){
  worker.on('message',data=>{if(data?.__webWorkerHarness==='artifact'){expect(data.identity).toEqual(expectedArtifact);return}this.onmessage?.({data} as MessageEvent)})
  worker.on('error',error=>this.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
 }
 postMessage(message:MainSolidRequest){this.worker.postMessage(message)}
 terminate(){void this.worker.terminate()}
}
function client(){const c=new MainSolidWorkerClient(()=>{
 const w=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false,expectedArtifact}})
 workers.push(w);return new NodePort(w)
});clients.push(c);return c}
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})
const source={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
function job(width:number):Extract<MainSolidJob,{kind:'surfaceBuild'}>{
 const plane=structuredClone(source);for(const row of plane.controlPoints)for(const p of row){p[2]=p[1];p[1]=1}
 const document:DirectDocument={version:1,bodies:[],sketches:[],surfaces:[{id:'source',name:'Source',surface:source,segmentsU:4,segmentsV:4},{id:'plane',name:'End',surface:plane,segmentsU:4,segmentsV:4}],curves:[{id:'curve',name:'Target',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,1,width],[1,1,width]],weights:[1,1]}}]}
 return {kind:'surfaceBuild',document,options:{kind:'nurbs-runout',ids:['source','plane','curve'],id:'runout',reversed:[],sweep:{mode:'translation',normal:[0,0,1],sections:4,maxDeviation:.01},runout:{sourceBoundary:'vMax',sourcePosition:1-width,tangentLengthMm:width,flipSide:false,reverse:false}}}
}
it('applies whole-field qualification through the actual worker realm and packaged WASM',async()=>{
 const c=client()
 for(const width of [.5,.1]){
  const request=job(width),before=structuredClone(request.document),r=await c.run(request)
  expect(r.runoutReport?.method).toBe('native-original-support-G2-runout/2')
  expect(r.runoutReport?.seamWithinTolerance).toBe(true)
  if(width===.5){expect(r.error).toBe('');expect(r.document?.surfaces).toHaveLength(3)}
  else{expect(r.document).toBe(null);expect(r.error).toBe('RUNOUT_CURVATURE_LIMIT');expect(r.runoutReport?.fields.some(f=>f.curvatureDecision==='exceeds')).toBe(true)}
  expect(request.document).toEqual(before)
 }
},60_000)
it('cancels the old realm and accepts a verified retry from a replacement',async()=>{
 const c=client(),first=c.run(job(.5)),rejected=expect(first).rejects.toMatchObject({name:'AbortError'})
 c.cancel();await rejected
 const r=await c.run(job(.5))
 expect(workers).toHaveLength(2);expect(r.error).toBe('');expect(r.runoutReport?.withinTolerance).toBe(true)
},60_000)
