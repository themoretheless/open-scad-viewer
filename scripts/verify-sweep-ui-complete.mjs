import assert from 'node:assert/strict'
import {readFile,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import {sweepBodyBoundarySources} from './sweep-body-matrix-sources.mjs'
const [input,output]=process.argv.slice(2)
assert.ok(input&&output,'Supply one complete body UI matrix and output receipt')
const sha=b=>createHash('sha256').update(b).digest('hex'),bytes=await readFile(input),matrix=JSON.parse(bytes)
assert.equal(matrix.passed,true);assert.equal(matrix.headed,false);assert.equal(matrix.evidenceOnly,false)
assert.equal(matrix.nativeRunningCancelProbe,true);assert.equal(matrix.gpuBackendRequested,'metal')
const expected=[1440,600].flatMap(width=>sweepBodyBoundarySources.map(file=>`${width}:${file}`))
assert.deepEqual(matrix.selection.fullInventory,expected);assert.deepEqual(matrix.selection.plannedCaseKeys,expected)
assert.deepEqual(matrix.cases.map(c=>`${c.viewport.width}:${c.file}`),expected)
assert.equal(new Set(expected).size,expected.length)
assert.equal(matrix.geometryWasmSha256,sha(await readFile('public/wasm/geometry-kernel.wasm')))
assert.equal(matrix.distIndexSha256,sha(await readFile('dist/index.html')))
const currentSourceDrift=[]
for(const [file,expectedSha] of Object.entries(matrix.sourceHashes)){
 const actual=sha(await readFile(file));if(actual!==expectedSha)currentSourceDrift.push({file,qualifiedSha256:expectedSha,currentSha256:actual})
}
const names=['native-build-cancellation','native-build-source-change','native-solid-cancel','native-solid-source-change']
const observations=[]
for(const c of matrix.cases){
 assert.equal(c.status,'passed');assert.deepEqual(c.pageErrors,[])
 assert.equal(c.sourceSha256,sha(await readFile(`examples/rush/${c.file}`)))
 assert.equal(c.gpuAdapter.vendor,'apple');assert.ok(c.gpuAdapter.architecture.startsWith('metal'))
 assert.ok(c.assertions.length>=23);assert.ok(c.assertions.every(a=>a.status==='passed'))
 for(const name of ['build','solid-publication','restore','invalid-path-build-refusal','invalid-path-solid-refusal','bounded-work-refusal'])assert.ok(c.assertions.some(a=>a.case===name),`${c.file}: missing ${name}`)
 assert.equal(c.solid.renderer,'webgpu');assert.ok(c.solid.faces>0&&c.solid.shells>0)
 assert.match(c.solid.geometrySha256,/^[a-f0-9]{64}$/)
 for(const name of names){
  const matches=c.assertions.filter(a=>a.case===name);assert.equal(matches.length,1)
  const a=matches[0],stack=a.nativeStack;assert.equal(stack.geometryWasmSha256,matrix.geometryWasmSha256)
  assert.ok(stack.nativeSampleCount>=5);assert.ok(Number.isFinite(stack.lastSampleToProfileEndMs)&&stack.lastSampleToProfileEndMs>=0&&stack.lastSampleToProfileEndMs<=5)
  assert.equal(stack.debuggerPaused,false);assert.ok(Number.isFinite(a.workerDestroyedAfterMs)&&a.workerDestroyedAfterMs>=0)
  observations.push({width:c.viewport.width,file:c.file,case:name,nativeSamples:stack.nativeSampleCount,tailMs:stack.lastSampleToProfileEndMs,workerDestroyedAfterMs:a.workerDestroyedAfterMs})
 }
}
assert.equal(observations.length,expected.length*4)
await writeFile(output,JSON.stringify({schema:'sweep-ui-complete-receipt/1',passed:true,geometryWasmSha256:matrix.geometryWasmSha256,reportSha256:sha(bytes),cases:matrix.cases.length,widths:[1440,600],assertions:matrix.cases.reduce((n,c)=>n+c.assertions.length,0),nativeObservations:observations.length,minNativeSamples:Math.min(...observations.map(a=>a.nativeSamples)),maxTailMs:Math.max(...observations.map(a=>a.tailMs)),allWorkersDestroyed:true,headed:false,gpuBackend:'apple-metal',currentSourceDrift,observations,scope:'Complete finite current124 body UI inventory, exact current main WASM/dist and captured sources; actual unpaused Worker destruction with authenticated native CPU tail, not cooperative kernel-polling latency or another artifact qualification'},null,2)+'\n')
console.log(`${matrix.cases.length}/${expected.length} body UI cases; ${observations.length} authenticated native lifecycle observations verified`)
