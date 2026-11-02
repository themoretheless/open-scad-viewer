import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import * as module from 'node:module'
import {load} from '../tests/fixtures/raw-import-loader.mjs'
import artifact from '../src/generated/geometry-kernels/identity'
import type {MainSolidJob} from '../src/services/mainSolidProtocol'
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const {createSourceGraphG2Preview}=await import('../src/services/sourceGraphG2Preview')
const {createMainSolidWorkerHandler}=await import('../src/services/mainSolidWorkerRuntime')
const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
const bytes=readFileSync('public/wasm/geometry-kernel.wasm'),sha256=createHash('sha256').update(bytes).digest('hex')
assert.equal(sha256,artifact.sha256);assert.equal(bytes.length,artifact.byteLength)
const sources:[any,any]=[0,1].map(side=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[0,1].map(x=>[[x,0,0],[x,side===0?-1:1,-1/1024]]),weights:[[1,1],[1,1]]})) as [any,any]
const width={degree:1,knots:[0,0,1,1],controlPoints:[[1/1024,0,0],[1/1024,0,0]],weights:[1,1]}
const options={radiusMm:1,radiusToleranceMm:.001,maxWork:1000000,maxCells:1000}
const before=JSON.stringify([sources,width]);const direct=createSourceGraphG2Preview(sources,width,options)
assert.equal(direct.report.curvatureDecision,'within');assert.equal(direct.report.radiusDecision,'exceeds')
assert.equal(direct.report.applicationAdmitted,false);assert.equal(direct.report.solidGeometryCertified,false)
const messages:any[]=[];const handler=createMainSolidWorkerHandler(reply=>messages.push(reply))
const job:MainSolidJob={kind:'sourceGraphG2Preview',sources,width,options}
await handler({version:1,id:1,job})
const reply=messages.at(-1);assert.equal(reply.ok,true);assert(mainSolidResult(mainSolidExpectation(job),reply.result))
assert.deepEqual(reply.result,direct)
await handler({version:1,id:2,job:{...job,options:{...options,maxWork:0}}})
assert.equal(messages.at(-1).ok,false)
assert.equal(JSON.stringify([sources,width]),before)
const directory=resolve(process.argv[2]??'output/qualification/source-graph-g2-preview-wasm');mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'report.json'),JSON.stringify({schema:'source-graph-g2-preview-wasm/1',passed:true,wasmSha256:sha256,wasmBytes:bytes.length,direct,worker:reply,exhausted:messages.at(-1),scope:'Real packaged WASM and actual worker handler. Rounded open-surface preview only; no closed-body transport, UI interaction, general NURBS or fillet admission.'},null,2)+'\n')
console.log(JSON.stringify({passed:true,wasmSha256:sha256,radiusDecision:direct.report.radiusDecision,worker:true}))
