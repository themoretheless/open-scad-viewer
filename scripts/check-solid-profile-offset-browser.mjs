import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile,readdir} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
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
 const source=process.argv.find(a=>a.startsWith('--source='))?.slice(9)??'/tmp/cad-offset-step/source.json'
 const original=JSON.parse(await readFile(source,'utf8'))
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'offset-source.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 const name=original.sketches[0].name;await solid.getByRole('button',{name,exact:true}).waitFor()
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name,exact:true}))
 await activate(solid.getByRole('region',{name:'2D — эскизы',exact:true}).getByRole('button',{name:'Вписать',exact:true}))
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,name);await page.keyboard.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command('Offset')
 const amount=solid.getByLabel('Расстояние, мм',{exact:true})
 await input(amount,'-10 mm');assert.equal(await apply.isDisabled(),true);await solid.getByText('Смещение удаляет профиль. Уменьшите модуль расстояния.',{exact:true}).first().waitFor()
 await page.screenshot({path:path.join(directory,'offset-refusal.png')})
 await input(amount,'0.05 cm');await apply.click({trial:true});assert.equal(await amount.inputValue(),'0.05 cm')
 const preview=solid.locator('[data-preview="offset-profile"]');assert.equal(await preview.count(),1)
 assert.equal((await preview.getAttribute('d')).match(/M /g).length,2)
 await page.screenshot({path:path.join(directory,'offset-preview.png')})
 await activate(solid.getByRole('button',{name:'Esc',exact:true}))
 const canceled=await download('Скачать проект JSON','canceled.json');assert.deepEqual(canceled,before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Offset');await activate(apply)
 const changed=await download('Скачать проект JSON','offset.json');assert.equal(changed.sketches[0].id,before.sketches[0].id);assert.equal(changed.sketches[0].retainedProfile.loops.length,2);assert.ok(Math.abs(changed.sketches[0].retainedProfile.areaMm2-62)<1e-7)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone,before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await download('Скачать проект JSON','redone.json'),changed)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}));if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'Повтор · Shift R',exact:true}));await activate(apply)
 const repeated=await download('Скачать проект JSON','repeated.json');assert.deepEqual(repeated,changed)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Extrude');await input(solid.getByLabel('Высота, мм',{exact:true}),'5 mm');await activate(apply)
 const extruded=await download('Скачать проект JSON','extruded.json');assert.equal(extruded.bodies.length,1)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('region',{name:'3D — тела',exact:true}).getByRole('button',{name:'Вписать',exact:true}))
 assert.equal(await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible'),true)
 await page.evaluate(()=>window.__solidGpuDevice.destroy());await solid.getByRole('button',{name:'Повторить WebGPU',exact:true}).waitFor();await solid.locator('[data-body]').first().waitFor({state:'visible'});assert.deepEqual(renderErrors,[])
 assert.deepEqual(await download('Скачать проект JSON','gpu-failed.json'),extruded);if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'Повторить WebGPU',exact:true}));await solid.getByRole('status',{name:'gpu-recovery',exact:true}).waitFor({state:'hidden'});assert.equal(await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible'),true)
 assert.deepEqual(await download('Скачать проект JSON','gpu-recovered.json'),extruded);if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await page.screenshot({path:path.join(directory,'offset-extruded.png')})
 const step=await download('STEP выбранного тела · текущая геометрия','browser-offset.step',false)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Browser offset plate',file:'browser-offset.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:310,boundsMm:[[-.5,-.5,0],[8.5,6.5,5]]}}]}))
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'offset.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(changed))})
 await solid.getByRole('button',{name:extruded.bodies[0].name,exact:true}).waitFor({state:'hidden'})
 const reloaded=await download('Скачать проект JSON','reloaded.json');assert.deepEqual(reloaded,changed)
 const workerRequests=await page.evaluate(()=>window.__profileWorkerRequests);assert.ok(workerRequests>0)
 await page.reload();await solid.getByRole('button',{name,exact:true}).waitFor();await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});assert.deepEqual(await download('Скачать проект JSON','restored.json'),changed)
 assert.deepEqual(renderErrors,[])
 const report={artifacts,redo:true,browserReloadExact:true,gpuRecovered:true,completeDocumentChecks:true,workerRequests,browser:browser.version(),roundOffset:true,holesPreviewed:true,emptyRefusal:true,units:true,cancel:true,undo:true,repeat:true,jsonReload:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'offset-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
