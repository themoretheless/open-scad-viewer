import assert from 'node:assert/strict'
import {readFile,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'
const [input,manifestPath,stageRoot,output]=process.argv.slice(2)
assert.ok(input&&manifestPath&&stageRoot&&output,'Supply UI report, STEP manifest, frozen application stage and output receipt')
const sha=b=>createHash('sha256').update(b).digest('hex')
const bytes=await readFile(input),matrix=JSON.parse(bytes),manifestBytes=await readFile(manifestPath),manifest=JSON.parse(manifestBytes)
const files=[]
for(const closed of [false,true])for(let bits=0;bits<8;bits++){
 const laws=[bits&1?'affine':null,bits&2?'authored':null,bits&4?'guide':null].filter(Boolean).join('-')||'plain'
 files.push(`miter-laws-${closed?'closed':'open'}-${laws}-profile-g2.r`)
}
files.sort()
assert.equal(manifest.cases.length,16)
assert.deepEqual(manifest.cases.map(c=>c.sourceFile).sort(),files)
const expected=[1440,600].flatMap(width=>files.map(file=>`${width}:${file}`))
for(const key of ['fullInventory','plannedCaseKeys'])assert.deepEqual(matrix.selection[key],expected)
assert.deepEqual(matrix.cases.map(c=>`${c.viewport.width}:${c.file}`),expected)
assert.equal(matrix.passed,true);assert.equal(matrix.headed,false);assert.equal(matrix.evidenceOnly,false)
assert.equal(matrix.nativeRunningCancelProbe,true);assert.equal(matrix.gpuBackendRequested,'metal')
const kernel=manifest.artifactProvenance.geometryWasmSha256
assert.equal(matrix.geometryWasmSha256,kernel)
for(const path of ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm'])assert.equal(sha(await readFile(resolve(stageRoot,path))),kernel)
assert.equal(matrix.distIndexSha256,sha(await readFile(resolve(stageRoot,'dist/index.html'))))
for(const [path,hash] of Object.entries(matrix.sourceHashes))assert.equal(sha(await readFile(resolve(stageRoot,path))),hash,`Frozen source changed: ${path}`)
const names=['native-build-cancellation','native-build-source-change','native-solid-cancel','native-solid-source-change'],observations=[]
for(const c of matrix.cases){
 assert.equal(c.status,'passed');assert.deepEqual(c.pageErrors,[])
 const source=await readFile(resolve(dirname(manifestPath),c.file))
 assert.equal(sha(source),c.sourceSha256)
 assert.equal(manifest.cases.find(item=>item.sourceFile===c.file).sourceSha256,c.sourceSha256)
 assert.equal(c.gpuAdapter.vendor,'apple');assert.ok(c.gpuAdapter.architecture.startsWith('metal'))
 assert.ok(c.assertions.length>=21);assert.ok(c.assertions.every(a=>a.status==='passed'))
 for(const name of ['build','viewport-renderer-ready','profile-G2','continuous-boundary-presentation','solid-publication','restore','invalid-path-build-refusal','invalid-path-solid-refusal','bounded-work-refusal'])assert.equal(c.assertions.filter(a=>a.case===name).length,1,`${c.file}: ${name}`)
 assert.equal(c.solid.renderer,'webgpu');assert.ok(c.solid.faces>0&&c.solid.shells>0);assert.match(c.solid.geometrySha256,/^[a-f0-9]{64}$/)
 for(const name of names){
  const matches=c.assertions.filter(a=>a.case===name);assert.equal(matches.length,1)
  const a=matches[0],stack=a.nativeStack;assert.equal(stack.geometryWasmSha256,kernel);assert.equal(a.sourceSha256,name.startsWith('native-build-')
   ?sha(Buffer.from(source.toString()+'\n// Native cancellation observes the original miter geometry in a new revision.\n'))
   :c.sourceSha256)
  assert.ok(stack.nativeSampleCount>=5);assert.equal(stack.debuggerPaused,false)
  assert.ok(Number.isFinite(stack.lastSampleToProfileEndMs)&&stack.lastSampleToProfileEndMs>=0&&stack.lastSampleToProfileEndMs<=5)
  assert.ok(Number.isFinite(a.workerDestroyedAfterMs)&&a.workerDestroyedAfterMs>=0)
  observations.push({sourceSha256:a.sourceSha256,file:c.file,width:c.viewport.width,case:name,nativeSamples:stack.nativeSampleCount,tailMs:stack.lastSampleToProfileEndMs,workerDestroyedAfterMs:a.workerDestroyedAfterMs})
 }
}
assert.equal(observations.length,128)
const receipt={schema:'sweep-ui-miter-laws-complete/1',scope:'Complete finite16 joint miter law sources at both widths on exact frozen application/WASM; build cancellation adds only the exact captured revision comment to the original source, Solid cancellation uses the original source; native profile tail is not worker destruction latency; not universal sweep qualification or main delivery',passed:true,reportSha256:sha(bytes),stepManifestSha256:sha(manifestBytes),verifierSha256:sha(await readFile(new URL(import.meta.url))),geometryWasmSha256:kernel,stageRoot:resolve(stageRoot),cases:32,assertions:matrix.cases.reduce((n,c)=>n+c.assertions.length,0),nativeObservations:observations.length,minNativeSamples:Math.min(...observations.map(a=>a.nativeSamples)),maxTailMs:Math.max(...observations.map(a=>a.tailMs)),allWorkersDestroyed:true,headed:false,gpuBackend:'apple-metal',observations}
await writeFile(output,JSON.stringify(receipt,null,2)+'\n')
console.log(`${receipt.cases} UI cases; ${receipt.assertions} assertions; ${receipt.nativeObservations} authenticated native observations verified`)
