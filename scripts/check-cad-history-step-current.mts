/** Re-export archived UI histories using the current Rust WASM; no UI replay. */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import {brotliDecompressSync} from 'node:zlib'
import identity from '../src/generated/geometry-kernels/identity'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {importDirectStepV9} from '../src/services/cadNurbsStep'
import {DirectHistory,parseDirectDocument,serializeDirectDocument,type DirectDocument} from '../src/services/directModeling'

const replayHistory=process.argv.includes('--replay-history')
const [sourceArg,outputArg]=process.argv.slice(2).filter(v=>v!=='--replay-history')
assert.ok(sourceArg&&outputArg,'Usage: history-evidence-root new-output-directory')
const source=resolve(sourceArg),output=resolve(outputArg)
await mkdir(output) // Refuse overwriting an earlier qualification run.
const hash=(bytes:string|Uint8Array)=>createHash('sha256').update(bytes).digest('hex')
const wasm=await readFile('public/wasm/geometry-kernel.wasm')
assert.equal(hash(wasm),identity.sha256)
assert.equal(wasm.length,identity.byteLength)
const audit=JSON.parse(await readFile(resolve(source,'history/history-audit.json'),'utf8'))
assert.equal(audit.archive.encoding,'brotli')
const packed=await readFile(resolve(source,'history',audit.archive.file))
assert.equal(hash(packed),audit.archive.sha256)
const archive=new Map<string,string>()
for(const line of brotliDecompressSync(packed).toString('utf8').trim().split('\n')){
 const record=JSON.parse(line)
 assert.equal(hash(record.json),record.sha256)
 archive.set(record.sha256,record.json)
}
await warmGeometryKernel()
const parts=[],checks=[]
for(const interaction of ['mouse','keyboard']){
 const reference=JSON.parse(await readFile(resolve(source,interaction+'-step/manifest.json'),'utf8'))
 for(const name of ['bracket','flange','enclosure']){
  const entry=audit.files.find((v:{file:string})=>v.file.endsWith(`-${interaction}/${name}-20.json`))
  assert.ok(entry,`Missing ${interaction}/${name} final state`)
  const json=archive.get(entry.sha256)
  assert.ok(json)
  assert.equal(Buffer.byteLength(json),entry.bytes)
  let historyReplay:unknown=null
  if(replayHistory){
   const states=Array.from({length:21},(_,i)=>{
    const entry=audit.files.find((v:{file:string})=>v.file.endsWith(`-${interaction}/${name}-${i}.json`))
    assert.ok(entry,`Missing history state ${interaction}/${name}/${i}`)
    const text=archive.get(entry.sha256);assert.ok(text)
    assert.equal(Buffer.byteLength(text),entry.bytes)
    return serializeDirectDocument(parseDirectDocument(text))
   })
   const history=new DirectHistory(parseDirectDocument(states[0]!))
   let canceledLoads=0
   for(let i=1;i<states.length;i++){
    history.commit(parseDirectDocument(states[i]!))
    assert.equal(history.snapshotKey,states[i])
    if((i-1)%3===0){
     const before=history.snapshotKey,stacks=history.storageStats
     let resolveLoad!:(value:DirectDocument)=>void
     const pending=history.commitAsync(()=>new Promise(resolve=>{resolveLoad=resolve}))
     history.cancelRestore();resolveLoad(parseDirectDocument(states[0]!))
     assert.equal(await pending,false,'Canceled document load must not commit late geometry')
     assert.equal(history.snapshotKey,before);assert.deepEqual(history.storageStats,stacks)
     canceledLoads++
    }
   }
   assert.equal(history.storageStats.undoStates,20,'Keep all twenty specified edits')
   for(let i=19;i>=0;i--){
    assert.equal(await history.restoreAsync('undo',async text=>parseDirectDocument(text)),true)
    assert.equal(history.snapshotKey,states[i])
   }
   assert.equal(history.canUndo,false)
   for(let i=1;i<=20;i++){
    assert.equal(await history.restoreAsync('redo',async text=>parseDirectDocument(text)),true)
    assert.equal(history.snapshotKey,states[i])
   }
   assert.equal(history.canRedo,false)
   assert.equal(serializeDirectDocument(parseDirectDocument(serializeDirectDocument(history.document))),states[20])
   historyReplay={commits:20,undo:20,redo:20,canceledLoads,reloadedExact:true,
    scope:'Current parser/history replay of archived geometry states; commands and browser input are not re-executed.'}
  }
  const document=JSON.parse(json) as DirectDocument
  const body=document.bodies.find(v=>v.id===name)
  assert.ok(body?.brep)
  const before=JSON.stringify(document)
  const first=await exportSolidStepCurrent(body)
  assert.equal(JSON.stringify(document),before,'Export must preserve source document')
  const imported=importDirectStepV9(first)
  if(!imported.identity.preserved){
   await writeFile(resolve(output,'identity-diagnostic.json'),JSON.stringify({interaction,name,
    sourceDocumentSha256:entry.sha256,wasmIdentity:identity,identity:imported.identity,
    sourceTopologyIds:body.brep.topologyIds,importedTopologyIds:imported.model.topologyIds},null,2)+'\n',{flag:'wx'})
   await writeFile(resolve(output,'identity-failed.step'),first,{flag:'wx'})
  }
  assert.equal(imported.identity.preserved,true)
  assert.equal(imported.identity.lostCount,0)
  for(const kind of ['vertices','edges','loops','faces','shells','bodies'] as const){
   assert.deepEqual([...imported.model.topologyIds![kind]].sort(),[...body.brep.topologyIds![kind]].sort(),`STEP ${kind} identities`)
  }
  const second=await exportSolidStepCurrent({...body,brep:imported.model})
  const expected=reference.parts.find((v:{name:string})=>v.name===name+'-ui20-0')?.expected
  assert.ok(expected)
  for(const [cycle,text] of [first,second].entries()){
   const part=`${interaction}-${name}-current-${cycle}`,file=part+'.step'
   await writeFile(resolve(output,file),text,{flag:'wx'})
   parts.push({name:part,file,sha256:hash(text),expected})
  }
  checks.push({interaction,name,sourceDocumentSha256:entry.sha256,sourceUnchanged:true,
   stepIdentity:imported.identity,topologyIdentitySetsEqual:true,historyReplay})
 }
}
await writeFile(resolve(output,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',
 toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n',{flag:'wx'})
await writeFile(resolve(output,'transport-report.json'),JSON.stringify({schema:'cad-history-current-step/1',
 passed:true,wasmIdentity:identity,historyArchiveSha256:audit.archive.sha256,checks,
 scope:replayHistory?'Fresh Rust WASM STEP export/import/re-export plus current document/history replay of archived geometry states. Current browser commands and general fillets are unproven.':'Fresh Rust WASM STEP export/import/re-export of archived final UI documents. Current UI/history execution and general fillet qualification are unproven.'},null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify({passed:true,files:parts.length,wasmSha256:identity.sha256}))
