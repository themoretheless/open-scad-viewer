import assert from 'node:assert/strict'
import {readFile,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'
const [input,manifestPath,packedReceiptPath,stageRoot,output]=process.argv.slice(2)
assert.ok(input&&manifestPath&&packedReceiptPath&&stageRoot&&output,'Supply UI report, original source identity manifest, new packed receipt, frozen stage and output receipt')
const sha=b=>createHash('sha256').update(b).digest('hex')
const bytes=await readFile(input),matrix=JSON.parse(bytes),manifestBytes=await readFile(manifestPath),manifest=JSON.parse(manifestBytes)
// The older STEP manifest supplies original source identities only. It does
// not qualify the new kernel or reuse its OpenCascade results.
const packedBytes=await readFile(packedReceiptPath),packed=JSON.parse(packedBytes)
const files=['circle-solid','circle-hollow','periodic-solid','periodic-hollow'].map(n=>`closed-frenet-arc-${n}-station-g2.r`)
assert.equal(manifest.cases.length,4)
assert.deepEqual(manifest.cases.map(c=>c.sourceFile).sort(),files.map(f=>'examples/rush/'+f).sort())
assert.equal(packed.rawAbiCasesPassed,4)
const expected=[1440,600].flatMap(width=>files.map(file=>`${width}:${file}`))
for(const key of ['fullInventory','plannedCaseKeys'])assert.deepEqual(matrix.selection[key],expected)
assert.deepEqual(matrix.cases.map(c=>`${c.viewport.width}:${c.file}`),expected)
assert.equal(matrix.passed,true);assert.equal(matrix.headed,false);assert.equal(matrix.evidenceOnly,false)
assert.equal(matrix.nativeRunningCancelProbe,true);assert.equal(matrix.gpuBackendRequested,'metal')
const kernel=packed.geometrySha256
assert.equal(matrix.geometryWasmSha256,kernel)
for(const path of ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm'])assert.equal(sha(await readFile(resolve(stageRoot,path))),kernel)
assert.equal(matrix.distIndexSha256,sha(await readFile(resolve(stageRoot,'dist/index.html'))))
for(const [path,hash] of Object.entries(matrix.sourceHashes))assert.equal(sha(await readFile(resolve(stageRoot,path))),hash,`Frozen source changed: ${path}`)
const names=['native-build-cancellation','native-build-source-change','native-solid-cancel','native-solid-source-change'],observations=[]
for(const c of matrix.cases){
 assert.equal(c.status,'passed');assert.deepEqual(c.pageErrors,[])
 const source=await readFile(resolve(stageRoot,'examples/rush',c.file))
 assert.equal(sha(source),c.sourceSha256)
 assert.equal(manifest.cases.find(item=>item.sourceFile==='examples/rush/'+c.file).sourceSha256,c.sourceSha256)
 assert.equal(c.gpuAdapter.vendor,'apple');assert.ok(c.gpuAdapter.architecture.startsWith('metal'))
 assert.ok(c.assertions.length>=22);assert.ok(c.assertions.every(a=>a.status==='passed'))
 for(const name of ['build','viewport-renderer-ready','profile-G2','station-G2','continuous-boundary-presentation','solid-publication','restore','invalid-path-build-refusal','invalid-path-solid-refusal','bounded-work-refusal'])assert.equal(c.assertions.filter(a=>a.case===name).length,1,`${c.file}: ${name}`)
 assert.equal(c.solid.renderer,'webgpu');assert.ok(c.solid.faces>0&&c.solid.shells>0);assert.match(c.solid.geometrySha256,/^[a-f0-9]{64}$/)
 for(const name of names){
  const matches=c.assertions.filter(a=>a.case===name);assert.equal(matches.length,1)
  const a=matches[0],stack=a.nativeStack;assert.equal(stack.geometryWasmSha256,kernel);assert.equal(a.sourceSha256,name.startsWith('native-build-')
   ?sha(Buffer.from(source.toString()+'\n// Observe the original progressive station geometry in a new revision.\n'))
   :c.sourceSha256)
  assert.ok(stack.nativeSampleCount>=5);assert.equal(stack.debuggerPaused,false)
  assert.ok(Number.isFinite(stack.lastSampleToProfileEndMs)&&stack.lastSampleToProfileEndMs>=0&&stack.lastSampleToProfileEndMs<=5)
  assert.ok(Number.isFinite(a.workerDestroyedAfterMs)&&a.workerDestroyedAfterMs>=0)
  observations.push({sourceSha256:a.sourceSha256,file:c.file,width:c.viewport.width,case:name,nativeSamples:stack.nativeSampleCount,tailMs:stack.lastSampleToProfileEndMs,workerDestroyedAfterMs:a.workerDestroyedAfterMs})
 }
}
assert.equal(observations.length,32)
assert.match(await readFile(resolve(stageRoot,'src/App.vue'),'utf8'),/workerSilenceTimeoutMs: 30_000/)
const receipt={schema:'sweep-ui-closed-frenet-complete/1',scope:'Eight finite closed Frenet UI cases on the exact new packed WASM. Older STEP manifest binds source bytes only, not new STEP/OCCT proof. Build cancellation adds only the exact revision comment; profile tail is not Worker destruction latency; no main delivery or universal claim',passed:true,reportSha256:sha(bytes),originalSourceManifestSha256:sha(manifestBytes),packedReceiptSha256:sha(packedBytes),verifierSha256:sha(await readFile(new URL(import.meta.url))),geometryWasmSha256:kernel,stageRoot:resolve(stageRoot),cases:8,assertions:matrix.cases.reduce((n,c)=>n+c.assertions.length,0),nativeObservations:observations.length,minNativeSamples:Math.min(...observations.map(a=>a.nativeSamples)),maxTailMs:Math.max(...observations.map(a=>a.tailMs)),allWorkersDestroyed:true,headed:false,gpuBackend:'apple-metal',observations}
await writeFile(output,JSON.stringify(receipt,null,2)+'\n')
console.log(`${receipt.cases} UI cases; ${receipt.assertions} assertions; ${receipt.nativeObservations} authenticated native observations verified`)
