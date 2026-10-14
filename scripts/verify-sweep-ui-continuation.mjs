import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFile,writeFile} from 'node:fs/promises'

const [prefixPath,continuationPath,outputPath]=process.argv.slice(2)
assert.ok(prefixPath&&continuationPath&&outputPath,'Supply prefix report, continuation report and output report')
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const reports=await Promise.all([prefixPath,continuationPath].map(async file=>{
 const bytes=await readFile(file)
 return {file,sha256:sha(bytes),report:JSON.parse(bytes)}
}))
const [prefix,next]=reports.map(r=>r.report)
assert.equal(next.passed,true,'Continuation must finish successfully')
const inventory=next.selection.fullInventory
const planned=next.selection.plannedCaseKeys
assert.equal(inventory.length,96,'Expected complete 48-source/two-width body inventory')
assert.equal(new Set(inventory).size,96)
assert.deepEqual(planned,inventory.slice(inventory.indexOf(next.selection.startAt)))
for(const field of ['geometryWasmSha256','appSourceSha256','distIndexSha256','browser'])assert.equal(prefix[field],next[field],`Changed ${field}`)
assert.deepEqual(prefix.artifactProvenance,next.artifactProvenance)
const harness='scripts/check-sweep-miter-matrix-browser.mjs'
const currentSourceDrift=[]
assert.deepEqual(Object.keys(prefix.sourceHashes).sort(),Object.keys(next.sourceHashes).sort())
for(const file of Object.keys(prefix.sourceHashes)){
 if(file!==harness)assert.equal(prefix.sourceHashes[file],next.sourceHashes[file],`Changed ${file}`)
 const currentSha256=sha(await readFile(file))
 if(next.sourceHashes[file]!==currentSha256)currentSourceDrift.push({file,qualifiedSha256:next.sourceHashes[file],currentSha256})
}
const key=c=>`${c.viewport.width}:${c.file}`
const oldPassed=prefix.cases.filter(c=>c.status==='passed')
assert.deepEqual(oldPassed.map(key),inventory.slice(0,inventory.length-planned.length),'Prefix successes must fill exactly the omitted prefix')
assert.deepEqual(next.cases.map(key),planned)
const cases=[...oldPassed,...next.cases]
assert.deepEqual(cases.map(key),inventory,'Complete inventory must run exactly once')
for(const r of [prefix,next]){
 assert.equal(r.nativeRunningCancelProbe,true)
 assert.equal(r.headed,false)
 assert.equal(r.evidenceOnly,false)
 assert.equal(r.gpuBackendRequested,'metal')
}
for(const c of cases){
 assert.equal(c.status,'passed')
 assert.deepEqual(c.pageErrors,[])
 assert.equal(c.sourceSha256,sha(await readFile(`examples/rush/${c.file}`)))
 assert.equal(c.gpuAdapter.vendor,'apple')
 assert.ok(c.gpuAdapter.architecture.startsWith('metal'))
 assert.ok(c.assertions.length>=21)
 assert.ok(c.assertions.every(a=>a.status==='passed'))
 for(const name of ['native-build-cancellation','native-solid-cancel','native-solid-source-change']){
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
 excludedFailedCases:prefix.cases.filter(c=>c.status!=='passed').map(key),
 harnessRevisions:reports.map(({report})=>report.sourceHashes[harness]),
 currentSourceDrift,
 scope:'Finite 96-case body UI union for the exact published artifact and captured sources, not subsequent unpublished source edits. Original failed report preserved; qualification harness changed to valid larger straight cancellation workloads and explicit continuation inventory. Product/kernel provenance identical between input reports.'},null,2)+'\n')
console.log(`${cases.length}/${inventory.length} body UI cases verified`)
