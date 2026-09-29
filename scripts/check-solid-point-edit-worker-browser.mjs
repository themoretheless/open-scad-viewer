import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
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
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='pointEdit'){window.__sceneEditRequests++;window.__transformOptions.push(message.job.options)}return super.postMessage(message,...args)}}
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
 const fixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-scene-edit/mouse/before.json','utf8'))
 const original={version:1,sketches:[],bodies:[{id:'mesh',name:'Mesh',mesh:fixture.bodies[0].mesh}]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command('NURBS surface')
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 for(const [label,value,message] of [['Вес','0','Введите положительный вес контрольной точки.'],['X','','Введите конечное число в поле X.']]){
  const field=solid.getByLabel(label,{exact:true}),previous=await field.inputValue()
  await tabTo(field);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.press('Backspace')
  if(value)await page.keyboard.type(value)
  await page.keyboard.press('Tab')
  const button=solid.getByRole('button',{name:'Применить CV',exact:true});await tabTo(button);await page.keyboard.press('Enter')
  await solid.getByText(message,{exact:false}).first().waitFor()
  assert.equal(await field.getAttribute('aria-invalid'),'true')
  assert.equal(await field.evaluate(el=>el===document.activeElement),true)
  assert.deepEqual(await download('Скачать проект JSON',label==='X'?'invalid-coordinate.json':'invalid-weight.json'),before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await field.fill(previous)
 }
 // Reach and edit exact CV fields through sequential keyboard focus.
 const numeric={X:'12.5',Y:'-7.25',Z:'3.125','Вес':'0.75'}
 for(const [label,value] of Object.entries(numeric)){
  const input=solid.getByLabel(label,{exact:true})
  await tabTo(input);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.type(value);await page.keyboard.press('Tab')
 }
 const cvApply=solid.getByRole('button',{name:'Применить CV',exact:true})
 await tabTo(cvApply);await page.keyboard.press('Enter')
 await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
 const exact=await download('Скачать проект JSON','numeric-cv.json')
 assert.deepEqual(exact.surfaces[0].surface.controlPoints[0][0],[12.5,-7.25,3.125])
 assert.equal(exact.surfaces[0].surface.weights[0][0],.75)
 assert.equal(exact.surfaces[0].id,before.surfaces[0].id)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 assert.deepEqual(await download('Скачать проект JSON','numeric-cv-undone.json'),before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 assert.deepEqual(await download('Скачать проект JSON','numeric-cv-redone.json'),exact)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 await ready()
 for(const kind of ['cv','vertices']){
  if(kind==='vertices'){await activate(solid.getByRole('button',{name:'Mesh',exact:true}));await command('Vertices')}
  async function drag(cancel){
   const handle=kind==='cv'?solid.locator('.nurbs-cage circle').first():solid.locator('svg[aria-label="Холст тел 3D"] circle[style*="cursor"]').first()
   const box=await handle.boundingBox();assert.ok(box)
   const x=box.x+box.width/2,y=box.y+box.height/2
   await page.keyboard.down('Alt');await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+20,y+15,{steps:6})
   await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
   await ready()
   if(kind==='cv')assert.ok(await solid.locator('[data-surface]').count()>0,'Surface remains visible after control point preview')
   if(cancel)await page.keyboard.press('Escape')
   else await page.screenshot({path:path.join(directory,kind+'-preview.png')})
   await page.mouse.up();await page.keyboard.up('Alt')
   await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
  }
  await drag(true)
  const canceled=await download('Скачать проект JSON',kind+'-canceled.json');assert.deepEqual(canceled.bodies,before.bodies);assert.deepEqual(canceled.surfaces,before.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await drag(false)
  const changed=await download('Скачать проект JSON',kind+'.json')
  if(kind==='cv'){
   assert.notDeepEqual(changed.surfaces[0].surface.controlPoints,before.surfaces[0].surface.controlPoints)
   assert.equal(changed.surfaces[0].id,before.surfaces[0].id);assert.deepEqual(changed.bodies,before.bodies)
  }else{assert.notDeepEqual(changed.bodies[0].mesh.positions,before.bodies[0].mesh.positions);assert.equal(changed.bodies[0].id,'mesh');assert.deepEqual(changed.surfaces,before.surfaces)}
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}));const undone=await download('Скачать проект JSON',kind+'-undone.json');assert.deepEqual(undone.bodies,before.bodies);assert.deepEqual(undone.surfaces,before.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↷',exact:true}));const redone=await download('Скачать проект JSON',kind+'-redone.json');assert.deepEqual(redone.bodies,changed.bodies);assert.deepEqual(redone.surfaces,changed.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
 }
 const requests=await page.evaluate(()=>window.__sceneEditRequests);assert.ok(requests>=4)
 const restored=await download('Скачать проект JSON','before-reload.json')
 await page.reload();await ready()
 const reloaded=await download('Скачать проект JSON','reloaded.json')
 assert.deepEqual(reloaded,restored)
 assert.ok(await solid.locator('[data-surface]').count()>0,'Restored surface has display geometry')
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,meshVerticesAndCv:true,numericCvKeyboard:true,invalidCvRefused:true,cancel:true,undoRedo:true,reload:true,displayVerified:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'point-edit-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
