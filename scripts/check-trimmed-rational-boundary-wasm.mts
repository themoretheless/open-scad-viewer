import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,mkdirSync,writeFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {resolve} from 'node:path'
import * as module from 'node:module'
import {load} from '../tests/fixtures/raw-import-loader.mjs'
import identity from '../src/generated/geometry-kernels/identity'
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const {callNurbsRust}=await import('../src/kernel/geometry/nurbs')
const {unpackBrotliWasmBase64}=await import('../src/kernel/wasmBrotliPacking')
const packed=(await import('../src/generated/geometry-kernels/bytes')).default
const wasm=readFileSync('public/wasm/geometry-kernel.wasm'),sha=createHash('sha256').update(wasm).digest('hex')
assert.equal(sha,identity.sha256);assert.equal(wasm.length,identity.byteLength)
assert.equal(createHash('sha256').update(unpackBrotliWasmBase64(packed)).digest('hex'),sha)
const model=JSON.parse(gunzipSync(readFileSync('tests/fixtures/trimmed-rational-cube.json.gz')).toString()),before=JSON.stringify(model)
const volume={maxExactWork:1000000,maxTrimPairs:100,maxTrimCells:100000,maxTrimDomainCells:1000000,maxSpans:128,maxPairs:100,maxCells:100000,maxDomainCells:1000000,cellsPerPair:10000,domainCellsPerPair:100000,nestingPairs:100,nestingCells:100000,nestingDomainCells:1000000,orientationCells:100000,orientationDomainCells:1000000,orientationSpans:128}
const limits={correspondenceCells:10000,correspondenceProducts:1000000,regularityCells:100000,linearCellsPerBoundary:10000,referenceVolume:volume,retainedVolume:volume}
const start=performance.now()
const report=callNurbsRust<any>('brep_boundary_certificate',{reference:model,retained:model,toleranceUv:1e-9,maxDeviation:1e-6,limits})
assert.equal(JSON.stringify(model),before)
const directory=resolve(process.argv[2]??'output/qualification/trimmed-rational-boundary-wasm');mkdirSync(directory,{recursive:true})
const receipt={schema:'trimmed-rational-boundary-wasm/1',passed:false,wasmSha256:sha,wasmBytes:wasm.length,embeddedArtifactHashVerified:true,elapsedMs:performance.now()-start,sourceUnchanged:true,alteredWeightsRefused:false,partialWindowsRefusedAsSolid:false,warmMs:[] as number[],scope:'Original rational control body. Exact boundary agreement and whole certificate status are distinct; no fillet or G2 admission.',report}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(receipt,null,2)+'\n')
assert.equal(report.certified,true)
assert.equal(report.referenceVolume?.proven,true)
assert.equal(report.retainedVolume?.proven,true)
assert.equal(report.referenceVolume?.exactBoundary,true)
assert.equal(report.retainedVolume?.exactBoundary,true)
assert.equal(report.smoothnessCertified,false)
const changed=structuredClone(model);changed.faces[0].surface.weights[1][0]+=0.125
assert.throws(()=>callNurbsRust('brep_boundary_certificate',{reference:changed,retained:changed,toleranceUv:1e-9,maxDeviation:1e-6,limits}))
receipt.alteredWeightsRefused=true
const partial=structuredClone(model)
for(const wire of partial.loops)for(const c of wire.coedges){c.trim.curveParameter=[0.125,0.75];c.trim.pcurveParameter=[0.125,0.75]}
const partialReport=callNurbsRust<any>('brep_boundary_certificate',{reference:partial,retained:partial,toleranceUv:1e-9,maxDeviation:1e-6,limits})
assert.equal(partialReport.certified,false)
assert.equal(partialReport.referenceVolume?.exactBoundary,false)
receipt.partialWindowsRefusedAsSolid=true
for(let i=0;i<3;i++) {const start=performance.now();const r=callNurbsRust<any>('brep_boundary_certificate',{reference:model,retained:model,toleranceUv:1e-9,maxDeviation:1e-6,limits});receipt.warmMs.push(performance.now()-start);assert.equal(r.certified,true)}
receipt.passed=true
writeFileSync(resolve(directory,'report.json'),JSON.stringify(receipt,null,2)+'\n')
console.log(JSON.stringify({passed:true,wasmSha256:sha,certified:report.certified,reason:report.reason}))
