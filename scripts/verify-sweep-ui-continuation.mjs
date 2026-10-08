import {sweepBodyBoundarySources} from './sweep-body-matrix-sources.mjs'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFile,writeFile} from 'node:fs/promises'

const args=process.argv.slice(2)
assert.ok(args.length>=3,'Supply ordered reports followed by output report')
const outputPath=args.at(-1),inputPaths=args.slice(0,-1)
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const reports=await Promise.all(inputPaths.map(async file=>{
 const bytes=await readFile(file)
 return {file,sha256:sha(bytes),report:JSON.parse(bytes)}
}))
const inputs=reports.map(r=>r.report)
const next=inputs.at(-1)
assert.equal(next.passed,true,'Continuation must finish successfully')
const inventory=next.selection.fullInventory
const planned=next.selection.plannedCaseKeys
const expectedInventory=[1440,600].flatMap(width=>sweepBodyBoundarySources.map(file=>`${width}:${file}`))
assert.deepEqual(inventory,expectedInventory,'Expected complete current body-source/two-width inventory')
assert.equal(new Set(inventory).size,expectedInventory.length)
assert.deepEqual(planned,inventory.slice(inventory.indexOf(next.selection.startAt)))
const harness='scripts/check-sweep-miter-matrix-browser.mjs'
const currentSourceDrift=[]
for(const file of Object.keys(next.sourceHashes)){
 const currentSha256=sha(await readFile(file))
 if(next.sourceHashes[file]!==currentSha256)currentSourceDrift.push({file,qualifiedSha256:next.sourceHashes[file],currentSha256})
}
const key=c=>`${c.viewport.width}:${c.file}`
const cases=[]
for(let i=0;i<inputs.length;i++){
 const r=inputs[i]
 for(const field of ['geometryWasmSha256','appSourceSha256','distIndexSha256','browser'])assert.equal(r[field],next[field],`Changed ${field}`)
 assert.deepEqual(r.artifactProvenance,next.artifactProvenance)
 assert.deepEqual(Object.keys(r.sourceHashes).sort(),Object.keys(next.sourceHashes).sort())
 for(const file of Object.keys(next.sourceHashes))if(file!==harness)assert.equal(r.sourceHashes[file],next.sourceHashes[file],`Changed ${file}`)
 assert.deepEqual(r.selection.fullInventory,inventory)
 assert.deepEqual(r.selection.plannedCaseKeys,inventory.slice(cases.length),'Each continuation must resume exactly at the first omitted case')
 const successes=r.cases.filter(c=>c.status==='passed')
 assert.deepEqual(successes.map(key),inventory.slice(cases.length,cases.length+successes.length),'Input successes must be a contiguous prefix without duplicates or skipped cases')
 if(i===inputs.length-1)assert.deepEqual(r.cases.map(key),r.selection.plannedCaseKeys)
 cases.push(...successes)
 assert.equal(r.nativeRunningCancelProbe,true)
 assert.equal(r.headed,false)
 assert.equal(r.evidenceOnly,false)
 assert.equal(r.gpuBackendRequested,'metal')
}
assert.deepEqual(cases.map(key),inventory,'Complete inventory must run exactly once')
for(const c of cases){
 assert.equal(c.status,'passed')
 assert.deepEqual(c.pageErrors,[])
 assert.equal(c.sourceSha256,sha(await readFile(`examples/rush/${c.file}`)))
 assert.equal(c.gpuAdapter.vendor,'apple')
 assert.ok(c.gpuAdapter.architecture.startsWith('metal'))
 assert.ok(c.assertions.length>=23)
 assert.ok(c.assertions.every(a=>a.status==='passed'))
 for(const name of ['native-build-cancellation','native-build-source-change','native-solid-cancel','native-solid-source-change']){
  const a=c.assertions.find(a=>a.case===name)
  assert.ok(a,`Missing ${key(c)} ${name}`)
  assert.equal(a.nativeStack.geometryWasmSha256,next.geometryWasmSha256)
  assert.ok(a.nativeStack.nativeSampleCount>=5)
  assert.ok(a.nativeStack.lastSampleToProfileEndMs<=5)
  assert.equal(a.nativeStack.debuggerPaused,false)
  assert.ok(Number.isFinite(a.workerDestroyedAfterMs)&&a.workerDestroyedAfterMs>=0)
 }
}
await writeFile(outputPath,JSON.stringify({schema:'sweep-ui-continuation-union/1',passed:true,
 geometryWasmSha256:next.geometryWasmSha256,inventory,cases,
 inputs:reports.map(({file,sha256})=>({file,sha256})),
 excludedFailedCases:inputs.flatMap(r=>r.cases.filter(c=>c.status!=='passed').map(key)),
 harnessRevisions:reports.map(({report})=>report.sourceHashes[harness]),
 currentSourceDrift,
 scope:`Finite ${inventory.length}-case body UI union for the exact published artifact and captured sources, not subsequent unpublished source edits. Original failed report preserved; qualification harness changed to valid larger straight cancellation workloads and explicit continuation inventory. Product/kernel provenance identical between input reports.`},null,2)+'\n')
console.log(`${cases.length}/${inventory.length} body UI cases verified`)
