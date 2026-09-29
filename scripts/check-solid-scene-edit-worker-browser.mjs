import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
const keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
assert.ok(['system','dark','light','nord','solarized'].includes(theme))
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{
 try {
  const url=new URL(req.url,'http://localhost'),file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
  if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser,page
const renderErrors=[]
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true})
 page.on('pageerror',e=>renderErrors.push(String(e)));page.on('console',m=>{if(m.type()==='error')renderErrors.push(m.text())})
 await page.addInitScript(()=>{
  window.__sceneEditRequests=0;window.__holdTransform=false;window.__lateTransform=null;window.__transformTerminated=false;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){if(message?.job?.kind==='sceneEdit'){
    window.__sceneEditRequests++
    if(window.__holdTransform&&message.job.options?.operation==='transform'){
     this.held=true;const callback=this.onmessage
     this.onmessage=event=>{if(event.data?.ok===true&&message.job.options.x===5)window.__lateTransform=()=>callback?.call(this,event)}
    }
   }return super.postMessage(message,...args)}
   terminate(){if(this.held)window.__transformTerminated=true;return super.terminate()}
  }
  if(!navigator.gpu)return
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const make=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const device=await make(...args);window.__qualificationGpuDevice=device;return device}}return adapter}
 })
 const origin=`http://127.0.0.1:${server.address().port}`
 await page.goto(origin)
 await page.getByRole('combobox',{name:'Тема',exact:true}).selectOption(theme)
 let tabPresses=0
 async function tabTo(locator){
  for(let i=0;i<250;i++){
   if(await locator.evaluate(el=>el===document.activeElement))return
   await page.keyboard.press('Tab');tabPresses++
  }
  throw Error('Target is unreachable through sequential Tab navigation: '+await locator.getAttribute('aria-label'))
 }
 async function activate(locator){
  if(keyboard){await tabTo(locator);await page.keyboard.press('Enter')}
  else await locator.click()
 }
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 let downloads=0
 const menu=solid.locator('summary[title="Файл"]')
 async function openMenu(){if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)}
 async function download(label,file,json=true){
  await openMenu()
  const pending=page.waitForEvent('download',{timeout:20000})
  await activate(solid.getByRole('button',{name:label,exact:true}))
  const item=await pending
  assert.equal(await item.failure(),null)
  await item.saveAs(path.join(directory,file));downloads++
  if(keyboard){
   await page.keyboard.press('Escape')
   assert.equal(await menu.evaluate(e=>e.parentElement.open),false)
   assert.equal(await menu.evaluate(e=>e===document.activeElement),true)
  }
  const text=await readFile(path.join(directory,file),'utf8')
  return json?JSON.parse(text):text
 }
 const original={version:1,sketches:[],bodies:[]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}else{await search.fill(name);await search.press('Enter')}}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command('Box')
 await solid.locator('[data-body]').first().waitFor({state:'visible'})
 await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'})
 await command('Create linked instance');await activate(apply)
 await command('Select instance source')
 const before=await download('Скачать проект JSON','before.json');assert.equal(before.bodies.length,2)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await page.evaluate(()=>window.__holdTransform=true)
 await command('Transform selection')
 await input(solid.getByLabel(/^X/),'5')
 await page.waitForFunction(()=>typeof window.__lateTransform==='function')
 if(process.argv.includes('--invalid-input')){
  await page.evaluate(()=>window.__transformTerminated=false)
  await input(solid.getByLabel(/^X/),'-')
  assert.equal(await solid.getByLabel(/^X/).getAttribute('aria-invalid'),'true')
  if(process.argv.includes('--field-errors')){
   const field=solid.getByLabel('X',{exact:true})
   const errorId=await field.getAttribute('aria-errormessage')
   assert.ok(errorId)
   assert.ok((await field.getAttribute('aria-describedby')).split(' ').includes(errorId))
   assert.equal(await page.locator('[id="'+errorId+'"]').innerText(),'Введите конечное число')
  }
  await page.waitForFunction(()=>window.__transformTerminated)
  await page.evaluate(()=>window.__lateTransform())
  assert.equal(await solid.locator('[data-preview-body]').count(),0)
  assert.equal(await apply.isDisabled(),true)
  if(process.argv.includes('--field-errors')){
   const field=solid.getByLabel('X',{exact:true}),errorId=await field.getAttribute('aria-errormessage')
   await input(field,'5')
   assert.equal(await field.getAttribute('aria-invalid'),'false')
   assert.equal(await field.getAttribute('aria-errormessage'),null)
   assert.equal(await field.getAttribute('aria-describedby'),null)
   assert.equal(await page.locator('[id="'+errorId+'"]').count(),0)
  }
 }
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__transformTerminated)
 await page.evaluate(()=>{window.__holdTransform=false;window.__lateTransform()})
 assert.deepEqual(await download('Скачать проект JSON','cancelled.json'),before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Transform selection')
 await input(solid.getByLabel(/^X/),'5')
 await apply.click({trial:true})
 assert.deepEqual(new Set(await solid.locator('[data-preview-body]').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('data-preview-body')))),new Set(before.bodies.map(b=>b.id)))
 const requests=await page.evaluate(()=>window.__sceneEditRequests);assert.ok(requests>0)
 await page.screenshot({path:path.join(directory,'scene-edit-preview.png')})
 await activate(apply);assert.equal(await page.evaluate(()=>window.__sceneEditRequests),requests)
 const changed=await download('Скачать проект JSON','transformed.json');assert.equal(changed.bodies.length,2)
 assert.equal(changed.bodies[0].id,before.bodies[0].id);assert.equal(changed.bodies[1].id,before.bodies[1].id);assert.equal(changed.bodies[1].instance.sourceId,changed.bodies[0].id)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const step=await download('STEP выбранного тела · текущая геометрия','browser-transform.step',false)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Worker source transform',file:'browser-transform.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:8000,boundsMm:[[-5,-10,0],[15,10,20]]}}]}))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone.bodies,before.bodies)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','redone.json');assert.deepEqual(redone.bodies,changed.bodies)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,applyWithoutNewWorkerRequest:true,cancelledLateSuccess:true,invalidInput:process.argv.includes('--invalid-input'),fieldErrors:process.argv.includes('--field-errors'),undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'scene-edit-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
