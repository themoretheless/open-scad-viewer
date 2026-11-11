import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {resolve} from 'node:path'
import * as module from 'node:module'
import {load} from '../tests/fixtures/raw-import-loader.mjs'
import identity from '../src/generated/geometry-kernels/identity'
import type {MainSolidJob} from '../src/kernel/mainSolidProtocol'
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const {restoreFrozenGraphBody,displayFrozenGraphBody}=await import('../src/kernel/frozenGraphBody')
const {createMainSolidWorkerHandler}=await import('../src/kernel/mainSolidWorkerRuntime')
const {mainSolidExpectation,mainSolidResult}=await import('../src/kernel/mainSolidProtocol')
const bytes=readFileSync('public/wasm/geometry-kernel.wasm'),sha256=createHash('sha256').update(bytes).digest('hex')
assert.equal(sha256,identity.sha256);assert.equal(bytes.length,identity.byteLength)
const packed=(await import('../src/generated/geometry-kernels/bytes')).default
const {unpackBrotliWasmBase64}=await import('../src/kernel/wasmBrotliPacking')
const embedded=unpackBrotliWasmBase64(packed)
assert.equal(embedded.length,bytes.length)
assert.equal(createHash('sha256').update(embedded).digest('hex'),sha256)
const artifact=JSON.parse(gunzipSync(readFileSync(process.argv[3]??'tests/fixtures/frozen-graph-body.json.gz')).toString()),before=JSON.stringify(artifact)
const options={maxWork:10000000,maxBytes:4194304,maxPairs:100}
const start=performance.now(),direct=restoreFrozenGraphBody(artifact,options),directMs=performance.now()-start
assert.equal(direct.report.faces,9);assert.equal(direct.report.pairProducts,36)
assert.equal(direct.report.nativeVolumeCertified,true);assert.equal(direct.report.applicationAdmitted,false)
assert.deepEqual(direct.artifact.geometry,artifact.geometry)
const replies:any[]=[],handler=createMainSolidWorkerHandler(reply=>replies.push(reply))
const job:MainSolidJob={kind:'frozenGraphBodyRestore',artifact,options}
const workerStart=performance.now();await handler({version:1,id:1,job});const workerMs=performance.now()-workerStart
const reply=replies.at(-1);assert.equal(reply.ok,true);assert(mainSolidResult(mainSolidExpectation(job),reply.result));assert.deepEqual(reply.result,direct)
const corrupted=structuredClone(artifact);corrupted.geometry.identityLinks.edges[0].sourceId='changed'
assert.throws(()=>restoreFrozenGraphBody(corrupted,options))
await handler({version:1,id:2,job:{...job,options:{...options,maxWork:0}}});assert.equal(replies.at(-1).ok,false)
const display=displayFrozenGraphBody(artifact,8,options)
assert.equal(display.display.edges.length,artifact.geometry.edges.length);assert.equal(display.display.polylineDeviationCertified,false)
assert(display.display.edges.every(e=>e.points.length===9&&e.root===artifact.geometry.edges[e.edge].root))
const displayJob:MainSolidJob={kind:'frozenGraphBodyDisplay',artifact,segments:8,options}
await handler({version:1,id:3,job:displayJob});const displayReply=replies.at(-1)
assert.equal(displayReply.ok,true);assert(mainSolidResult(mainSolidExpectation(displayJob),displayReply.result));assert.deepEqual(displayReply.result,display)
assert.equal(JSON.stringify(artifact),before)
const directory=resolve(process.argv[2]??'output/qualification/frozen-graph-body-wasm');mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'report.json'),JSON.stringify({schema:'frozen-graph-body-wasm/1',passed:true,embeddedArtifactHashVerified:true,artifactFormat:artifact.format,naturalRootCuts:artifact.recipe.naturalCuts?.length??0,wasmSha256:sha256,wasmBytes:bytes.length,directMs,workerMs,report:direct.report,displayEdges:display.display.edges.length,displaySegments:8,sampleStorageErrorUpperMm:display.display.sampleStorageErrorUpperMm,displayWorkerVerified:true,geometryAndIdentityLinksEqual:true,corruptedIdentityRefused:true,zeroWorkRefused:true,sourceUnchanged:true,scope:'Real packaged WASM and actual worker handler on nine-face control body. No UI publication, general NURBS, radius-law, thickness or STEP/history admission.'},null,2)+'\n')
console.log(JSON.stringify({passed:true,wasmSha256:sha256,faces:direct.report.faces,directMs,workerMs}))
