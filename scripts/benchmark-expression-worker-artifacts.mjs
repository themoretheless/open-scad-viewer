import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {pathToFileURL} from 'node:url'
import {Worker} from 'node:worker_threads'

const [baselinePath,candidatePath,requestPath,nativePath,outputPath]=process.argv.slice(2)
assert(outputPath,'Provide baseline stage, candidate stage, saved request, native replay and new output directory')
const output=resolve(outputPath);mkdirSync(output)
const hash=bytes=>createHash('sha256').update(bytes).digest('hex')
const bytes=readFileSync(requestPath),request=JSON.parse(bytes),native=JSON.parse(readFileSync(nativePath,'utf8'))
assert.equal(native.schema,'expression-inspection-native-replay/1')
assert.equal(native.requestSha256,hash(bytes));assert.equal(native.requestBytes,bytes.length)
assert.deepEqual(native.request,request);assert.equal(request.op,'cad_expression_shell_inspect')
const {op,...options}=request
const stages=[]
for(const [name,directory] of [['baseline',baselinePath],['candidate',candidatePath]]){
 const root=resolve(directory),manifestBytes=readFileSync(resolve(root,'runtime-inputs.json'))
 const manifest=JSON.parse(manifestBytes)
 for(const [file,digest] of Object.entries(manifest.files))assert.equal(hash(readFileSync(resolve(root,file))),digest,`Stage changed: ${file}`)
 const wasm=readFileSync(resolve(root,'public/wasm/geometry-kernel.wasm'))
 const {default:identity}=await import(pathToFileURL(resolve(root,'src/generated/geometry-kernels/identity.ts')).href)
 assert.equal(hash(wasm),identity.sha256);assert.equal(wasm.length,identity.byteLength)
 const {MainSolidWorkerClient}=await import(pathToFileURL(resolve(root,'src/services/mainSolidWorkerClient.ts')).href)
 stages.push({name,root,identity,manifest,manifestSha256:hash(manifestBytes),Client:MainSolidWorkerClient})
}
// Pin the entire service graph and worker entry, not just the wrapper API.
for(const [file,digest] of Object.entries(stages[0].manifest.files))if(file.startsWith('src/services/')||file==='scripts/check-cad-whole-wall-worker-entry.mjs')assert.equal(stages[1].manifest.files[file],digest,`Unequal runtime: ${file}`)
const rows=[]
let failure
try{
 for(let round=0;round<3;round++)for(const stage of round%2?[...stages].reverse():stages){
  const artifacts=[],realms=[]
  const client=new stage.Client(()=>{
   const worker=new Worker(pathToFileURL(resolve(stage.root,'scripts/check-cad-whole-wall-worker-entry.mjs')),{workerData:{expectedArtifact:stage.identity}})
   const exited=new Promise(resolve=>worker.once('exit',resolve))
   const port={onmessage:null,onerror:null,onmessageerror:null,
    postMessage:request=>worker.postMessage(request),terminate:()=>{void worker.terminate()}}
   worker.on('message',data=>{
    if(data?.artifact){artifacts.push(data.artifact);return}
    if(data?.entered)return
    port.onmessage?.({data})
   })
   worker.on('error',error=>port.onerror?.({message:error.message}))
   worker.on('messageerror',()=>port.onmessageerror?.({}))
   realms.push({exited});return port
  })
  const timings=[],started=performance.now()
  try{
   const response=await client.run({kind:'expressionShellInspect',options},{timeoutMs:120000,onTiming:t=>timings.push(t)})
   assert.deepEqual(response,native.response,'Complete geometry response changed')
   assert.deepEqual(artifacts,[stage.identity]);assert.equal(timings.length,1)
   const row={round:round+1,artifact:stage.name,sha256:stage.identity.sha256,elapsedMs:performance.now()-started,timing:timings[0],fullNativeParity:true}
   rows.push(row);console.log(JSON.stringify(row))
  }finally{client.dispose();await Promise.all(realms.map(r=>r.exited))}
 }
}catch(e){failure=String(e)}
const report={schema:'expression-worker-artifact-benchmark/1',passed:!failure,failure,requestSha256:hash(bytes),requestBytes:bytes.length,
 artifacts:stages.map(s=>({name:s.name,...s.identity,runtimeManifestSha256:s.manifestSha256})),rows,
 scope:'Three alternating serial rounds per artifact, fresh production Node worker/client each time, complete native response parity on one saved request. Timings include host load and cold worker startup; not browser latency or universal geometry performance.'}
writeFileSync(resolve(output,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'})
assert(!failure,failure)
