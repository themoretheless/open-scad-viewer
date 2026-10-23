import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import path from 'node:path'
import {build} from 'esbuild'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const output=path.resolve(process.argv[2]??'/private/tmp/solid-draft-pending-browser')
await mkdir(output,{recursive:true})
const source='src/services/solidDraftHeadStore.ts'
const bundle=await build({entryPoints:[source],bundle:true,format:'esm',platform:'browser',write:false})
const javascript=bundle.outputFiles[0].contents
const server=createServer((req,res)=>{
 res.setHeader('Content-Type',req.url==='/module.js'?'text/javascript':'text/html')
 res.end(req.url==='/module.js'?javascript:'<!doctype html><title>Draft transaction acceptance</title>')
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 const page=await browser.newPage(),errors=[]
 page.on('pageerror',e=>errors.push(String(e)))
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const report=await page.evaluate(async()=>{
  const {readSolidDraftHead,writeSolidDraftHead}=await import('/module.js')
  const initial=await writeSolidDraftHead('model',null,'retained')
  const prototype=IDBObjectStore.prototype,put=prototype.put
  let current=true,error
  prototype.put=function(...args){const request=put.apply(this,args);queueMicrotask(()=>{current=false});return request}
  try{await writeSolidDraftHead('model',initial.revision,'obsolete',()=>current)}catch(e){error=e.message}
  finally{prototype.put=put}
  const retained=await readSolidDraftHead('model')
  const latest=await writeSolidDraftHead('model',initial.revision,'latest')
  const competing=await Promise.allSettled([
   writeSolidDraftHead('model',latest.revision,'first'),
   writeSolidDraftHead('model',latest.revision,'second'),
  ])
  return {error,retainedExact:JSON.stringify(retained)===JSON.stringify(initial),retry:latest.text,
   committed:competing.filter(r=>r.status==='fulfilled').length,
   conflicts:competing.filter(r=>r.status==='rejected'&&r.reason.message==='DRAFT_CONFLICT').length,
   finalText:(await readSolidDraftHead('model')).text}
 })
 assert.equal(report.error,'DRAFT_SUPERSEDED');assert.equal(report.retainedExact,true)
 assert.equal(report.retry,'latest');assert.equal(report.committed,1);assert.equal(report.conflicts,1)
 assert.ok(['first','second'].includes(report.finalText));assert.deepEqual(errors,[])
 const evidence={scope:'real browser IndexedDB; source module bundled with esbuild; full interface not exercised',
  browser:browser.version(),sourceSha256:createHash('sha256').update(await readFile(source)).digest('hex'),
  bundleSha256:createHash('sha256').update(javascript).digest('hex'),...report,errors}
 await writeFile(path.join(output,'report.json'),JSON.stringify(evidence,null,2)+'\n')
 console.log(JSON.stringify(evidence))
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
