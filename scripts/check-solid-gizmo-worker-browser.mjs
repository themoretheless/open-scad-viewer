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
  window.__sceneEditRequests=0;window.__transformOptions=[];const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='sceneEdit'){window.__sceneEditRequests++;window.__transformOptions.push(message.job.options)}return super.postMessage(message,...args)}}
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
 await solid.getByRole('button',{name:'Куб · 3D',exact:true}).waitFor()
 await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'})
 await command('Create linked instance');await activate(apply)
 await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 await command('Select instance source')
 const before=await download('Скачать проект JSON','before.json');assert.equal(before.bodies.length,2)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 let numericUnits=false
 if(process.argv.includes('--numeric-units')){
  await activate(solid.getByRole('tab',{name:'Свойства',exact:true}))
  const x=solid.getByRole('textbox',{name:'ΔX',exact:true})
  async function enter(locator,value){await tabTo(locator);await page.keyboard.press('ControlOrMeta+a');await page.keyboard.insertText(value)}
  const numericApply=solid.getByRole('button',{name:'Применить',exact:true})
  await enter(x,'bad');assert.equal(await x.getAttribute('aria-invalid'),'true');assert.equal(await numericApply.isDisabled(),true)
  await page.screenshot({path:path.join(directory,'numeric-error.png')})
  await enter(x,'2 cm')
  await enter(solid.getByRole('textbox',{name:'Поворот Z',exact:true}),'0.5 rad')
  await enter(solid.getByRole('textbox',{name:'Масштаб',exact:true}),'1.5')
  await activate(numericApply)
  await page.waitForFunction(()=>document.querySelector('input[aria-label="ΔX"]')?.value==='0')
  const option=await page.evaluate(()=>window.__transformOptions.at(-1))
  assert.equal(option.x,20);assert.ok(Math.abs(option.angle-0.5*180/Math.PI)<1e-10);assert.equal(option.scale,1.5)
  const changed=await download('Скачать проект JSON','numeric-applied.json');assert.notDeepEqual(changed.bodies[0].mesh.positions,before.bodies[0].mesh.positions)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  assert.deepEqual((await download('Скачать проект JSON','numeric-undone.json')).bodies,before.bodies)
  await activate(solid.getByRole('button',{name:'↷',exact:true}))
  assert.deepEqual((await download('Скачать проект JSON','numeric-redone.json')).bodies,changed.bodies)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  numericUnits=true
 }
 const parts=[]
 for(const kind of ['rotate','scale']){
  await command('Gizmo: '+kind)
  async function drag(cancel){
   const svg=solid.locator('svg[aria-label="Холст тел 3D"]')
   const handle=kind==='rotate'?svg.locator('polyline[stroke="#ff7777"][style*="cursor"]'):svg.locator('g[style*="cursor"] > circle[fill="#ff7777"]')
   const point=await handle.evaluate((el,rotate)=>{let p;if(rotate){const pairs=el.getAttribute('points').trim().split(/\s+/).map(v=>v.split(',').map(Number));p=pairs[8]}else p=[Number(el.getAttribute('cx')),Number(el.getAttribute('cy'))];const q=new DOMPoint(...p).matrixTransform(el.getScreenCTM());return {x:q.x,y:q.y}},kind==='rotate')
   await page.keyboard.down('Alt');await page.mouse.move(point.x,point.y);await page.mouse.down();await page.mouse.move(point.x+30,point.y+20,{steps:8})
   await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
   if(cancel)await page.keyboard.press('Escape')
   else await page.screenshot({path:path.join(directory,kind+'-preview.png')})
   await page.mouse.up();await page.keyboard.up('Alt')
   await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
  }
  await drag(true)
  const canceled=await download('Скачать проект JSON',kind+'-canceled.json');assert.deepEqual(canceled.bodies,before.bodies)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await drag(false)
  const options=await page.evaluate(()=>window.__transformOptions.at(-1))
  assert.equal(options.operation,'transform');assert.ok(kind==='rotate'?Math.abs(options.angle)>0.1:Math.abs(options.scale-1)>0.01)
  const changed=await download('Скачать проект JSON',kind+'.json');assert.notDeepEqual(changed.bodies[0].mesh.positions,before.bodies[0].mesh.positions)
  assert.equal(changed.bodies[0].id,before.bodies[0].id);assert.deepEqual(changed.bodies[1].instance,before.bodies[1].instance)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  const step=await download('STEP выбранного тела · текущая геометрия',kind+'.step',false)
  const positions=changed.bodies[0].mesh.positions,bounds=[[],[]]
  for(let axis=0;axis<3;axis++){const values=positions.filter((_,i)=>i%3===axis);bounds[0].push(Math.min(...values));bounds[1].push(Math.max(...values))}
  parts.push({name:'Worker gizmo '+kind,file:kind+'.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:8000*options.scale**3,boundsMm:bounds}})
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}));const undone=await download('Скачать проект JSON',kind+'-undone.json');assert.deepEqual(undone.bodies,before.bodies)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↷',exact:true}));const redone=await download('Скачать проект JSON',kind+'-redone.json');assert.deepEqual(redone.bodies,changed.bodies)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 }
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts}))
 const requests=await page.evaluate(()=>window.__sceneEditRequests)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,rotateScale:true,cancel:true,undoRedo:true,keyboard,tabPresses,downloads,numericUnits}
 await writeFile(path.join(directory,'gizmo-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
