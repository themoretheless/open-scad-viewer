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
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='restoreDocument'&&sessionStorage.getItem('holdRecovery')){window.__recoveryHeld=true;window.__releaseRecovery=()=>super.postMessage(message,...args);return}if(['nurbsEdit','brepTool'].includes(message?.job?.kind)){window.__sceneEditRequests++;window.__transformOptions.push(message.job.options);if(window.__bridgeFail&&message.job.options?.kind==='bridge'){window.__bridgeFail=false;queueMicrotask(()=>this.onerror?.({message:'Injected G2 failure'}));return}if(window.__bridgeHold&&message.job.options?.kind==='bridge'){this.held=true;window.__bridgeHeld=true;return}}return super.postMessage(message,...args)}terminate(){if(this.held)window.__bridgeTerminated=true;return super.terminate()}}
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
  await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
  await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})
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
 const curveFixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-native-nurbs/mouse/before.json','utf8'))
 const a=structuredClone(curveFixture.curves[0]),b=structuredClone(a);a.id='a';a.name='First';b.id='b';b.name='Second';b.curve.controlPoints.forEach(p=>p[0]+=60)
 const original={version:1,sketches:[],bodies:[fixture.bodies[0]],curves:[a,b],...(process.argv.includes('--group-change')?{groups:[{name:'Destination',source:''}]}:{})}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'bridge.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 await activate(solid.getByRole('button',{name:'First',exact:true}));await page.keyboard.down('Shift');await activate(solid.getByRole('button',{name:'Second',exact:true}));await page.keyboard.up('Shift')
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const createBridge=solid.getByRole('button',{name:'Создать переход G2',exact:true})
 const tensionInput=solid.getByRole('spinbutton',{name:'Натяжение',exact:true})
 async function setTension(value){if(keyboard){await tabTo(tensionInput);await page.keyboard.press('ControlOrMeta+a');if(value)await page.keyboard.insertText(value);else await page.keyboard.press('Backspace')}else await tensionInput.fill(value)}
 for(const value of ['', '0', '-1', '10.1']){
  await setTension(value)
  await solid.getByRole('alert').filter({hasText:'Введите натяжение от 0,01 до 10.'}).waitFor()
  assert.equal(await tensionInput.getAttribute('aria-invalid'),'true')
  assert.equal(await createBridge.getAttribute('aria-disabled'),'true')
  assert.equal(await page.evaluate(()=>window.__sceneEditRequests),0)
 }
 await setTension('1');assert.equal(await tensionInput.getAttribute('aria-invalid'),'false')
 for(const cancel of ['escape','parameter',...(process.argv.includes('--group-change')?['group']:[])]){
  await page.evaluate(()=>{window.__bridgeHold=true;window.__bridgeHeld=false;window.__bridgeTerminated=false})
  await activate(createBridge);await page.waitForFunction(()=>window.__bridgeHeld)
  if(cancel==='escape')await page.keyboard.press('Escape')
  else if(cancel==='group')await activate(solid.getByRole('button',{name:'Активная группа: Destination',exact:true}))
  else {
   const tension=solid.getByRole('spinbutton',{name:'Натяжение',exact:true})
   if(keyboard){await tabTo(tension);await page.keyboard.press('ControlOrMeta+a');await page.keyboard.insertText('1.5')}else await tension.fill('1.5')
  }
  await page.waitForFunction(()=>window.__bridgeTerminated)
  assert.deepEqual(await download('Скачать проект JSON','cancel-'+cancel+'.json'),before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 }
 await page.evaluate(()=>window.__bridgeHold=false)
 await page.evaluate(()=>window.__bridgeFail=true);await activate(createBridge)
 await solid.getByRole('alert').filter({hasText:'Не удалось создать переход G2.'}).waitFor()
 assert.deepEqual(await download('Скачать проект JSON','failed-bridge.json'),before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)

 await activate(solid.getByRole('button',{name:'Создать переход G2',exact:true}))
 await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется NURBS-команда.'))
 const bridge=await download('Скачать проект JSON','bridge.json');assert.equal(bridge.curves.length,3);assert.deepEqual(new Set([bridge.curves[2].bridge.sourceA,bridge.curves[2].bridge.sourceB]),new Set(['a','b']))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await page.screenshot({path:path.join(directory,'bridge.png')})
 await activate(solid.getByRole('button',{name:'↶',exact:true}));const undone=await download('Скачать проект JSON','bridge-undone.json');assert.deepEqual(undone,before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}));const redone=await download('Скачать проект JSON','bridge-redone.json');assert.deepEqual(redone,bridge)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const requestsBeforeReload=await page.evaluate(()=>window.__sceneEditRequests)
 await page.evaluate(()=>sessionStorage.setItem('holdRecovery','1'))
 await page.reload()
 await page.waitForFunction(()=>window.__recoveryHeld)
 assert.equal(await solid.getByRole('button',{name:'Скачать проект JSON',exact:true,includeHidden:true}).isDisabled(),true)
 for(const name of ['SVG выбранного тела · проекция XY','Экспорт для Blender','STEP выбранного тела · текущая геометрия','STEP сборки · тела и группы','Экспорт AP242-оригинала'])assert.equal(await solid.getByRole('button',{name,exact:true,includeHidden:true}).isDisabled(),true)
 await solid.getByRole('status',{name:'draft-recovery',exact:true}).waitFor()
 await page.evaluate(()=>{sessionStorage.removeItem('holdRecovery');window.__releaseRecovery()})
 await solid.getByRole('button',{name:'G2 bridge',exact:true}).waitFor()
 await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 const restored=await download('Скачать проект JSON','bridge-restored.json');assert.deepEqual(restored,bridge)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:fixture.bodies[0].name,exact:true}))
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 await command('B-rep detail')
 await activate(solid.getByRole('button',{name:'Свойства B-rep',exact:true}))
 await page.waitForFunction(()=>document.body.innerText.includes('mm³ · A ='))
 const properties=await solid.locator('small').filter({hasText:'mm³ · A ='}).innerText()
 assert.match(properties,/V = 8000\.0000/)
 const measured=await download('Скачать проект JSON','measured.json');assert.deepEqual(measured,bridge)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.locator('.exact-detail input').fill('8')
 await activate(solid.getByRole('button',{name:'Перестроить mesh',exact:true}))
 await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется B-rep.'))
 const mesh=await download('Скачать проект JSON','detail.json');assert.deepEqual(mesh.bodies[0].brep,bridge.bodies[0].brep);assert.equal(mesh.bodies[0].id,bridge.bodies[0].id)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await page.screenshot({path:path.join(directory,'detail.png')})
 const detailUnchanged=await download('Скачать проект JSON','detail-unchanged.json');assert.deepEqual(detailUnchanged,bridge)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Body diagnostics')
 await page.waitForFunction(()=>document.body.innerText.includes('Открытых рёбер: 0'))
 assert.ok(await solid.locator('[data-diagnostic="section"]').count()>0)
 await page.screenshot({path:path.join(directory,'diagnostics.png')})
 const requests=requestsBeforeReload+await page.evaluate(()=>window.__sceneEditRequests);assert.equal(requests,process.argv.includes('--group-change')?8:7)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,bridgeAndBrepDiagnostics:true,properties,undoRedo:true,escapeCancellation:true,parameterCancellation:true,groupCancellation:process.argv.includes('--group-change'),invalidTension:true,workerFailureRetry:true,bridgeReload:true,exportBlockedDuringRecovery:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'bridge-diagnostics-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
