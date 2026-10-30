import '../src/mcp/registerRawSourceImports.mjs'
import {parentPort,workerData} from 'node:worker_threads'
import assert from 'node:assert/strict'
import {register} from 'tsx/esm/api'
import {setFlagsFromString} from 'node:v8'

// Match the existing disposable Node worker's termination workaround.
if(process.versions.node.startsWith('20.')&&process.arch==='arm64')setFlagsFromString('--liftoff-only')
register()
const {createMainSolidWorkerHandler}=await import('../src/services/mainSolidWorkerRuntime.ts')
// Bind qualification to the identity imported by this worker's kernel module,
// including replacement workers created by Cancel/Retry during publication.
if(workerData?.expectedArtifact){
 const {default:identity}=await import('../src/generated/geometry-kernels/identity.ts')
 assert.deepEqual(identity,workerData.expectedArtifact,'Worker loaded a different WASM publication')
 parentPort.postMessage({artifact:identity})
}
const handler=createMainSolidWorkerHandler((response,transfer)=>parentPort.postMessage(response,transfer))
parentPort.on('message',request=>{
  // Separate observation of dispatch from the production response protocol.
  parentPort.postMessage({entered:request.id})
  void handler(request)
})
