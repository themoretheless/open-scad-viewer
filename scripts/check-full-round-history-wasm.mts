import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {encodeBinary,decodeBinary} from '../src/core/valueBinaryCodec.ts'
const [wasmPath,nativePath,directory]=process.argv.slice(2)
assert(wasmPath&&nativePath&&directory)
mkdirSync(directory,{recursive:true})
const bytes=readFileSync(wasmPath),nativeBytes=readFileSync(nativePath),native=JSON.parse(nativeBytes.toString('utf8'))
const module=await WebAssembly.compile(bytes);assert.deepEqual(WebAssembly.Module.imports(module),[])
const instance=await WebAssembly.instantiate(module,{}),w=instance.exports as any
function request(value:unknown):any {
 const data=encodeBinary(value),ptr=w.abi_alloc(data.length);assert(ptr)
 new Uint8Array(w.memory.buffer,ptr,data.length).set(data)
 try{const packed=w.abi_request(0,ptr,data.length),p=Number(packed&0xffffffffn),n=Number(packed>>32n)
  try{return decodeBinary(new Uint8Array(w.memory.buffer,p,n))}finally{w.abi_free(p,n)}
 }finally{w.abi_free(ptr,data.length)}
}
const cases:object[]=[]
function run(name:string,value:unknown,ok:boolean):any {
 const start=performance.now(),reply=request(value);assert.equal(reply.ok,ok,`${name}: ${JSON.stringify(reply.error)}`)
 const row={name,ok,elapsedMs:performance.now()-start,error:reply.error};cases.push(row);console.log(JSON.stringify(row));return reply.value
}
const op='cad_full_round_exact_history'
const restored=run('native-history-exact-replay',{op,action:'restore',history:native.history},true)
assert.deepEqual(restored.document,native.history);assert.deepEqual(restored.work,native.work)
const artifact=native.history.revisions[0].artifact
const root=run('create-from-native-root',{op,action:'create',artifact},true)
function transform(scale:number):any {
 const reply=request({op:'brep_nurbs_transform',model:artifact.source,matrix:[[1,0,0,0],[0,1,0,0],[0,0,scale,0],[0,0,0,1]]})
 assert(reply.ok,JSON.stringify(reply.error));return reply.value
}
const source=transform(1.25)
const changed=run('live-source-rebuild',{op,action:'rebase-source',history:root.document,source},true)
assert.equal(changed.cursor,1);assert.equal(changed.revisionCount,2)
assert.deepEqual(changed.selectedSourceFaceIds,root.selectedSourceFaceIds)
assert(changed.document.revisions[1].artifact.geometry.vertices.some((v:any)=>JSON.stringify(v.point[2])==='["15","2"]'))
const undone=run('undo',{op,action:'undo',history:changed.document},true);assert.equal(undone.cursor,0)
const redone=run('redo',{op,action:'redo',history:undone.document},true);assert.deepEqual(redone.document,changed.document)
const branch=run('prefix-branch',{op,action:'branch',history:undone.document},true);assert.equal(branch.revisionCount,1)
const forked=run('branch-source-rebuild',{op,action:'rebase-source',history:branch.document,source:transform(1.5)},true)
assert.notDeepEqual(forked.document,changed.document)
const selected=[3,2,5].map(i=>artifact.source.topologyIds.faces[i])
const reselection=run('authored-id-reselection',{op,action:'reselect',history:root.document,faceIds:selected},true)
assert.equal(reselection.revisionCount,2)
assert.notDeepEqual(reselection.document.revisions[1].artifact.geometry,artifact.geometry)
run('edit-after-undo-requires-branch',{op,action:'rebase-source',history:undone.document,source},false)
const bad=structuredClone(undone.document);bad.revisions[1].artifact.geometry.vertices[0].point[0]=['999','1']
run('forged-hidden-redo',{op,action:'restore',history:bad},false)
const wrongTag=structuredClone(changed.document);wrongTag.revisions[1].edit='reselect'
run('inconsistent-operation-tag',{op,action:'restore',history:wrongTag},false)
run('no-caller-proof-credit',{op,action:'restore',history:root.document,work:{pairProjections:0}},false)
run('caller-material-limit',{op,action:'create',artifact,budget:{material:{maxPairProjections:0}}},false)
run('caller-byte-limit',{op,action:'restore',history:root.document,budget:{maxBytes:1}},false)
const regenerated=run('regenerate-earlier-selection',{op,action:'regenerate-selection',history:undone.document,revision:0,faceIds:selected},true)
assert.equal(regenerated.cursor,0);assert.equal(regenerated.revisionCount,2)
assert.deepEqual(regenerated.document.revisions[0].artifact.geometry,reselection.document.revisions[1].artifact.geometry)
assert.notDeepEqual(regenerated.document.revisions[1].artifact.geometry,changed.document.revisions[1].artifact.geometry)
const regeneratedRedo=run('redo-regenerated-descendant',{op,action:'redo',history:regenerated.document},true)
assert.deepEqual(regeneratedRedo.selectedSourceFaceIds,selected)
run('source-step-has-no-selection-parameter',{op,action:'regenerate-selection',history:undone.document,revision:1,faceIds:selected},false)
assert.deepEqual(root.document.revisions[0].artifact,artifact)
const report={passed:true,cases,wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,
 nativeFixtureSha256:createHash('sha256').update(nativeBytes).digest('hex'),scope:'Actual shared ABI: frozen native history, live feature edits, undo/redo/fork and refusal. Not editor-history integration or general history parity.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
writeFileSync(resolve(directory,'native-fixture.json'),nativeBytes)
writeFileSync(resolve(directory,'wasm-history.json'),JSON.stringify(changed.document)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,wasmSha256:report.wasmSha256}))
