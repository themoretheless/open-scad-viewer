import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve,dirname} from 'node:path'
import {createHash} from 'node:crypto'
const [input,output,...options]=process.argv.slice(2)
assert.ok(input&&output,'Supply inventory directory and output receipt')
const root=process.cwd(),base=resolve(input),sha=b=>createHash('sha256').update(b).digest('hex')
const read=p=>readFileSync(p),json=p=>JSON.parse(read(p))
const planBytes=read(resolve(base,'inventory-plan.json')),plan=JSON.parse(planBytes)
assert.equal(plan.passedPreflight,true);assert.equal(plan.totalCases,478);assert.equal(plan.nativeObservationsRequired,1472)
assert.equal(plan.jobs.length,11)
const baselineBytes=read(resolve(root,'scripts/sweep-ui-required-baseline.json')),baseline=JSON.parse(baselineBytes);
assert.equal(plan.baselineSha256,sha(baselineBytes));
assert.deepEqual(plan.jobs.map(j=>j.name),Object.keys(baseline.groups));
for(const job of plan.jobs)assert.deepEqual([...job.caseKeys].sort(),[...baseline.groups[job.name],...(baseline.extensions[job.name]??[])].sort(),'Missing or undeclared UI case');
const mapArg=options.find(a=>a.startsWith('--report-map='))
const reportMap=mapArg?json(resolve(mapArg.slice('--report-map='.length))):{}
const harnessArchive=options.find(a=>a.startsWith('--harness-archive='))?.slice('--harness-archive='.length)
assert.ok(Object.keys(reportMap).every(k=>plan.jobs.some(j=>j.name===k)),'Unknown replacement group')
const kernel=sha(read(resolve(root,'public/wasm/geometry-kernel.wasm'))),index=sha(read(resolve(root,'dist/index.html')))
assert.equal(kernel,plan.geometryWasmSha256);assert.equal(index,plan.distIndexSha256)
for(const p of ['src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm'])assert.equal(sha(read(resolve(root,p))),kernel)
const groups=[],observations=[];let count=0
for(const job of plan.jobs){
 const reportPath=resolve(base,reportMap[job.name]??`${job.name}/matrix.json`),bytes=read(reportPath),matrix=JSON.parse(bytes)
 assert.equal(matrix.passed,true,job.name);assert.equal(matrix.headed,false);assert.equal(matrix.evidenceOnly,false)
 assert.equal(matrix.gpuBackendRequested,'metal');assert.equal(matrix.geometryWasmSha256,kernel);assert.equal(matrix.distIndexSha256,index)
 assert.equal(matrix.artifactProvenance.sourcePublicDistVerified,true);assert.equal(matrix.artifactProvenance.sha256,kernel)
 assert.deepEqual(matrix.selection.fullInventory,job.caseKeys);assert.deepEqual(matrix.selection.plannedCaseKeys,job.caseKeys)
 assert.deepEqual(matrix.cases.map(c=>`${c.viewport.width}:${c.file}`),job.caseKeys)
 if(matrix.continuationInputs){
  const captured=[]
  for(const segment of matrix.continuationInputs){
   const segmentBytes=read(segment.path);assert.equal(sha(segmentBytes),segment.sha256)
   const report=JSON.parse(segmentBytes)
   assert.deepEqual(report.sourceHashes,matrix.sourceHashes)
   assert.equal(report.geometryWasmSha256,kernel);assert.equal(report.distIndexSha256,index)
   assert.deepEqual(report.selection.fullInventory,job.caseKeys)
   assert.deepEqual(report.selection.plannedCaseKeys,job.caseKeys.slice(captured.length))
   const failed=report.cases.findIndex(c=>c.status!=='passed')
   if(failed>=0)assert.equal(report.cases.length,failed+1)
   const successes=failed<0?report.cases:report.cases.slice(0,failed)
   assert.equal(successes.length,segment.successfulCases);captured.push(...successes)
  }
  assert.deepEqual(captured,matrix.cases,'Composite differs from actual captured successful cases')
 }
 assert.equal(new Set(job.caseKeys).size,job.caseKeys.length)
 for(const [p,h] of Object.entries(matrix.sourceHashes)){
  if(sha(read(resolve(root,p)))===h)continue
  // Only the test harness may differ between completed groups. Every product
  // source stays byte-identical; the exact earlier harness must be preserved.
  assert.equal(p,'scripts/check-sweep-miter-matrix-browser.mjs',`Changed application source: ${p}`)
  assert.ok(harnessArchive,'Missing exact historical harness')
  assert.match(h,/^[a-f0-9]{64}$/)
  assert.equal(sha(read(resolve(harnessArchive,`${h}.mjs`))),h)
 }
 if(job.nativeRunningCancelApplicable)assert.equal(matrix.nativeRunningCancelProbe,true)
 for(const c of matrix.cases){
  assert.equal(c.status,'passed');assert.deepEqual(c.pageErrors,[]);assert.ok([1440,600].includes(c.viewport.width))
  const source=job.sources.find(s=>s.file===c.file);assert.ok(source)
  assert.equal(sha(read(resolve(job.sourceDirectory,c.file))),source.sha256);assert.equal(c.sourceSha256,source.sha256)
  assert.equal(c.gpuAdapter.vendor,'apple');assert.ok(c.gpuAdapter.architecture.startsWith('metal'))
  assert.ok(c.assertions.length>0);assert.ok(c.assertions.every(a=>a.status==='passed'))
  for(const name of ['build','viewport-renderer-ready','build-cancellation','solid-cancellation','source-change','invalid-path-build-refusal','invalid-path-solid-refusal'])
   assert.equal(c.assertions.filter(a=>a.case===name).length,1,`${job.name}/${c.file}: ${name}`)
  const outcomes=['solid-publication','surface-solid-refusal','bounded-closed-body-solid-refusal','unproved-spatial-solid-refusal']
  const outcome=c.assertions.filter(a=>outcomes.includes(a.case));assert.equal(outcome.length,1,'Missing or conflicting Solid outcome')
  if(job.name==='surface')assert.equal(outcome[0].case,'surface-solid-refusal')
  else if(job.name==='folded-fixed-refusal')assert.equal(outcome[0].case,'bounded-closed-body-solid-refusal')
  else if(job.name==='miter')assert.equal(outcome[0].case,'solid-publication')
  else assert.equal(outcome[0].case,'solid-publication')
  if(outcome[0].case==='solid-publication'){
   assert.ok(c.solid);assert.equal(c.solid.renderer,'webgpu');assert.ok(c.solid.faces>0&&c.solid.shells>0);assert.match(c.solid.geometrySha256,/^[a-f0-9]{64}$/)
   assert.equal(c.assertions.filter(a=>a.case==='restore').length,1)
  }else{
   assert.ok(!c.solid);assert.equal(c.assertions.filter(a=>a.case==='refused-empty-project-restoration').length,1)
  }
  if(job.nativeRunningCancelApplicable)for(const name of ['native-build-cancellation','native-build-source-change','native-solid-cancel','native-solid-source-change']){
   const matches=c.assertions.filter(a=>a.case===name);assert.equal(matches.length,1)
   const a=matches[0],s=a.nativeStack;assert.equal(s.geometryWasmSha256,kernel);assert.match(a.sourceSha256,/^[a-f0-9]{64}$/)
   assert.ok(s.nativeSampleCount>=5);assert.equal(s.debuggerPaused,false)
   assert.ok(Number.isFinite(s.lastSampleToProfileEndMs)&&s.lastSampleToProfileEndMs>=0&&s.lastSampleToProfileEndMs<=5)
   assert.ok(Number.isFinite(a.workerDestroyedAfterMs)&&a.workerDestroyedAfterMs>=0)
   observations.push({group:job.name,file:c.file,width:c.viewport.width,case:name,sourceSha256:a.sourceSha256,samples:s.nativeSampleCount,tailMs:s.lastSampleToProfileEndMs,workerDestroyedAfterMs:a.workerDestroyedAfterMs})
  }
 }
 count+=matrix.cases.length;groups.push({name:job.name,reportPath,sha256:sha(bytes),cases:matrix.cases.length,replacement:Object.hasOwn(reportMap,job.name)})
}
assert.equal(count,478);assert.equal(observations.length,1472)
assert.match(read(resolve(root,'src/App.vue')).toString(),/workerSilenceTimeoutMs: 30_000/)
writeFileSync(output,JSON.stringify({schema:'sweep-full-ui-reconciliation/1',passed:true,scope:'Finite478-case matrix on exact isolated WASM/dist and source bytes; authenticated executing native CPU tail followed by Worker destruction, not cooperative cancellation latency or universal geometry proof',geometryWasmSha256:kernel,distIndexSha256:index,inventorySha256:sha(planBytes),verifierSha256:sha(read(new URL(import.meta.url))),cases:count,nativeObservations:observations.length,groups,observations},null,2)+'\n')
console.log('478 UI cases and1472 authenticated native observations verified')
