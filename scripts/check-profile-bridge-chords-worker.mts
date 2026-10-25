import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import {type ProfileBridgeChordOptions} from '../src/services/nurbsProfileBridgeChords'
import identity from '../src/generated/geometry-kernels/identity'
import {verifyPackedWasmChunk} from './verify-packed-wasm.mjs'
assert(process.argv[2],'Provide an output report path')
const wasm=readFileSync('public/wasm/geometry-kernel.wasm')
const sha256=createHash('sha256').update(wasm).digest('hex')
const requiredSha=process.argv.find(arg=>arg.startsWith('--expected-sha256='))?.slice('--expected-sha256='.length)
if(requiredSha){assert.match(requiredSha,/^[a-f0-9]{64}$/);assert.equal(sha256,requiredSha,'Requested WASM artifact has not been published; wait for the build to finish')}
assert.equal(identity.sha256,sha256);assert.equal(identity.byteLength,wasm.length)
verifyPackedWasmChunk(readFileSync('src/generated/geometry-kernels/bytes.ts','utf8'),wasm,'profile bridge chords')
const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/profile-bridge-native-transport-2026-10-10.json','utf8'))
const {op:_,...options}=fixture.request as ProfileBridgeChordOptions&{op:string}
const exits:Promise<number>[]=[]
let checkedWorkerArtifacts=0
const client=new MainSolidWorkerClient(()=>{
 const worker=new Worker(new URL('./check-cad-whole-wall-worker-entry.mjs',import.meta.url),{workerData:{expectedArtifact:{sha256,byteLength:wasm.length}}})
 exits.push(new Promise(resolve=>worker.once('exit',resolve)))
 const port:MainSolidPort={onmessage:null,onerror:null,onmessageerror:null,
  postMessage(request){worker.postMessage(request)},terminate(){void worker.terminate()}}
 worker.on('message',data=>{
  if(data?.artifact){assert.deepEqual(data.artifact,{sha256,byteLength:wasm.length});checkedWorkerArtifacts++;return}
  if(!data?.entered)port.onmessage?.({data} as MessageEvent)
 })
 worker.on('error',error=>port.onerror?.({message:error.message} as ErrorEvent))
 worker.on('messageerror',()=>port.onmessageerror?.({} as MessageEvent))
 return port
})
try{
 const actual=await client.run({kind:'profileBridgeChords',options},{timeoutMs:120000})
 assert.deepEqual(actual,fixture.report,'WASM worker must reproduce the complete native report')
 const {op:__,...rationalOptions}=fixture.rationalProposed.request as ProfileBridgeChordOptions&{op:string}
 const rational=await client.run({kind:'profileBridgeChords',options:rationalOptions},{timeoutMs:120000})
 assert.deepEqual(rational,fixture.rationalProposed.report,'Modified rational profiles must preserve native deviation bounds')
 assert.equal(rational.proposedExpression,true);assert.equal(rational.solidQualified,false)

 const {op:___,...diameterOptions}=fixture.normalDiameter.request as ProfileBridgeChordOptions&{op:string}
 const diameter=await client.run({kind:'profileBridgeChords',options:diameterOptions},{timeoutMs:120000})
 assert.deepEqual(diameter,fixture.normalDiameter.report)
 assert.notEqual(diameter.excluded,true,'Do not exclude a genuine normal diameter')
 const refusal=await client.run({kind:'profileBridgeChords',options:{...options,maxControls:0}},{timeoutMs:120000})
 assert.equal(refusal.excluded,null);assert.equal(refusal.spans,0)
 assert.equal(refusal.solidQualified,false);assert.equal(refusal.wholeThicknessQualified,false)
 if(process.argv.includes('--whole-coverage')){
  for(const key of ['wholeCoverage','stoppedCoverage','curvedWholeCoverage']){
   const {op:_,...input}=fixture[key].request as ProfileBridgeChordOptions&{op:string}
   const checked=await client.run({kind:'profileBridgeChords',options:input},{timeoutMs:120000}).catch(error=>{throw new Error(`Whole coverage fixture ${key}: ${error.message}`,{cause:error})})
   assert.deepEqual(checked,fixture[key].report,key)
  }
 }
 const invalid=structuredClone(options)
 invalid.pair[0][0][1]=2;invalid.maxSpans=0
 await assert.rejects(client.run({kind:'profileBridgeChords',options:invalid},{timeoutMs:120000}))
 // Native validation failure must release busy state for the same worker.
 assert.deepEqual(await client.run({kind:'profileBridgeChords',options},{timeoutMs:120000}),actual)
 const canceled=client.run({kind:'profileBridgeChords',options})
 const rejected=assert.rejects(canceled,{name:'AbortError'})
 client.cancel();await rejected
 const retry=await client.run({kind:'profileBridgeChords',options},{timeoutMs:120000})
 assert.deepEqual(retry,actual)
 assert(checkedWorkerArtifacts>=2,'Check both the initial and post-cancellation worker artifacts')
 writeFileSync(process.argv[2],JSON.stringify({checkedWorkerArtifacts,wasm:{sha256,byteLength:wasm.length},nativeParity:true,wholeCoverageParity:process.argv.includes('--whole-coverage'),rationalProfileParity:true,normalDiameterPreserved:true,nativeErrorRecovery:true,zeroBudgetRefusal:true,cancelAndRetry:true,report:actual},null,2)+'\n')
}finally{client.dispose();await Promise.all(exits)}
