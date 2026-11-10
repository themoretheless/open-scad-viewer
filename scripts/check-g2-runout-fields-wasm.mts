import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import artifact from '../src/generated/geometry-kernels/identity'
import * as module from 'node:module'
import {load} from '../tests/fixtures/raw-import-loader.mjs'
import type {MainSolidJob} from '../src/services/mainSolidProtocol'
import type {DirectDocument} from '../src/services/directModeling'
import type {SolidSurfaceBuildOptions} from '../src/kernel/solidSurfaceConstruction'
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const {createG2RunoutSurface,validG2RunoutResult}=await import('../src/services/geometricG2')
const {createMainSolidWorkerHandler}=await import('../src/services/mainSolidWorkerRuntime')
const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
const directory=resolve(process.argv[2]??'output/qualification/runout-fields-wasm');mkdirSync(directory,{recursive:true})
const bytes=readFileSync('public/wasm/geometry-kernel.wasm'),sha256=createHash('sha256').update(bytes).digest('hex')
assert.equal(sha256,artifact.sha256);assert.equal(bytes.length,artifact.byteLength)
const source={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
const plane=structuredClone(source);for(const row of plane.controlPoints)for(const p of row){p[2]=p[1];p[1]=1}
const before=JSON.stringify([source,plane]),cases=[]
for(const [name,width,maxCells] of [['wide',.5,100000],['one-tenth-mm',.1,100000],['exhausted',.5,0]] as const){
 const started=performance.now(),r=createG2RunoutSurface(source,plane,{degree:1,knots:[0,0,1,1],controlPoints:[[0,width],[1,width]],weights:[1,1]},
  {sourceBoundary:'vMax',sourceTrace:{at:1-width,along:[0,1]},transverseUv:[0,1],gains:[width,width],maxCells})
 assert(validG2RunoutResult(r,maxCells,1000000,256));assert.equal(r.report.method,'native-original-support-G2-runout/2')
 assert.equal(r.report.withinTolerance,name==='wide')
 if(name==='one-tenth-mm'){assert(r.report.seamWithinTolerance);assert(r.report.fields.some(f=>f.curvatureDecision==='exceeds'))}
 if(name==='exhausted')assert(!r.report.complete)
 assert.equal(JSON.stringify([source,plane]),before)
 cases.push({name,elapsedMs:performance.now()-started,report:r.report})
}
const messages:any[]=[],handler=createMainSolidWorkerHandler(message=>messages.push(message)),worker=[]
for(const [id,width] of [[1,.5],[2,.1]] as const){
 const document:DirectDocument={version:1,bodies:[],sketches:[],surfaces:[{id:'source',name:'Source',surface:source,segmentsU:4,segmentsV:4},{id:'plane',name:'End',surface:plane,segmentsU:4,segmentsV:4}],
  curves:[{id:'target',name:'Target',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,1,width],[1,1,width]],weights:[1,1]}}]}
 const snapshot=JSON.stringify(document)
 const options:SolidSurfaceBuildOptions={kind:'nurbs-runout',ids:['source','plane','target'],id:'new-runout',reversed:[],sweep:{mode:'translation',normal:[0,0,1],sections:4,maxDeviation:.01},
  runout:{sourceBoundary:'vMax',sourcePosition:1-width,tangentLengthMm:width,flipSide:false,reverse:false}}
 const job:MainSolidJob={kind:'surfaceBuild',document,options}
 await handler({version:1,id,job});const reply=messages.at(-1)
 assert.equal(reply.ok,true);assert(mainSolidResult(mainSolidExpectation(job),reply.result))
 if(width===.5){assert.equal(reply.result.error,'');assert.equal(reply.result.document.surfaces.length,3)}
 else {assert.equal(reply.result.document,null);assert.equal(reply.result.error,'RUNOUT_CURVATURE_LIMIT')}
 assert.equal(JSON.stringify(document),snapshot)
 worker.push({width,result:reply.result})
}
const report={schema:'g2-runout-whole-field-wasm/1',passed:true,wasmSha256:sha256,wasmBytes:bytes.length,cases,worker,
 scope:'Real packaged WASM, TypeScript receipt validation and actual worker-handler surfaceBuild. Dedicated worker cancellation/Retry/late replies are separately covered by controlled-port tests; browser UI and closed body admission are not claimed.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,workerCases:worker.length,wasmSha256:sha256}))
