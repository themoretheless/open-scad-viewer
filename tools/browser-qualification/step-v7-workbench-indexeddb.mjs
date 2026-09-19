import {chromium} from 'playwright'
import {spawn} from 'node:child_process'
import {mkdir,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'

const port=4179,url=`http://127.0.0.1:${port}`
const server=spawn('npm',['run','dev','--','--host','127.0.0.1','--port',String(port)],{
  cwd:process.cwd(),stdio:['ignore','pipe','pipe'],
})
let serverError=''
server.stderr.on('data',chunk=>{serverError+=chunk})
const wait=async()=>{
  for(let attempt=0;attempt<480;attempt++){
    try{if((await fetch(url)).ok)return}catch{}
    await new Promise(resolve=>setTimeout(resolve,250))
  }
  throw Error(`Vite did not start: ${serverError}`)
}
let browser
try{
  await wait()
  browser=await chromium.launch({headless:true,args:['--enable-features=WebAssemblyUnlimitedSyncCompilation']})
  const page=await browser.newPage({acceptDownloads:true})
  await page.addInitScript(()=>localStorage.setItem('scad-lang','en'))
  await page.goto(url,{waitUntil:'domcontentloaded',timeout:120000})
  await page.getByRole('button',{name:'CAD operations',exact:true}).waitFor({timeout:120000})
  await page.getByRole('button',{name:'CAD operations',exact:true}).evaluate(button=>button.click())
  await page.locator('.cad-workbench').waitFor({timeout:30000})
  const retained=page.locator('.cad-workbench input[accept=".step,.stp"]').first()
  await retained.setInputFiles(resolve('tests/fixtures/step-v6/self-authored-ap242-periodic-cylinder.step'))
  await page.locator('.cad-workbench').getByText(/AP242 semantics: not-checked/).waitFor({timeout:120000})

  const editor=page.locator('textarea').first()
  await editor.fill((await editor.inputValue())+'\n// STEP /7 lifecycle edit')
  await page.getByRole('button',{name:'Save in browser',exact:true}).evaluate(button=>button.click())
  await page.getByText('Saved to IndexedDB',{exact:true}).waitFor({timeout:30000})
  const beforeReload=await page.evaluate(async()=>{
    const request=indexedDB.open('open-scad-viewer')
    const database=await new Promise((resolve,reject)=>{request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error)})
    const transaction=database.transaction('workspace','readonly')
    const get=transaction.objectStore('workspace').get('active')
    const row=await new Promise((resolve,reject)=>{get.onsuccess=()=>resolve(get.result);get.onerror=()=>reject(get.error)})
    database.close()
    return {source:row.snapshot.source,revision:row.snapshot.revision}
  })
  if(!beforeReload.source.includes('STEP /7 lifecycle edit'))throw Error('edited project was not saved to IndexedDB')

  await page.reload({waitUntil:'domcontentloaded',timeout:120000})
  if(!(await editor.inputValue()).includes('STEP /7 lifecycle edit'))throw Error('IndexedDB project did not reload')
  await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(button=>button.textContent?.trim()==='CAD operations'))
  await page.evaluate(()=>[...document.querySelectorAll('button')].find(button=>button.textContent?.trim()==='CAD operations')?.click())
  await page.locator('.cad-workbench').waitFor({timeout:30000})
  await page.evaluate(()=>[...document.querySelectorAll('button')].find(button=>button.textContent?.trim()==='Export retained model')?.click())
  await page.locator('.cad-workbench').getByText(/step-interchange\/7; identity preserved/).waitFor({timeout:30000})
  const exported=await page.evaluate(async()=>{
    const store=await import('/src/services/cadStepIndexedDb.ts')
    const step=await import('/src/services/cadNurbsStep.ts')
    const row=await store.loadProjectStepModel()
    if(!row)throw Error('project STEP model is missing')
    const text=step.exportDirectStepV7(row.model).text
    if(step.importDirectStepV7(text).model.bodies.length!==1)throw Error('export reimport failed')
    return text
  })
  await retained.setInputFiles({name:'reimport.step',mimeType:'application/step',buffer:Buffer.from(exported)})
  await page.locator('.cad-workbench').getByText(/retained-ap242-brep/).waitFor({timeout:120000})
  const result={capability:'step-interchange/7',indexedDbRevision:beforeReload.revision,
    sourceBytes:new TextEncoder().encode(beforeReload.source).byteLength,exportBytes:Buffer.byteLength(exported),
    semanticConformance:'not-checked',reimported:true,userAgent:await page.evaluate(()=>navigator.userAgent)}
  await mkdir('output/qualification/step-interchange-7-v1',{recursive:true})
  await writeFile('output/qualification/step-interchange-7-v1/browser-workbench-indexeddb.json',JSON.stringify(result,null,2)+'\n')
  console.log(JSON.stringify(result))
}finally{
  await browser?.close()
  server.kill('SIGTERM')
}
