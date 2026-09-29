import assert from 'node:assert/strict'
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
  window.__sceneEditRequests=0;window.__transformOptions=[];const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='nurbsEdit'){window.__sceneEditRequests++;window.__transformOptions.push(message.job.options);if(window.__holdNurbs){this.held=true;window.__heldNurbs=true;return}}return super.postMessage(message,...args)}terminate(){if(this.held)window.__terminatedNurbs=true;return super.terminate()}}
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
 async function enterValue(locator,value){
  if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+a');await page.keyboard.insertText(value)}else await locator.fill(value)
  assert.equal(await locator.inputValue(),value)
 }
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 let downloads=0
 const menu=solid.locator('summary[title="Файл"]')
 async function openMenu(){if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)}
 async function ready(){
  await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'))
  for(const name of ['history-restore','surface-display','curve-display','profile-display','display-refinement'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})
 }
 async function download(label,file,json=true){
  await ready()
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
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await enterValue(search,name);await search.press('Enter')}
 await command('NURBS curve')
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function verifyCancel(button,original){
  await page.evaluate(()=>{window.__holdNurbs=true;window.__heldNurbs=false;window.__terminatedNurbs=false})
  const target=solid.getByRole('button',{name:button,exact:true})
  await activate(target);await page.waitForFunction(()=>window.__heldNurbs)
  const requests=await page.evaluate(()=>window.__sceneEditRequests)
  if(keyboard)await page.keyboard.press('Enter')
  else {const box=await target.boundingBox();assert.ok(box);await page.mouse.click(box.x+box.width/2,box.y+box.height/2)}
  assert.equal(await page.evaluate(()=>window.__sceneEditRequests),requests)
  await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__terminatedNurbs)
  assert.deepEqual(await download('Скачать проект JSON',`cancel-${downloads}.json`),original)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await page.evaluate(()=>window.__holdNurbs=false)
 }
 const cases=[['Insert knot','knot'],['Degree +1','elevate'],['Выдавить NURBS 10 мм','extrude'],['Копировать sampled-кривую в эскиз','bake']]
 for(const [button,kind] of cases){
  if(button==='Обрезать диапазон UV'){await enterValue(solid.locator('.trim-grid label').nth(0).locator('input'),'0.2');await enterValue(solid.locator('.trim-grid label').nth(1).locator('input'),'0.8');await enterValue(solid.locator('.trim-grid label').nth(2).locator('input'),'0.1');await enterValue(solid.locator('.trim-grid label').nth(3).locator('input'),'0.9')}
  await verifyCancel(button,before)
  await activate(solid.getByRole('button',{name:button,exact:true}))
  await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется NURBS-команда.'))
  const changed=await download('Скачать проект JSON',kind+'.json')
  assert.equal(changed.curves[0].id,before.curves[0].id)
  if(kind==='knot')assert.equal(changed.curves[0].curve.controlPoints.length,before.curves[0].curve.controlPoints.length+1)
  if(kind==='elevate')assert.equal(changed.curves[0].curve.degree,before.curves[0].curve.degree+1)
  if(kind==='extrude')assert.equal(changed.surfaces.length,1)
  if(kind==='bake')assert.equal(changed.sketches.length,1)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  const undone=await download('Скачать проект JSON',kind+'-undone.json');assert.deepEqual(undone,before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↷',exact:true}))
  const redone=await download('Скачать проект JSON',kind+'-redone.json');assert.deepEqual(redone,changed)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  await activate(solid.getByRole('button',{name:'NURBS curve',exact:true}))
 }
 await command('NURBS surface')
 const surfaceBefore=await download('Скачать проект JSON','surface-before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 for(const button of ['Knot U','Knot V','Degree U +1','Degree V +1','Iso U','Iso V','Обрезать диапазон UV','Bake поверхности в mesh-тело']){
  if(button==='Обрезать диапазон UV'){await enterValue(solid.locator('.trim-grid label').nth(0).locator('input'),'0.2');await enterValue(solid.locator('.trim-grid label').nth(1).locator('input'),'0.8');await enterValue(solid.locator('.trim-grid label').nth(2).locator('input'),'0.1');await enterValue(solid.locator('.trim-grid label').nth(3).locator('input'),'0.9')}
  await verifyCancel(button,surfaceBefore)
  await activate(solid.getByRole('button',{name:button,exact:true}))
  await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется NURBS-команда.'))
  const changed=await download('Скачать проект JSON',`surface-${downloads}.json`)
  assert.equal(changed.surfaces[0].id,surfaceBefore.surfaces[0].id)
  if(button==='Обрезать диапазон UV'){const s=changed.surfaces[0].surface;assert.deepEqual([s.knotsU[0],s.knotsU.at(-1),s.knotsV[0],s.knotsV.at(-1)],[0.2,0.8,0.1,0.9])}
  if(button.startsWith('Iso'))assert.equal(changed.curves.length,2)
  else if(button.startsWith('Bake'))assert.equal(changed.bodies.length,1)
  else assert.notDeepEqual(changed.surfaces,surfaceBefore.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  const undone=await download('Скачать проект JSON',`surface-undone-${downloads}.json`);assert.deepEqual(undone,surfaceBefore)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↷',exact:true}))
  assert.deepEqual(await download('Скачать проект JSON',`surface-redone-${downloads}.json`),changed)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}));await ready()
  assert.ok(await solid.locator('[data-surface]').count()>0)

  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'NURBS surface',exact:true}))
 }
 await page.screenshot({path:path.join(directory,'nurbs-edit.png')})
 const requests=await page.evaluate(()=>window.__sceneEditRequests);assert.equal(requests,24)
 const saved=await download('Скачать проект JSON','before-reload.json')
 await page.reload();await ready()
 assert.deepEqual(await download('Скачать проект JSON','reloaded.json'),saved)
 assert.ok(await solid.locator('[data-surface]').count()>0)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,nativeNurbsCommands:12,cancelledCommands:12,repeatedLaunchBlocked:true,keyboardNumericInput:keyboard,undoRedo:true,reload:true,displayReady:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'nurbs-edit-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
