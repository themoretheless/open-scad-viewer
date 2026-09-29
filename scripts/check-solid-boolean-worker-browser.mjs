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
  window.__booleanRequests=0;window.__holdBoolean=false;window.__lateBoolean=null;window.__booleanTerminated=false;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='boolean'){
     window.__booleanRequests++
     if(window.__holdBoolean){this.held=true;const callback=this.onmessage;this.onmessage=event=>{if(event.data?.ok===true)window.__lateBoolean=()=>callback?.call(this,event)}}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.held)window.__booleanTerminated=true;return super.terminate()}
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
 const fixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-scene-edit/mouse/before.json','utf8'))
 const source=fixture.bodies.find(b=>!b.instance)
 const original={version:1,sketches:[],bodies:[{...structuredClone(source),id:'stock',name:'Stock'},{...structuredClone(source),id:'cutter',name:'Cutter'}]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}else{await search.fill(name);await search.press('Enter')}}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await activate(solid.getByRole('button',{name:'Cutter',exact:true}))
 await command('Transform selection');await input(solid.getByLabel('X',{exact:true}),'5');await activate(apply)
 const before=await download('Скачать проект JSON','before.json');assert.equal(before.bodies.length,2)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function selectPair(){
  await activate(solid.getByRole('button',{name:'Stock',exact:true}))
  const cutter=solid.getByRole('button',{name:'Cutter',exact:true})
  if(keyboard){await tabTo(cutter);await page.keyboard.down('Shift');await page.keyboard.press('Enter');await page.keyboard.up('Shift')}
  else await cutter.click({modifiers:['Shift']})
 }
 await page.evaluate(()=>window.__holdBoolean=true)
 await selectPair();await command('Union bodies')
 await page.waitForFunction(()=>typeof window.__lateBoolean==='function')
 await writeFile(path.join(directory,'pending-focus.json'),JSON.stringify(await page.evaluate(()=>({tag:document.activeElement?.tagName,html:document.activeElement?.outerHTML,pending:document.body.innerText.includes('Boolean')})),null,2))
 await page.keyboard.press('Escape')
 await page.waitForFunction(()=>window.__booleanTerminated)
 await page.evaluate(()=>{window.__holdBoolean=false;window.__lateBoolean()})
 assert.deepEqual(await download('Скачать проект JSON','cancelled.json'),before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await selectPair();await command('Union bodies')
 await solid.getByRole('button',{name:'Cutter',exact:true}).waitFor({state:'detached'})
 const united=await download('Скачать проект JSON','union.json');assert.equal(united.bodies.length,1);assert.equal(united.bodies[0].id,'stock')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 await selectPair();await command('B-rep A − B')
 await activate(solid.getByRole('button',{name:'OK',exact:true}))
 await solid.getByRole('dialog',{name:'Вычитание',exact:true}).waitFor({state:'detached'})
 const requests=await page.evaluate(()=>window.__booleanRequests);assert.equal(requests,3)
 await page.screenshot({path:path.join(directory,'boolean-result.png')})
 const changed=await download('Скачать проект JSON','subtracted.json');assert.equal(changed.bodies.length,1);assert.equal(changed.bodies[0].id,'stock')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const step=await download('STEP выбранного тела · текущая геометрия','browser-boolean.step',false)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Worker subtraction',file:'browser-boolean.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:2000,boundsMm:[[-10,-10,0],[-5,10,20]]}}]}))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone.bodies,before.bodies)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','redone.json');assert.deepEqual(redone.bodies,changed.bodies)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,booleanOperations:2,cancelledLateSuccess:true,undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'boolean-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
