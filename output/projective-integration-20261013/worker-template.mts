import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import {wholeWallExpectation,validWholeWall} from '../src/services/solidWholeWall'

assert.ok(process.argv[2],'Provide a qualification output directory')
const directory=resolve(process.argv[2])
const wasmPath='public/wasm/geometry-kernel.wasm'
const hash=()=>createHash('sha256').update(readFileSync(wasmPath)).digest('hex')
const wasmSha256=hash()
const fixtures=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/whole-wall/contract.json','utf8')).cases
const box=fixtures.find((c:any)=>c.name==='box-minimum')
const curved=fixtures.find((c:any)=>c.name==='curved-self-unresolved')
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
try{
  warm=await client.run({kind:'wholeWall',options:box.request})
  assert(validWholeWall(wholeWallExpectation(box.request),warm))
  assert.equal(warm.converged,true)
  const abort=new AbortController()
  const pending=client.run({kind:'wholeWall',options:curved.request},{signal:abort.signal})
  const refused=assert.rejects(pending,(e:any)=>e.name==='AbortError'&&e.code==='CAD_CANCELLED')
  await realms[0].entered
  abort.abort()
  await refused
  assert.equal(realms[0].terminated,true)
  await realms[0].exited
  retry=await client.run({kind:'wholeWall',options:box.request})
  assert.equal(realms.length,2,'Retry must create a fresh actual worker realm')
  assert(validWholeWall(wholeWallExpectation(box.request),retry))
  assert.equal(retry.converged,true)
  assert.deepEqual(retry.intervalMm,warm.intervalMm)
}finally{
  client.dispose()
  await Promise.all(realms.map(r=>r.exited))
}
assert.equal(hash(),wasmSha256,'WASM changed during qualification')
const report={schema:'cad-whole-wall-real-worker/1',passed:true,wasmSha256,
  workersStarted:realms.length,workersJoined:realms.length,warm,retry,
  scope:'Actual Node worker realms and production handler/client: warm native result, abort after dispatch before response admission, terminate/join, and fresh-worker Retry. Dispatch observation does not prove interruption inside a Rust instruction. Browser UI and exact expression-shell admission remain unqualified.'}
mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'worker-report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify({passed:true,workersStarted:realms.length,wasmSha256}))
