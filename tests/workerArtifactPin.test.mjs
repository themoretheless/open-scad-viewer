import {test} from 'node:test'
import assert from 'node:assert/strict'
import {Worker} from 'node:worker_threads'
import {register} from 'tsx/esm/api'
register()
const {default:identity}=await import('../src/generated/geometry-kernels/identity.ts')
function startup(expectedArtifact,harness=false){
 const worker=new Worker(new URL(harness?'./fixtures/web-worker-node-harness.mjs':'../scripts/check-cad-whole-wall-worker-entry.mjs',import.meta.url),{workerData:{expectedArtifact,...(harness?{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false}:{})}})
 const ready=new Promise((resolve,reject)=>{
  worker.once('error',reject)
  worker.once('message',resolve)
  worker.once('exit',code=>reject(new Error(`Worker exited before artifact handshake: ${code}`)))
 })
 return {worker,ready}
}
test('qualification worker reports its imported artifact before accepting requests',{timeout:60000},async()=>{
 const {worker,ready}=startup(identity)
 try{assert.deepEqual(await ready,{artifact:identity})}finally{await worker.terminate()}
})
test('qualification worker refuses a different requested publication',{timeout:60000},async()=>{
 const {worker,ready}=startup({...identity,sha256:identity.sha256==='0'.repeat(64)?'1'.repeat(64):'0'.repeat(64)})
 try{await assert.rejects(ready,/Worker loaded a different WASM publication/)}finally{await worker.terminate()}
})

test('source-body harness pins the kernel before dispatching queued jobs',{timeout:60000},async()=>{
 const {worker,ready}=startup(identity,true)
 try{assert.deepEqual(await ready,{__webWorkerHarness:'artifact',identity})}finally{await worker.terminate()}
})
test('source-body harness rejects a publication change before queued jobs',{timeout:60000},async()=>{
 const {worker,ready}=startup({...identity,byteLength:identity.byteLength+1},true)
 try{await assert.rejects(ready,/Worker loaded a different WASM publication/)}finally{await worker.terminate()}
})
