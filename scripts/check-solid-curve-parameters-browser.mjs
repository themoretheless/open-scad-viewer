import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile,readdir} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-curve-parameters')
const keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
assert.ok(['system','dark','light','nord','solarized'].includes(theme))
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{
 try {
  const url=new URL(req.url,'http://localhost'),file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
  if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}
  if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const chunks=(await readdir(path.join(root,'assets'))).filter(n=>/^DirectModeler-[^/]+\.js$/.test(n));assert.equal(chunks.length,1)
const digest=async file=>createHash('sha256').update(await readFile(file)).digest('hex')
const artifacts={directModeler:{file:chunks[0],sha256:await digest(path.join(root,'assets',chunks[0]))},wasmSha256:await digest(path.join(root,'wasm/geometry-kernel.wasm')),sourceSha256:await digest('src/features/DirectModeler.vue')}
let browser,page
const renderErrors=[]
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true})
 page.on('pageerror',e=>renderErrors.push(String(e)));page.on('console',m=>{if(m.type()==='error')renderErrors.push(m.text())})
 await page.addInitScript(()=>{
  window.__profileWorkerRequests=0;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='profileEdit')window.__profileWorkerRequests++;return super.postMessage(message,...args)}}
  if(window.GPUCanvasContext){const configure=GPUCanvasContext.prototype.configure;GPUCanvasContext.prototype.configure=function(descriptor){if(this.canvas.className==='gpu-layer')window.__solidGpuDevice=descriptor.device;return configure.call(this,descriptor)}}
  if(!navigator.gpu)return
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const make=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const device=await make(...args);window.__qualificationGpuDevice=device;return device}}return adapter}
 })
 const origin=`http://127.0.0.1:${server.address().port}`
 await page.goto(origin)
 await page.getByRole('combobox',{name:'Тема',exact:true}).selectOption(theme)
 let tabPresses=0
 async function tabTo(locator){
  for(let i=0;i<1500;i++){
   if(await locator.evaluate(el=>el===document.activeElement))return
   await page.keyboard.press('Tab');tabPresses++
  }
  throw Error('Target is unreachable through sequential Tab navigation: '+await locator.getAttribute('aria-label'))
 }
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 async function activate(locator){
  if(keyboard){await tabTo(locator);await page.keyboard.press('Enter')}
  else await locator.click()
 }
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 let downloads=0
 const menu=solid.locator('summary[title="Файл"]')
 async function openMenu(){if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)}
 async function download(label,file,json=true){
  await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
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
 const arc=process.argv.includes('--arc'),curve={id:'editable',name:'Editable curve',closed:!arc,points:[],analytic:{kind:arc?'arc':'circle',center:[0,0],radius:2,start:0,sweep:arc?180:360}}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'curve.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[curve],bodies:[]}))})
 await solid.getByRole('button',{name:curve.name,exact:true}).waitFor();const before=await download('Скачать проект JSON','before.json');if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:curve.name,exact:true}))
 async function command(){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,'Curve parameters');await page.keyboard.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true});await command();await apply.click({trial:true})
 if(arc){for(const value of ['0','0.01','-0.05']){const jobs=await page.evaluate(()=>window.__profileWorkerRequests);await input(solid.getByRole('textbox',{name:'Угол дуги',exact:true}),value);await solid.getByRole('alert').filter({hasText:'Модуль угла дуги'}).waitFor();assert.equal(await apply.isDisabled(),true);assert.equal(await page.evaluate(()=>window.__profileWorkerRequests),jobs)}}
 for(const value of ['0','-1','1000001']){await input(solid.getByRole('textbox',{name:'Радиус / размер, мм',exact:true}),value);assert.equal(await apply.isDisabled(),true)}
 async function values(){for(const [name,value] of [['Радиус / размер, мм','6 mm'],['Центр X','2 cm'],['Центр Y','-5 mm'],['Начальный угол','30 deg']])await input(solid.getByRole('textbox',{name,exact:true}),value);if(arc)await input(solid.getByRole('textbox',{name:'Угол дуги',exact:true}),'-120 deg');await apply.click({trial:true})}
 await values();assert.deepEqual(await download('Скачать проект JSON','draft.json'),before);if(await menu.evaluate(e=>e.parentElement.open))await activate(menu);await page.keyboard.press('Escape');assert.deepEqual(await download('Скачать проект JSON','canceled.json'),before);if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command();await values();await page.screenshot({path:path.join(directory,'parameters-preview.png')});await activate(apply)
 const changed=await download('Скачать проект JSON','changed.json');assert.equal(changed.sketches[0].id,'editable');assert.deepEqual(changed.sketches[0].analytic,{kind:arc?'arc':'circle',center:[20,-5],radius:6,start:30,sweep:arc?-120:360})
 const points=changed.sketches[0].points;assert.ok(points.every(p=>Math.abs(Math.hypot(p[0]-20,p[1]+5)-6)<1e-9))
 const endpoint=angle=>[20+6*Math.cos(angle*Math.PI/180),-5+6*Math.sin(angle*Math.PI/180)]
 assert.ok(Math.hypot(...points[0].map((x,i)=>x-endpoint(30)[i]))<1e-9);if(arc)assert.ok(Math.hypot(...points.at(-1).map((x,i)=>x-endpoint(-90)[i]))<1e-9)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu);await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await download('Скачать проект JSON','undone.json'),before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu);await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await download('Скачать проект JSON','redone.json'),changed)
 await page.reload();await solid.getByRole('button',{name:curve.name,exact:true}).waitFor();await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});assert.deepEqual(await download('Скачать проект JSON','restored.json'),changed)
 assert.deepEqual(renderErrors,[]);assert.equal(await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible'),true)
 const report={artifacts,browser:browser.version(),arc,keyboard,tabPresses,invalidRadius:true,invalidArcSweep:arc,invalidSweepNoDispatch:arc,cancel:true,undoRedo:true,reloadExact:true,exactParameters:true,independentPointToleranceMm:1e-9};await writeFile(path.join(directory,'curve-parameters-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
