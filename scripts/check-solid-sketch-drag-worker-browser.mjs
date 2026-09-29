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
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(['pointEdit','sceneEdit'].includes(message?.job?.kind)){window.__sceneEditRequests++;window.__transformOptions.push(message.job.options)}return super.postMessage(message,...args)}}
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
 const circle={kind:'circle',center:[20,5],radius:3,start:0,sweep:360},arc={kind:'arc',center:[32,5],radius:4,start:30,sweep:120}
 const sample=a=>Array.from({length:a.kind==='circle'?64:33},(_,i)=>{const t=(a.start+a.sweep*i/(a.kind==='circle'?64:32))*Math.PI/180;return [a.center[0]+a.radius*Math.cos(t),a.center[1]+a.radius*Math.sin(t)]})
 const original={version:1,sketches:[{id:'profile',name:'Profile',closed:true,points:[[0,0],[8,0],[8,8],[0,8]]},{id:'circle',name:'Circle',closed:true,analytic:circle,points:sample(circle)},{id:'arc',name:'Arc',closed:false,analytic:arc,points:sample(arc)}],bodies:[{...fixture.bodies[0],id:'body',name:'Body'}]}

 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const svg=solid.locator('svg[aria-label="Холст эскизов 2D"]')
 for(const kind of ['mixed','vertex','radius','arc-start']){
  const name=kind==='radius'?'Circle':kind==='arc-start'?'Arc':'Profile'
  await activate(solid.getByRole('button',{name,exact:true}))
  if(kind==='mixed'){
   const body=solid.getByRole('button',{name:'Body',exact:true})
   if(keyboard){await tabTo(body);await page.keyboard.down('Shift');await page.keyboard.press('Enter');await page.keyboard.up('Shift')}else await body.click({modifiers:['Shift']})
  }
  async function drag(cancel){
   const handle=kind==='mixed'?svg.locator('path[d="M 0,0 L 8,0 L 8,-8 L 0,-8 Z"]'):kind==='vertex'?svg.locator('circle[fill="var(--accent)"]').first():kind==='radius'?svg.locator('circle[fill="#77eac5"][style*="cursor"]'):svg.locator('circle[fill="#ffb877"]').first()
   const box=await handle.boundingBox();assert.ok(box)
   const x=box.x+box.width/2,y=box.y+box.height/2
   await page.keyboard.down('Alt');await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+18,y+9,{steps:6})
   await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
   if(cancel)await page.keyboard.press('Escape');else await page.screenshot({path:path.join(directory,kind+'-preview.png')})
   await page.mouse.up();await page.keyboard.up('Alt')
   await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
  }
  await drag(true)
  const canceled=await download('Скачать проект JSON',kind+'-canceled.json');assert.deepEqual(canceled.bodies,before.bodies);assert.deepEqual(canceled.sketches,before.sketches)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await drag(false)
  const changed=await download('Скачать проект JSON',kind+'.json')
  assert.deepEqual(changed.sketches.map(s=>s.id),before.sketches.map(s=>s.id))
  if(kind==='mixed'){
   const delta=changed.sketches[0].points[0].map((x,i)=>x-before.sketches[0].points[0][i]);assert.ok(Math.hypot(...delta)>0.1)
   for(let j=0;j<before.bodies[0].mesh.positions.length;j++)assert.ok(Math.abs(changed.bodies[0].mesh.positions[j]-before.bodies[0].mesh.positions[j]-(j%3===2?0:delta[j%3]))<1e-7)
   assert.deepEqual(changed.bodies[0].brep.topologyIds,before.bodies[0].brep.topologyIds)
  }else{
   assert.deepEqual(changed.bodies,before.bodies)
   if(kind==='vertex'){assert.notDeepEqual(changed.sketches[0].points[0],before.sketches[0].points[0]);assert.deepEqual(changed.sketches[0].points.slice(1),before.sketches[0].points.slice(1))}
   if(kind==='radius'){assert.notEqual(changed.sketches[1].analytic.radius,before.sketches[1].analytic.radius);assert.deepEqual(changed.sketches[1].analytic.center,before.sketches[1].analytic.center)}
   if(kind==='arc-start'){assert.notEqual(changed.sketches[2].analytic.start,before.sketches[2].analytic.start);assert.ok(Math.abs(changed.sketches[2].analytic.start+changed.sketches[2].analytic.sweep-150)<1e-7)}
  }
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}));const undone=await download('Скачать проект JSON',kind+'-undone.json');assert.deepEqual(undone.bodies,before.bodies);assert.deepEqual(undone.sketches,before.sketches)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↷',exact:true}));const redone=await download('Скачать проект JSON',kind+'-redone.json');assert.deepEqual(redone.bodies,changed.bodies);assert.deepEqual(redone.sketches,changed.sketches)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
 }
 const requests=await page.evaluate(()=>window.__sceneEditRequests);assert.ok(requests>=8)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,mixedAndSketchHandles:true,cancel:true,undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'sketch-drag-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
