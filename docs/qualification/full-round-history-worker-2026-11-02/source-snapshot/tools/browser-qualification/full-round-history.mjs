import assert from 'node:assert/strict'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import {build,preview} from 'vite'
import {chromium} from 'playwright'
const output=resolve(process.argv[2]);await mkdir(output,{recursive:true})
const root=process.cwd(),dist=resolve(output,'dist')
await build({configFile:false,root,publicDir:false,worker:{format:'es'},build:{outDir:dist,emptyOutDir:true,rollupOptions:{input:resolve(root,'tools/browser-qualification/full-round-history.ts'),output:{entryFileNames:'qualification.js'}}}})
let browser,server
try{
 server=await preview({configFile:false,root,publicDir:false,build:{outDir:dist},preview:{host:'127.0.0.1',port:4190,strictPort:true}})
 browser=await chromium.launch({headless:true})
 const page=await browser.newPage(),errors=[],workers=[]
 await page.addInitScript(()=>{
  const Native=globalThis.Worker
  globalThis.historyWorkerAudit={started:0,terminated:0}
  globalThis.Worker=class extends Native{
   constructor(...args){super(...args);globalThis.historyWorkerAudit.started++}
   terminate(){globalThis.historyWorkerAudit.terminated++;super.terminate()}
  }
 })
 page.on('pageerror',e=>errors.push(e.message));page.on('worker',w=>workers.push(w.url()))
 await page.route('**/__history',r=>r.fulfill({contentType:'text/html',body:'<!doctype html><script type="module" src="/qualification.js"></script>'}))
 await page.goto('http://127.0.0.1:4190/__history')
 await page.waitForFunction(()=>typeof globalThis.runHistoryQualification==='function')
 const result=await page.evaluate(()=>globalThis.runHistoryQualification())
 for(const k of ['changed','undoExact','redoExact','inputIntact','moreTriangles'])assert.equal(result[k],true,k)
 assert.match(result.busy,/busy/);assert.match(result.cancelled,/cancelled/);assert.match(result.deadline,/deadline/)
 assert.match(result.malformed,/clone|request/i);assert(!result.failed.includes('Expected refusal'))
 assert.equal(result.ownership.active,false);assert.equal(result.ownership.workersStarted,6);assert.equal(result.ownership.workersTerminated,6)
 const platformAudit=await page.evaluate(()=>globalThis.historyWorkerAudit)
 assert.deepEqual(platformAudit,{started:6,terminated:6})
 // Immediate deadline may terminate a Worker before Chromium emits its debug event.
 assert(workers.length>=4&&workers.length<=6);assert(result.ticks>0);assert.deepEqual(errors,[])
 const identity=await readFile(resolve(root,'src/generated/geometry-kernels/identity.ts'),'utf8')
 await writeFile(resolve(output,'report.json'),JSON.stringify({passed:true,browser:browser.version(),installedPackage:true,identity,result,workers,platformAudit},null,2)+'\n')
 console.log(JSON.stringify({passed:true,...result}))
}finally{await browser?.close();await server?.httpServer.close()}
