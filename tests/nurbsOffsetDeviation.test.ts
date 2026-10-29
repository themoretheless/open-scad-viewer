import {afterEach,beforeAll,expect,it} from 'vitest'
import {Worker} from 'node:worker_threads'
import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import identity from '../src/generated/geometry-kernels/identity'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import type {NurbsSurface} from '../src/services/nurbsSurface'
import {inspectNurbsOffsetDeviation,validOffsetDeviation,type OffsetDeviationOptions} from '../src/services/nurbsOffsetDeviation'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import {mainSolidExpectation,mainSolidResult,type MainSolidRequest} from '../src/services/mainSolidProtocol'

const plane=():NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
 controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false})
function options(distance=.25):OffsetDeviationOptions {
 const source=plane(),candidate=structuredClone(source)
 for(const row of candidate.controlPoints)for(const p of row)p[2]+=distance
 return {source,candidate,distance,tolerance:1e-8,maxCells:10000,maxDepth:24}
}
beforeAll(async()=>{await warmGeometryKernel()})
it.each([.25,-.25])('certifies the saved signed offset %s through the actual WASM and preserves inputs',distance=>{
 const o=options(distance),before=structuredClone(o),r=inspectNurbsOffsetDeviation(o)
 expect(r).toMatchObject({decision:'within_tolerance',stop:'tolerance',continuousBound:true,
  sourceNormalsProven:true,idealLocalFoldFree:true,candidateLocallyRegular:true,topologyAuthority:false,globalEmbeddingCertified:false})
 expect(r.errorUpper).toBeLessThanOrEqual(o.tolerance)
 expect(validOffsetDeviation(o,r)).toBe(true)
 const expected=mainSolidExpectation({kind:'offsetDeviation',options:o})
 expect(mainSolidResult(expected,r)).toBe(true)
 for(const forged of [{...r,topologyAuthority:true},{...r,globalEmbeddingCertified:true},{...r,errorUpper:-1},
  {...r,errorUpper:o.tolerance*2},{...r,continuousBound:false},{...r,cells:o.maxCells+1}])expect(mainSolidResult(expected,forged)).toBe(false)
 expect(o).toEqual(before)
})
it('finds a narrow defect missed by the former uniform samples and returns no partial bound',()=>{
 const o=options(.5),us=[0,.5,.5001,.5002,1]
 o.candidate={degreeU:1,degreeV:1,knotsU:[0,0,.5,.5001,.5002,1,1],knotsV:[0,0,1,1],
  controlPoints:us.map(u=>[0,1].map(v=>[u,v,u===.5001?.6:.5])),weights:us.map(()=>[1,1])}
 const r=inspectNurbsOffsetDeviation(o)
 expect(r).toMatchObject({decision:'exceeds_tolerance',stop:'witness',errorUpper:null,continuousBound:false})
 expect(r.errorLower).toBeGreaterThan(o.tolerance)
 expect(r.witnessUv![0]).toBeGreaterThan(.5)
 expect(r.witnessUv![0]).toBeLessThan(.5002)
 expect(validOffsetDeviation(o,r)).toBe(true)
 const limited=inspectNurbsOffsetDeviation({...o,maxCells:0})
 expect(limited).toMatchObject({decision:'unresolved',stop:'work_limit',cells:0,errorUpper:null,continuousBound:false})
 expect(validOffsetDeviation({...o,maxCells:0},limited)).toBe(true)
 expect(validOffsetDeviation(o,{...limited,continuousBound:true})).toBe(false)
})
const workers:Worker[]=[],clients:MainSolidWorkerClient[]=[]
class NodePort implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null
 onerror:MainSolidPort['onerror']=null
 onmessageerror:MainSolidPort['onmessageerror']=null
 constructor(private worker:Worker){
  worker.on('message',data=>{
   if(data?.__webWorkerHarness==='artifact'){expect(data.identity).toEqual(identity);return}
   this.onmessage?.({data} as MessageEvent)
  })
  worker.on('error',error=>this.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
 }
 postMessage(message:MainSolidRequest){this.worker.postMessage(message)}
 terminate(){void this.worker.terminate()}
}
function realWorker(){
 const worker=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{
  workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false,expectedArtifact:identity}})
 workers.push(worker);return new NodePort(worker)
}
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})
it('runs the shipped worker on the same artifact and recovers after an invalid request and cancellation',async()=>{
 const bytes=readFileSync('public/wasm/geometry-kernel.wasm')
 expect({sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length}).toEqual(identity)
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const o=options(),snapshot=structuredClone(o)
 await expect(client.run({kind:'offsetDeviation',options:{...o,maxDepth:41}})).rejects.toThrow()
 expect(await client.run({kind:'offsetDeviation',options:o})).toMatchObject({decision:'within_tolerance'})
 expect(await client.run({kind:'offsetDeviation',options:{...o,maxCells:0}})).toMatchObject({decision:'unresolved',errorUpper:null})
 const abort=new AbortController(),pending=client.run({kind:'offsetDeviation',options:o},{signal:abort.signal})
 abort.abort()
 await expect(pending).rejects.toMatchObject({code:'CAD_CANCELLED'})
 expect(await client.run({kind:'offsetDeviation',options:o})).toMatchObject({decision:'within_tolerance',topologyAuthority:false})
 expect(o).toEqual(snapshot)
},30_000)
