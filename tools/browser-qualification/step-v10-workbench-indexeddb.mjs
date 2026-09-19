import {chromium} from 'playwright'
import {spawn} from 'node:child_process'
import {mkdir,writeFile,readFile} from 'node:fs/promises'
import {resolve} from 'node:path'

const port=4182,url=`http://127.0.0.1:${port}`
const server=spawn('npm',['run','dev','--','--host','127.0.0.1','--port',String(port)],{cwd:process.cwd(),stdio:['ignore','pipe','pipe']})
let serverError='';server.stderr.on('data',chunk=>{serverError+=chunk})
const wait=async()=>{for(let attempt=0;attempt<480;attempt++){try{if((await fetch(url)).ok)return}catch{}await new Promise(resolve=>setTimeout(resolve,250))}throw Error(`Vite did not start: ${serverError}`)}
let browser
try{
 await wait();browser=await chromium.launch({headless:true,args:['--enable-features=WebAssemblyUnlimitedSyncCompilation']})
 const page=await browser.newPage({acceptDownloads:true})
 await page.addInitScript(()=>localStorage.setItem('scad-lang','en'))
 await page.goto(url,{waitUntil:'domcontentloaded',timeout:120000})
 await page.getByRole('button',{name:'CAD operations',exact:true}).waitFor({timeout:120000})
 await page.getByRole('button',{name:'CAD operations',exact:true}).evaluate(button=>button.click())
 const input=page.locator('.cad-workbench input[accept=".step,.stp"]').first()
 await input.setInputFiles(resolve('tests/fixtures/step-v6/self-authored-ap242-assembly.step'))
 await page.locator('.cad-workbench').getByText(/3 occurrence\(s\)/).waitFor({timeout:120000})
 const result=await page.evaluate(async()=>{
  const store=await import('/src/services/cadStepIndexedDb.ts')
  const step=await import('/src/services/cadNurbsStep.ts')
  const row=await store.loadProjectStepModel()
  if(!row?.document)throw Error('retained STEP /10 graph is missing')
  const exported=step.exportDirectStepV10(row.document)
  return {graphIdentity:row.document.graphIdentity,occurrences:row.document.occurrenceIdentities.length,
   definitions:row.document.definitionIdentities.length,text:exported.text}
 })
 const original=await readFile('tests/fixtures/step-v6/self-authored-ap242-assembly.step','utf8')
 if(result.text!==original||result.occurrences!==3||result.definitions!==1)throw Error('browser graph roundtrip mismatch')
 const evidence={capability:'step-interchange/10',route:'retained-affine-occurrence-graph',
  graphIdentity:result.graphIdentity,occurrences:result.occurrences,definitions:result.definitions,reimported:true,
  userAgent:await page.evaluate(()=>navigator.userAgent)}
 await mkdir('output/qualification/step-interchange-10-v1',{recursive:true})
 await writeFile('output/qualification/step-interchange-10-v1/browser-workbench-indexeddb.json',JSON.stringify(evidence,null,2)+'\n')
 console.log(JSON.stringify(evidence))
}finally{await browser?.close();server.kill('SIGTERM')}
