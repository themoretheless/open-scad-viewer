import artifactIdentity from '../src/generated/geometry-kernels/identity'
import {verifyPackedWasmChunk} from './verify-packed-wasm.mjs'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import {expressionShellExpectation,validExpressionShell} from '../src/services/expressionShell'

assert.ok(process.argv[2],'Provide a qualification output directory')
const directory=resolve(process.argv[2])
const timeoutArgument=process.argv.find(value=>value.startsWith('--timeout-ms='))
const timeoutMs=timeoutArgument?Number(timeoutArgument.slice('--timeout-ms='.length)):120000
assert(Number.isSafeInteger(timeoutMs)&&timeoutMs>=1&&timeoutMs<=120000,'Bound qualification timeout to 1..120000 ms')
const wasmPath='public/wasm/geometry-kernel.wasm'
const hash=()=>createHash('sha256').update(readFileSync(wasmPath)).digest('hex')
const wasmSha256=hash()
assert.equal(artifactIdentity.sha256,wasmSha256,'Packed identity must match published WASM')
assert.equal(artifactIdentity.byteLength,readFileSync(wasmPath).length)
verifyPackedWasmChunk(readFileSync('src/generated/geometry-kernels/bytes.ts','utf8'),readFileSync(wasmPath),'expression shell worker package')
const {op:_,...options}=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-replay-request-2026-10-09.json','utf8'))
if(process.argv.includes('--neighbor-tangency')){
  const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json','utf8'))
  options.neighborTangencyCheck=fixture[process.argv.includes('--neighbor-tangency-unproven')?'unproven':'admitted'].request.neighborTangencyCheck
}
const refusedOptions=structuredClone(options);refusedOptions.profileLimits.controls=0
const realms:{port:MainSolidPort;entered:Promise<void>;exited:Promise<number>;terminated:boolean}[]=[]
const client=new MainSolidWorkerClient(()=>{
  const worker=new Worker(new URL('./check-cad-whole-wall-worker-entry.mjs',import.meta.url))
  let entered!:()=>void
  const realm={port:null as unknown as MainSolidPort,entered:new Promise<void>(resolve=>{entered=resolve}),
    exited:new Promise<number>(resolve=>worker.once('exit',resolve)),terminated:false}
  const port:MainSolidPort={onmessage:null,onerror:null,onmessageerror:null,
    postMessage(request){
      realm.entered=new Promise<void>(resolve=>{entered=resolve})
      worker.postMessage(request)
    },
    terminate(){realm.terminated=true;void worker.terminate()},
  }
  worker.on('message',data=>{
    if(data?.entered){entered();return}
    port.onmessage?.({data} as MessageEvent)
  })
  worker.on('error',error=>port.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>port.onmessageerror?.({} as MessageEvent))
  realm.port=port;realms.push(realm);return port
})
let warm:any,retry:any
let qualificationPhase='budget-refusal'
const qualificationStarted=performance.now()
try{
  warm=await client.run({kind:'expressionShellRestore',options:refusedOptions},{timeoutMs})
  assert(validExpressionShell(expressionShellExpectation(refusedOptions),warm))
  assert.equal(warm.incidenceQualified,false)
  qualificationPhase='dispatch-cancellation'
  const abort=new AbortController()
  const pending=client.run({kind:'expressionShellRestore',options},{signal:abort.signal,timeoutMs})
  const refused=assert.rejects(pending,(e:any)=>e.name==='AbortError'&&e.code==='CAD_CANCELLED')
  await realms[0].entered;abort.abort();await refused
  assert.equal(realms[0].terminated,true);await realms[0].exited
  qualificationPhase='fresh-worker-retry'
  retry=await client.run({kind:'expressionShellRestore',options},{timeoutMs})
  assert.equal(realms.length,2)
  assert(validExpressionShell(expressionShellExpectation(options),retry))
  assert.equal(retry.incidenceQualified,true,retry.reason)
  if(options.neighborTangencyCheck){
    assert.equal(retry.shell.neighborTangencyCheck.qualified,!process.argv.includes('--neighbor-tangency-unproven'))
    assert.equal(retry.shell.neighborTangencyCheck.solidQualified,false)
  }
  assert.equal(retry.shell.faceCount,29);assert.equal(retry.shell.components,1)
  assert.equal(retry.embeddingQualified,false);assert.equal(retry.materialQualified,false)
}catch(error){
  mkdirSync(directory,{recursive:true})
  writeFileSync(resolve(directory,'expression-worker-failure.json'),JSON.stringify({
    schema:'expression-worker-qualification-failure/1',passed:false,phase:qualificationPhase,
    elapsedMs:performance.now()-qualificationStarted,timeoutMs,wasmSha256,
    wasmUnchanged:hash()===wasmSha256,workersStarted:realms.length,
    error:error instanceof Error?{name:error.name,message:error.message,code:(error as {code?:string}).code}:{message:String(error)},
    scope:'Failed qualification only; no geometry or latency admission',
  },null,2)+'\n',{flag:'wx'})
  throw error
}finally{
  client.dispose();await Promise.all(realms.map(r=>r.exited))
}
assert.equal(hash(),wasmSha256,'WASM changed during qualification')
const report={schema:'expression-shell-real-worker/1',passed:true,wasmSha256,
 workersStarted:realms.length,workersJoined:realms.length,warm,retry,
 scope:'Actual Node worker production handler/client: native budget refusal, dispatch cancellation, terminated realm, fresh-worker Retry of annular recipe. Closed incidence only; material, embedding, browser preview and general fillets remain unqualified.'}
mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'expression-worker-report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify({passed:true,workersStarted:realms.length,wasmSha256,faces:retry.shell.faceCount}))
