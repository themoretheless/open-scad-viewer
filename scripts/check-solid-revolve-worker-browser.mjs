import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
const extrude=process.argv.includes('--extrude'),gpu=process.env.SOLID_GPU_HEADED==='1'
const keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
assert.ok(['system','dark','light','nord','solarized'].includes(theme))
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{
 try {
  const url=new URL(req.url,'http://localhost');if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}
  const file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
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
 page.on('response',r=>{if(r.status()>=400)console.error('HTTP',r.status(),r.url())})
 page.on('pageerror',e=>renderErrors.push(String(e)));page.on('console',m=>{if(m.type()==='error')renderErrors.push(m.text())})
 await page.addInitScript(extrude=>{
  window.__failRevolve=false;window.__revolveRequests=0;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind===(extrude?'extrusion':'revolve')){window.__revolveRequests++;if(window.__failRevolve){window.__failRevolve=false;const callback=this.onmessage;this.onmessage=event=>callback?.call(this,{data:{...event.data,ok:false,error:{name:'Error',code:'CAD_CRASH',message:'Injected worker failure'}}})}}return super.postMessage(message,...args)}}
  window.__holdSolid=false;window.__lateSolid=null;window.__solidTerminated=false
  const PreviewWorker=window.Worker
  window.Worker=class extends PreviewWorker{
   postMessage(message,...args){
    if(window.__holdSolid&&message?.job?.kind===(extrude?'extrusion':'revolve')){
     this.held=true;const callback=this.onmessage
     this.onmessage=event=>{if(event.data?.ok===true)window.__lateSolid=fail=>callback?.call(this,fail?{data:{...event.data,ok:false,error:{name:'Error',code:'CAD_CRASH',message:'Late closed preview'}}}:event)}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.held)window.__solidTerminated=true;return super.terminate()}
  }
  if(!navigator.gpu)return
  window.__qualificationGpuDevices=new WeakMap()
  const configure=GPUCanvasContext.prototype.configure
  GPUCanvasContext.prototype.configure=function(config){window.__qualificationGpuDevices.set(this.canvas,config.device);return configure.call(this,config)}
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const make=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const device=await make(...args);window.__qualificationGpuDevice=device;return device}}return adapter}
 },extrude)
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
 const original={version:1,sketches:[{id:'profile',name:'Revolve profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[]}
 if(process.argv.includes('--occlusion')){
  const yaw=Math.PI/4,pitch=Math.atan(1/Math.sqrt(2)),right=[Math.cos(yaw),-Math.sin(yaw),0],up=[Math.sin(yaw)*Math.sin(pitch),Math.cos(yaw)*Math.sin(pitch),-Math.cos(pitch)],toward=[Math.sin(yaw)*Math.cos(pitch),Math.cos(yaw)*Math.cos(pitch),Math.sin(pitch)]
  const positions=[[-50,-50],[50,-50],[50,50],[-50,50]].flatMap(([x,y])=>right.map((v,i)=>x*v+y*up[i]+(process.argv.includes('--behind')?-100:100)*toward[i]))
  original.bodies.push({id:'occluder',name:'Occluder',mesh:{positions,indices:[0,1,2,0,2,3]}})
 }
 if(process.argv.includes('--group-change'))original.groups=[{name:'Destination',source:''}]
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'offset-source.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 const name=original.sketches[0].name;await solid.getByRole('button',{name,exact:true}).waitFor()
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name,exact:true}))
 await activate(solid.getByRole('region',{name:'2D — эскизы',exact:true}).getByRole('button',{name:'Вписать',exact:true}))
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}else{await search.fill(name);await search.press('Enter')}}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 if(process.argv.includes('--group-change')){
  await page.evaluate(()=>window.__holdSolid=true);await command(extrude?'Extrude':'Revolve')
  await page.waitForFunction(()=>typeof window.__lateSolid==='function')
  await page.evaluate(()=>{window.__oldGroupReply=window.__lateSolid;window.__lateSolid=null})
  await activate(solid.getByRole('button',{name:'Активная группа: Destination',exact:true}))
  await page.waitForFunction(()=>window.__solidTerminated)
  await page.evaluate(()=>{window.__oldGroupReply(false);window.__oldGroupReply(true)})
  assert.equal(await solid.locator('[data-preview-body]').count(),0)
  assert.equal(await apply.isDisabled(),true)
  assert.deepEqual(await download('Скачать проект JSON','group-stale.json'),before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await page.keyboard.press('Escape');await page.evaluate(()=>window.__holdSolid=false)
 }
 if(process.argv.includes('--context-switch')){
  await page.evaluate(()=>window.__holdSolid=true)
  await command(extrude?'Extrude':'Revolve')
  await page.waitForFunction(()=>typeof window.__lateSolid==='function')
  await page.getByRole('button',{name:'Mesh',exact:true}).click()
  await page.waitForFunction(()=>window.__solidTerminated)
  await page.evaluate(()=>{window.__holdSolid=false;window.__lateSolid(false);window.__lateSolid(true)})
  await page.getByRole('button',{name:'Solid',exact:true}).click()
  assert.equal(await solid.locator('[data-preview-body]').count(),0)
  assert.deepEqual(await download('Скачать проект JSON','context-restored.json'),before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 }
 await command(extrude?'Extrude':'Revolve')
 if(!extrude){
 const geometry=solid.locator('label').filter({hasText:/^Поверхности вращения/}).locator('select')
 if(keyboard){await tabTo(geometry);await page.keyboard.press('End');await page.keyboard.press('Enter');await page.keyboard.press('Tab');assert.equal(await geometry.inputValue(),'exact')}
 else await geometry.selectOption('exact')
 await input(solid.getByLabel(/^Смещение оси, мм/),'-5 mm')
 await input(solid.getByLabel(/^Угол/),'180')
 }
 const amount=solid.getByLabel(extrude?/^Высота, мм/:/^Угол/),value=extrude?'12':'360'
 await input(amount,value)
 await apply.click({trial:true})
 let previewHash
 if(gpu){
  await solid.locator('canvas.gpu-layer').waitFor({state:'visible'})
  assert.ok(await solid.locator('[data-preview-body]').count()>0)
  assert.ok(await solid.locator('[data-preview-body]').evaluateAll(nodes=>nodes.every(n=>getComputedStyle(n).fill==='rgba(0, 0, 0, 0)'&&getComputedStyle(n).stroke==='none')))
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))
  previewHash=createHash('sha256').update(await solid.locator('canvas.gpu-layer').screenshot({style:'body * {visibility:hidden!important} canvas.gpu-layer {visibility:visible!important}'})).digest('hex')
 }
 if(!gpu&&process.argv.includes('--occlusion')){
  const top=await solid.locator('polygon[fill="#75e4b8"]').first().evaluate(n=>{
   const point=new DOMPoint(Array.from(n.points).reduce((s,p)=>s+p.x,0)/3,Array.from(n.points).reduce((s,p)=>s+p.y,0)/3)
   const candidates=Array.from(n.ownerSVGElement.querySelectorAll('[data-preview-body],[data-body]')).filter(p=>getComputedStyle(p).display!=='none'&&p.isPointInFill(point))
   const last=candidates.at(-1);return {body:last?.getAttribute('data-body'),preview:!!last?.getAttribute('data-preview-body')}
  })
  if(process.argv.includes('--behind'))assert.equal(top.preview,true);else assert.equal(top.body,'occluder')
  await page.screenshot({path:path.join(directory,'preview-occlusion.png')})
 }
 const readyRequests=await page.evaluate(()=>window.__revolveRequests)
 if(!gpu)assert.ok(await solid.locator('polygon[fill="#75e4b8"]').count()>0)
 await input(amount,'-')
 assert.equal(await apply.isDisabled(),true)
 await page.waitForFunction(()=>!document.querySelector('polygon[fill="#75e4b8"]'))
 assert.equal(await page.evaluate(()=>window.__revolveRequests),readyRequests)
 if(gpu){
  await page.waitForFunction(()=>!document.querySelector('[data-preview-body]'))
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))
  const clearHash=createHash('sha256').update(await solid.locator('canvas.gpu-layer').screenshot({style:'body * {visibility:hidden!important} canvas.gpu-layer {visibility:visible!important}'})).digest('hex')
  if(process.argv.includes('--occlusion')&&!process.argv.includes('--behind'))assert.equal(previewHash,clearHash,'Opaque foreground must completely hide GPU preview')
  else assert.notEqual(previewHash,clearHash,'Visible GPU preview must change the rendered frame')
 }

 if(process.argv.includes('--fail-preview'))await page.evaluate(()=>window.__failRevolve=true)
 await input(amount,value)
 if(process.argv.includes('--fail-preview')){
  await solid.getByRole('alert').filter({hasText:'Вычисление прервано из-за сбоя'}).waitFor()
  assert.equal(await apply.isDisabled(),true)
  assert.deepEqual(await download('Скачать проект JSON','failed.json'),before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await page.screenshot({path:path.join(directory,'revolve-error.png')})
  await activate(solid.getByRole('button',{name:'Повторить вычисление',exact:true}))
 }
 await apply.click({trial:true})
 const requests=await page.evaluate(()=>window.__revolveRequests);assert.equal(requests,readyRequests+(process.argv.includes('--fail-preview')?2:1))
 if(process.argv.includes('--lose-device')){
  assert.ok(gpu)
  await page.evaluate(()=>{const device=window.__qualificationGpuDevices.get(document.querySelector('canvas.gpu-layer'));if(!device)throw Error('No Solid GPU device captured');device.destroy()})
  await solid.locator('canvas.gpu-layer').waitFor({state:'hidden'})
  await solid.getByRole('status').filter({hasText:'WebGPU отключён'}).waitFor()
  await page.getByText('CPU · SVG',{exact:true}).waitFor()
  assert.equal(await page.locator('.status-item').filter({hasText:'CPU · SVG'}).locator('.status-dot').getAttribute('class'),'status-dot off')
  assert.ok(await solid.locator('polygon[fill="#75e4b8"]').count()>0)
  assert.deepEqual(await download('Скачать проект JSON','after-device-loss.json'),before)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 }

 await page.screenshot({path:path.join(directory,'revolve-preview.png')})
 await activate(apply)
 assert.equal(await page.evaluate(()=>window.__revolveRequests),requests)
 const changed=await download('Скачать проект JSON','revolved.json');assert.equal(changed.bodies.length,original.bodies.length+1);assert.ok(changed.bodies.at(-1).brep);if(process.argv.includes('--group-change'))assert.equal(changed.bodies.at(-1).group,'Destination')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const step=await download('STEP выбранного тела · текущая геометрия','browser-revolve.step',false)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Worker revolve',file:'browser-revolve.step',sha256:createHash('sha256').update(step).digest('hex'),expected:extrude?{volumeMm3:1200,boundsMm:[[0,0,0],[10,10,12]]}:{volumeMm3:2000*Math.PI,boundsMm:[[-20,0,-15],[10,10,15]]}}]}))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone.bodies,before.bodies)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','redone.json');assert.deepEqual(redone.bodies,changed.bodies)

 if(process.argv.includes('--lose-device')){
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  const modes=page.getByRole('group',{name:'Режим работы',exact:true})
  await modes.getByRole('button',{name:'Mesh',exact:true}).click()
  await page.getByText('CPU · SVG',{exact:true}).waitFor({state:'hidden'})
  await modes.getByRole('button',{name:'Solid',exact:true}).click()
  await page.getByText('CPU · SVG',{exact:true}).waitFor()
  assert.deepEqual(await download('Скачать проект JSON','after-mode-switch.json'),redone)
 }
 assert.deepEqual(renderErrors,[])
 const report={gpu,deviceLoss:process.argv.includes('--lose-device'),occlusion:process.argv.includes('--occlusion'),bodyBehind:process.argv.includes('--behind'),operation:extrude?'extrusion':'revolve',browser:browser.version(),workerRequests:requests,retryFailure:process.argv.includes('--fail-preview'),applyWithoutRecompute:true,invalidQuantityClearsPreview:true,undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'revolve-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
