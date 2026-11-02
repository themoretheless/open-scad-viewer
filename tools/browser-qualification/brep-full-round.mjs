import assert from 'node:assert/strict'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
import { build, preview } from 'vite'
import { chromium } from 'playwright'

const [candidate, outputArg] = process.argv.slice(2)
assert(candidate && outputArg, 'Expected staged geometry-kernel package directory and output directory')
const output=resolve(outputArg), stage=resolve(candidate), root=process.cwd()
await mkdir(output,{recursive:true})
const identityText=await readFile(resolve(stage,'identity.ts'),'utf8')
const identity=JSON.parse(identityText.match(/Object\.freeze\((\{.*\})\)/)[1])
const kernel=await readFile(resolve(stage,'kernel_bg.wasm'))
assert.equal(createHash('sha256').update(kernel).digest('hex'),identity.sha256)
assert.equal(kernel.length,identity.byteLength)
const alias=()=>({name:'staged-geometry-package',enforce:'pre',resolveId(source){
  const match=/generated\/geometry-kernels\/(identity|bytes)(?:\.ts)?$/.exec(source)
  return match?resolve(stage,match[1]+'.ts'):null
}})
const installed = stage === resolve(root,'src/generated/geometry-kernels')
const plugins = () => installed ? [] : [alias()]
const dist=resolve(output,'dist')
await build({configFile:false,root,publicDir:false,plugins:plugins(),worker:{format:'es',plugins},
  build:{outDir:dist,emptyOutDir:true,rollupOptions:{input:resolve(root,'tools/browser-qualification/brep-full-round.ts'),output:{entryFileNames:'qualification.js'}}}})
let server,browser
try {
  server=await preview({configFile:false,root,publicDir:false,build:{outDir:dist},preview:{host:'127.0.0.1',port:4189,strictPort:true}})
  browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
  const page=await browser.newPage(), errors=[], workerUrls=[]
  page.on('pageerror',error=>errors.push(error.message))
  page.on('worker',worker=>workerUrls.push(worker.url()))
  await page.route('**/__full-round-worker',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><script type="module" src="/qualification.js"></script>'}))
  await page.goto('http://127.0.0.1:4189/__full-round-worker')
  await page.waitForFunction(()=>typeof globalThis.runFullRoundWorkerQualification==='function')
  const result=await page.evaluate(()=>globalThis.runFullRoundWorkerQualification())
  assert.equal(result.single.meshes.length,1)
  assert.equal(result.multi.meshes.length,2)
  assert.equal(result.recovered.meshes.length,1)
  for(const receipt of [result.single,result.multi,result.recovered]) {
    assert.equal(receipt.metrics,'display_mesh_estimates')
    assert.equal(receipt.identity.diagnosticOnly,true)
    for(let i=0;i<receipt.meshes.length;i++) {
      const mesh=receipt.meshes[i]
      assert(mesh.triangles>0)
      assert.equal(mesh.coefficientType,'string')
      assert.equal(mesh.faceIdsAuthoritative,true)
      assert.deepEqual(mesh.topologyFaceIds,receipt.outputs[i].topologyFaceIds)
    }
  }
  assert(Object.values(result.messageChecks).every(ok=>ok===true))
  assert.equal(result.invalidSelection,'E_SEMANTIC_BACKEND_FAILURE')
  assert.equal(result.cancelled,'E_BREP_DIAGNOSTIC_CANCELLED')
  assert.equal(result.ownership.activeWorkerEpoch,null)
  assert.equal(result.ownership.workersStarted,5)
  assert.equal(result.ownership.workersTerminated,5)
  assert.equal(result.ownership.quarantined,false)
  assert.equal(workerUrls.length,5)
  assert(result.ticks>0,'Main realm did not tick during worker evaluation')
  assert.deepEqual(errors,[])
  await writeFile(resolve(output,'report.json'),JSON.stringify({passed:true,installedPackage:installed,kernel:identity,browser:browser.version(),workerUrls,result},null,2)+'\n')
  console.log(JSON.stringify({passed:true,workers:workerUrls.length,kernel:identity.sha256,ticks:result.ticks}))
}finally{await browser?.close();await server?.httpServer.close()}
