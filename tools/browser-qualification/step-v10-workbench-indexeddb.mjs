import {chromium} from 'playwright'
import {preview} from 'vite'
import {mkdir,writeFile,readFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import assert from 'node:assert/strict'

const port=4182,url=`http://127.0.0.1:${port}`
const output=resolve(process.env.STEP_V10_OUTPUT||'output/qualification/step-interchange-10-v1')
// The qualification parent builds the kernels and production app before this check.
let server,browser,page
const pageErrors=[]
try{
 server=await preview({preview:{host:'127.0.0.1',port,strictPort:true}})
 browser=await chromium.launch({headless:true,args:['--enable-features=WebAssemblyUnlimitedSyncCompilation'],
  ...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true})
 page.on('pageerror',error=>pageErrors.push(error.message))
 await page.addInitScript(()=>localStorage.setItem('scad-lang','en'))
 await page.goto(url,{waitUntil:'domcontentloaded',timeout:120000})
 const workspace=page.locator('.direct-workspace')
 await workspace.getByRole('spinbutton',{name:'Size, mm',exact:true}).fill('2')
 await workspace.getByRole('button',{name:'Box',exact:true}).click()
 const before=await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1')))
 const fileMenu=workspace.locator('details.file-menu')
 await fileMenu.locator('summary').click()
 const choose=page.waitForEvent('filechooser')
 await fileMenu.getByRole('button',{name:'Import STEP',exact:true}).click()
 await (await choose).setFiles(resolve('tests/fixtures/step-v6/self-authored-ap242-assembly.step'))
 await workspace.getByText(/3 occurrence\(s\); original saved/).waitFor({timeout:120000})
 const importedScene=await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1')))
 assert.equal(importedScene.bodies.length,before.bodies.length+1)
 assert.deepEqual(importedScene.bodies.slice(0,before.bodies.length),before.bodies)
 assert.equal(importedScene.bodies.at(-1).brep.bodies.length,3)
 const result=await page.evaluate(async()=>{
  const row=await new Promise((resolve,reject)=>{
   const open=indexedDB.open('open-scad-viewer')
   open.onupgradeneeded=()=>{open.transaction.abort();reject(Error('Project database was not created'))}
   open.onerror=()=>reject(open.error)
   open.onsuccess=()=>{
    const db=open.result
    try{
     const request=db.transaction('step-models','readonly').objectStore('step-models').get('active')
     request.onsuccess=()=>{db.close();resolve(request.result)}
     request.onerror=()=>{db.close();reject(request.error)}
    }catch(error){db.close();reject(error)}
   }
  })
  if(!row?.document)throw Error('retained STEP /10 graph is missing')
  return {graphIdentity:row.document.graphIdentity,occurrences:row.document.occurrenceIdentities.length,
   definitions:row.document.definitionIdentities.length,text:row.document.source}
 })
 const original=await readFile('tests/fixtures/step-v6/self-authored-ap242-assembly.step','utf8')
 if(result.text!==original||result.occurrences!==3||result.definitions!==1)throw Error('browser graph roundtrip mismatch')
 const exportOriginal=async()=>{
  if(!await fileMenu.evaluate(element=>element.open))await fileMenu.locator('summary').click()
  const downloading=page.waitForEvent('download')
  await fileMenu.getByRole('button',{name:'Export AP242 original',exact:true}).click()
  const download=await downloading
  assert.equal(await download.failure(),null)
  assert.equal(await readFile(await download.path(),'utf8'),original)
 }
 await exportOriginal()
 const invalid=page.waitForEvent('filechooser')
 await fileMenu.getByRole('button',{name:'Import STEP',exact:true}).click()
 await (await invalid).setFiles({name:'invalid.step',mimeType:'application/step',buffer:Buffer.from('not STEP')})
 await workspace.getByRole('alert').filter({hasText:/STEP product root/}).waitFor()
 assert.deepEqual(await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1'))),importedScene)
 await exportOriginal()
 await page.reload({waitUntil:'domcontentloaded'})
 await workspace.getByRole('button',{name:'Box',exact:true}).waitFor()
 await exportOriginal()
 assert.deepEqual(await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1'))),importedScene)
 assert.deepEqual(pageErrors,[])
 await fileMenu.locator('summary').click()
 await mkdir(output,{recursive:true})
 await page.screenshot({path:resolve(output,'solid-step-desktop.png'),fullPage:true})
 await page.setViewportSize({width:390,height:844})
 await fileMenu.locator('summary').click()
 await fileMenu.getByRole('button',{name:'Import STEP',exact:true}).waitFor()
 await page.screenshot({path:resolve(output,'solid-step-mobile.png'),fullPage:true})
 const mobileButton=await fileMenu.getByRole('button',{name:'Import STEP',exact:true}).boundingBox()
 assert.ok(mobileButton&&mobileButton.x>=0&&mobileButton.x+mobileButton.width<=390)
 const evidence={capability:'step-interchange/10',route:'retained-affine-occurrence-graph',
  graphIdentity:result.graphIdentity,occurrences:result.occurrences,definitions:result.definitions,
  uiRoute:'solid-file-menu',productionBuild:true,existingBodiesPreserved:true,invalidImportPreservedScene:true,
  originalDownloadMatches:true,reloadPreservedSceneAndOriginal:true,
  userAgent:await page.evaluate(()=>navigator.userAgent)}
 await mkdir(output,{recursive:true})
 await writeFile(resolve(output,'browser-workbench-indexeddb.json'),JSON.stringify(evidence,null,2)+'\n')
 console.log(JSON.stringify(evidence))
}catch(error){
 await mkdir(output,{recursive:true})
 await writeFile(resolve(output,'browser-failure.json'),JSON.stringify({
  message:String(error),url:page?.url()??null,pageErrors,
 },null,2)+'\n')
 if(page)await page.screenshot({path:resolve(output,'browser-failure.png'),fullPage:true}).catch(()=>{})
 throw error
}finally{
 try{await browser?.close()}finally{await server?.close()}
}
