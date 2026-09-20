import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {Worker} from 'node:worker_threads'
import {GcodePreviewWorker,type GcodeWorkerPort} from '../src/services/gcodePreviewWorker'
import {executeGcodePreviewAsync} from '../src/services/gcodePreviewRuntime'
import type {GcodePreviewRequest,GcodePreviewDocument} from '../src/services/gcodePreviewProtocol'
import identity from '../src/generated/geometry-kernels/identity'

const artifact=readFileSync('public/wasm/geometry-kernel.wasm')
const sha256=(value:Uint8Array|string)=>createHash('sha256').update(value).digest('hex')
assert.equal(sha256(artifact),identity.sha256)
assert.equal(artifact.length,identity.byteLength)
assert.equal(sha256(readFileSync('src/generated/geometry-kernels/kernel_bg.wasm')),identity.sha256)
const terminations:Promise<number>[]=[]
const transport=process.env.GCODE_WORKER_TRANSPORT??'transfer'
assert.ok(transport==='transfer'||transport==='legacy')
let created=0
class NodePort implements GcodeWorkerPort {
  private callbacks=new Map<string,Map<EventListener,(data:unknown)=>void>>()
  private terminated=false
  constructor(private worker:Worker){}
  postMessage(request:GcodePreviewRequest){
    const {responseFormat,...legacy}=request
    this.worker.postMessage(transport==='legacy'?legacy:request)
  }
  terminate(){if(!this.terminated){this.terminated=true;terminations.push(this.worker.terminate())}}
  addEventListener(type:'message'|'error'|'messageerror',listener:EventListener){
    const callback=(data:unknown)=>listener({data} as MessageEvent)
    const group=this.callbacks.get(type)??new Map()
    group.set(listener,callback);this.callbacks.set(type,group);this.worker.on(type,callback)
  }
  removeEventListener(type:'message'|'error'|'messageerror',listener:EventListener){
    const group=this.callbacks.get(type),callback=group?.get(listener)
    if(callback)this.worker.off(type,callback)
    group?.delete(listener)
  }
}
const client=new GcodePreviewWorker(()=>{
  created++
  return new NodePort(new Worker(new URL('../tests/fixtures/web-worker-node-harness.mjs',import.meta.url),{
    workerData:{entryUrl:new URL('../src/workers/gcodePreview.worker.ts',import.meta.url).href,announceReady:false},
  }))
})
const summary=(samplesMs:number[])=>{
  const sorted=[...samplesMs].sort((a,b)=>a-b)
  return {p50Ms:sorted[15],p95Ms:sorted[29],samplesMs}
}
const results=[]
try {
  for(const moves of [100,10_000,80_000]) {
    let gcode='G90\nM83\nG1 X0 Y0 Z0.2 F600\n'
    for(let i=0;i<moves;i++)gcode+=`G1 X${(i+1)%2} Y0 E0.01 F600\n`
    const job={kind:'parse' as const,gcode},request={version:1 as const,id:1,job}
    const direct=async()=>{
      const response=await executeGcodePreviewAsync(request)
      if(!response.ok)throw Error(response.error)
      return response.result
    }
    const expected=await direct()
    assert.equal(expected.preview.moves.length,moves+1)
    assert.equal(expected.preview.printDistanceMm,moves)
    assert.ok(Math.abs(expected.preview.extrusionMm-moves*.01)<1e-7)
    assert.deepEqual(await client.run(job),expected)
    for(let i=0;i<10;i++){await direct();await client.run(job)}
    const directMs:number[]=[],workerMs:number[]=[]
    const verify=(result:GcodePreviewDocument)=>{
      assert.equal(result.preview.moves.length,moves+1)
      assert.equal(result.preview.estimatedTimeS,expected.preview.estimatedTimeS)
      assert.deepEqual(result.preview.moves.at(-1),expected.preview.moves.at(-1))
    }
    for(let i=0;i<31;i++)for(const worker of i%2?[true,false]:[false,true]){
      const start=performance.now(),result=await(worker?client.run(job):direct()),elapsed=performance.now()-start
      verify(result)
      ;(worker?workerMs:directMs).push(elapsed)
    }
    results.push({moves,inputBytes:Buffer.byteLength(gcode),inputSha256:sha256(gcode),
      resultSha256:sha256(JSON.stringify(expected)),resultJsonBytes:Buffer.byteLength(JSON.stringify(expected)),
      direct:summary(directMs),workerRoundtrip:summary(workerMs)})
  }
  assert.equal(created,1,'Warm campaign must reuse a single worker')
  console.log(JSON.stringify({node:process.version,arch:process.arch,platform:process.platform,
    artifactSha256:identity.sha256,transport,workersCreated:created,warmups:10,samples:31,
    workerSourceSha256:sha256(readFileSync('src/workers/gcodePreview.worker.ts')),
    scope:'Actual worker entrypoint and client through Node worker_threads; includes validation, message transport and result delivery, not browser rendering or cold startup',results},null,2))
} finally {client.dispose();await Promise.all(terminations)}
