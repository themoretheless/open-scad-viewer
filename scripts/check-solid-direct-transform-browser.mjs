import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
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
  window.__sceneEditRequests=0;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='sceneEdit')window.__sceneEditRequests++;return super.postMessage(message,...args)}}
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
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command('Box')
 await command('Create linked instance');await activate(apply)
 await command('Select instance source')
 const before=await download('Скачать проект JSON','before.json');assert.equal(before.bodies.length,2)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('tab',{name:'Свойства',exact:true}))
 await solid.getByLabel('ΔX',{exact:true}).fill('5')
 const beforeRequests=await page.evaluate(()=>window.__sceneEditRequests)
 await activate(solid.getByRole('button',{name:'Применить',exact:true}))
 await page.waitForFunction(()=>document.querySelector('input[aria-label="ΔX"]')?.value==='0')
 const requests=await page.evaluate(()=>window.__sceneEditRequests);assert.equal(requests,beforeRequests+1)
 await page.screenshot({path:path.join(directory,'direct-transform.png')})
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
 if(!keyboard){
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  const tip=solid.locator('svg[aria-label="Холст тел 3D"] g[style*="cursor"] > circle[fill="#ff7777"]')
  const box=await tip.boundingBox();assert.ok(box)
  const x=box.x+box.width/2,y=box.y+box.height/2
  const priorRequests=await page.evaluate(()=>window.__sceneEditRequests)
  await page.keyboard.down('Alt');await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+30,y,{steps:5});
  for(const body of redone.bodies){const offsets=await solid.locator(`[data-body="${body.id}"][transform]`).evaluateAll(nodes=>nodes.map(n=>n.getAttribute('transform')));assert.ok(offsets.length>0);assert.ok(offsets.every(v=>v.startsWith('translate(')))}
  await page.screenshot({path:path.join(directory,'translation-preview.png')});await page.mouse.up();await page.keyboard.up('Alt')
  await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
  assert.equal(await page.evaluate(()=>window.__sceneEditRequests),priorRequests+1)
  const dragged=await download('Скачать проект JSON','dragged.json')
  const delta=dragged.bodies[0].mesh.positions[0]-redone.bodies[0].mesh.positions[0];assert.ok(Math.abs(delta)>0.1)
  assert.deepEqual(dragged.bodies[1].instance,redone.bodies[1].instance)
  for(let i=0;i<1;i++)for(let j=0;j<redone.bodies[i].mesh.positions.length;j++)assert.ok(Math.abs(dragged.bodies[i].mesh.positions[j]-redone.bodies[i].mesh.positions[j]-(j%3===0?delta:0))<1e-7)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}));const dragUndone=await download('Скачать проект JSON','drag-undone.json');assert.deepEqual(dragUndone.bodies,redone.bodies)
 }
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,numericTransformWorker:true,mouseDrag:!keyboard,undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'direct-transform-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
