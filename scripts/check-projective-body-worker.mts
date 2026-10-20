import artifactIdentity from '../src/generated/geometry-kernels/identity'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import {sourceBodyExpectation,validSourceBody,type SourceBodyOptions} from '../src/services/sourceBody'

assert.ok(process.argv[2]&&process.argv[3],'Provide native fixture and output directories')
const fixture=resolve(process.argv[2]),directory=resolve(process.argv[3])
const wasmPath='public/wasm/geometry-kernel.wasm'
const hash=()=>createHash('sha256').update(readFileSync(wasmPath)).digest('hex')
const wasmSha256=hash()
assert.equal(artifactIdentity.sha256,wasmSha256)
const request=JSON.parse(readFileSync(resolve(fixture,'request.json'),'utf8'))
const {op:_op,...options}=request as {op:string}&SourceBodyOptions
const native=JSON.parse(readFileSync(resolve(fixture,'native-response.json'),'utf8'))
const expected=sourceBodyExpectation(options)
const timings:unknown[]=[]
const starts=new Map<number,number>()
const realms:{port:MainSolidPort;entered:Promise<void>;exited:Promise<number>;terminated:boolean}[]=[]
const client=new MainSolidWorkerClient(()=>{
  const worker=new Worker(new URL('./check-cad-whole-wall-worker-entry.mjs',import.meta.url),{workerData:{expectedArtifact:artifactIdentity}})
  let entered!:()=>void
  const realm={port:null as unknown as MainSolidPort,entered:new Promise<void>(resolve=>{entered=resolve}),
    exited:new Promise<number>(resolve=>worker.once('exit',resolve)),terminated:false}
  const port:MainSolidPort={onmessage:null,onerror:null,onmessageerror:null,
    postMessage(request){
      realm.entered=new Promise<void>(resolve=>{entered=resolve})
      starts.set(request.id,performance.now())
      worker.postMessage({...request,traceTiming:true})
    },
    terminate(){realm.terminated=true;void worker.terminate()},
  }
  worker.on('message',data=>{
    if(data?.artifact){assert.deepEqual(data.artifact,artifactIdentity);return}
    if(data?.entered){entered();return}
    if(data?.kind==='sourceBodyRestore'&&data.timing)timings.push({id:data.id,ok:data.ok,admitted:data.result?.admitted,elapsedMs:performance.now()-(starts.get(data.id)??performance.now()),...data.timing})
    port.onmessage?.({data} as MessageEvent)
  })
  worker.on('error',error=>port.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>port.onmessageerror?.({} as MessageEvent))
  realm.port=port;realms.push(realm);return port
})
let warm:any,retry:any
try{
  warm=await client.run({kind:'sourceBodyRestore',options})
  assert(validSourceBody(expected,warm))
  assert.equal(warm.admitted,true)
  assert.deepEqual(warm.sourceBody,native.sourceBody)
  assert.deepEqual(warm.edges,native.edges)
  assert.deepEqual(warm.volume,native.volume)
  assert.deepEqual(warm.wallCoverage,native.wallCoverage)
  assert.deepEqual(warm.wallScan,native.wallScan)
  assert.deepEqual(warm.stepExchange,native.stepExchange)
  const exhausted=structuredClone(options)
  exhausted.limits.embedding.pairs.pairs=1
  const budgetRefusal=await client.run({kind:'sourceBodyRestore',options:exhausted})
  assert(validSourceBody(sourceBodyExpectation(exhausted),budgetRefusal))
  assert.equal(budgetRefusal.admitted,false)
  assert.equal(budgetRefusal.sourceBody,null)
  const forged=structuredClone(options)
  ;(forged.definition.shell.pairs[0]!.edge as any).uses[0].weights=[1,2]
  const mapRefusal=await client.run({kind:'sourceBodyRestore',options:forged})
  assert(validSourceBody(sourceBodyExpectation(forged),mapRefusal))
  assert.equal(mapRefusal.admitted,false)
  assert.equal(mapRefusal.sourceBody,null)
  const abort=new AbortController()
  const pending=client.run({kind:'sourceBodyRestore',options},{signal:abort.signal})
  const refused=assert.rejects(pending,(e:any)=>e.name==='AbortError'&&e.code==='CAD_CANCELLED')
  await realms[0].entered
  abort.abort()
  await refused
  assert.equal(realms[0].terminated,true)
  await realms[0].exited
  retry=await client.run({kind:'sourceBodyRestore',options})
  assert.equal(realms.length,2,'Retry must create a fresh actual worker realm')
  assert(validSourceBody(expected,retry))
  assert.equal(retry.admitted,true)
  assert.deepEqual(retry,warm)
}finally{
  client.dispose()
  await Promise.all(realms.map(r=>r.exited))
}
assert.equal(hash(),wasmSha256,'WASM changed during qualification')
const report={schema:'projective-source-body-real-worker/1',passed:true,wasmSha256,
  workersStarted:realms.length,workersJoined:realms.length,timings,warm,retry,
  scope:'Actual Node worker realms and production handler/client: warm native result, abort after dispatch before response admission, terminate/join, and fresh-worker Retry. Dispatch observation does not prove interruption inside a Rust instruction. Timings measure client/worker transport and native adapter execution; they do not isolate Rust-only time or rendered UI latency. Native projective cuboid parity includes preview anchors, whole-wall coverage, thin witness and STEP. Browser UI and general curved bodies remain unqualified.'}
mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'worker-report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify({passed:true,workersStarted:realms.length,wasmSha256}))
