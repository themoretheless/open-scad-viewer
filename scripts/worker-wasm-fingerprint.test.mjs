import {test} from 'node:test'
import assert from 'node:assert/strict'
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import path from 'node:path'
import {traceWorkerWasmFingerprint} from './worker-wasm-fingerprint.mjs'

const sha='1'.repeat(64)
async function fixture(run){
 const root=await mkdtemp(path.join(tmpdir(),'osv-worker-provenance-'))
 try{await mkdir(path.join(root,'assets'));await run(root)}finally{await rm(root,{recursive:true,force:true})}
}
test('follows shared and statically named dynamic worker modules',()=>fixture(async root=>{
 await writeFile(path.join(root,'assets/worker.js'),'import "./shared.js"')
 await writeFile(path.join(root,'assets/shared.js'),'export async function load(){return import("./identity.js")}')
 await writeFile(path.join(root,'assets/identity.js'),`export const identity="${sha}"`)
 const trace=await traceWorkerWasmFingerprint(root,'worker.js',sha)
 assert.deepEqual(trace.matchedModules,['assets/identity.js'])
 assert.equal(trace.modules.length,3)
}))
test('rejects a digest in an unreachable asset or a fake import inside a string',()=>fixture(async root=>{
 await writeFile(path.join(root,'assets/worker.js'),'export const text=\'import "./identity.js"\'')
 await writeFile(path.join(root,'assets/identity.js'),`export const identity="${sha}"`)
 await assert.rejects(traceWorkerWasmFingerprint(root,'worker.js',sha),/does not contain/)
}))
test('rejects a stale reachable identity',()=>fixture(async root=>{
 await writeFile(path.join(root,'assets/worker.js'),'import "./identity.js"')
 await writeFile(path.join(root,'assets/identity.js'),`export const identity="${'2'.repeat(64)}"`)
 await assert.rejects(traceWorkerWasmFingerprint(root,'worker.js',sha),/does not contain/)
}))
test('rejects module paths outside the built distribution',()=>fixture(async root=>{
 await writeFile(path.join(root,'assets/worker.js'),'import "../../outside.js"')
 await assert.rejects(traceWorkerWasmFingerprint(root,'worker.js',sha),/escapes/)
}))
