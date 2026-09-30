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
  window.__failSketch=false;window.__sketchEditRequests=0;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='sketchEdit'){window.__sketchEditRequests++;if(window.__failSketch){window.__failSketch=false;const callback=this.onmessage;this.onmessage=event=>callback?.call(this,{data:{...event.data,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Injected transport failure'}}})}}return super.postMessage(message,...args)}}
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
 const original={version:1,sketches:[{id:'square',name:'Square',closed:true,points:[[10,0],[20,0],[20,10],[10,10]]}],bodies:[]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}else{await search.fill(name);await search.press('Enter')}}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 async function invalidateQuantity(field,value,preview){
  await apply.click({trial:true})
  assert.ok(await solid.locator(preview).count()>0)
  const requests=await page.evaluate(()=>window.__sketchEditRequests)
  await input(field,'-')
  assert.equal(await apply.isDisabled(),true)
  await page.waitForFunction(selector=>!document.querySelector(selector),preview)
  assert.equal(await page.evaluate(()=>window.__sketchEditRequests),requests)
  if(process.argv.includes('--fail-preview'))await page.evaluate(()=>window.__failSketch=true)
  await input(field,value)
  if(process.argv.includes('--fail-preview')){
   await solid.getByRole('alert').filter({hasText:'Не удалось получить корректный результат вычисления'}).waitFor()
   assert.equal(await apply.isDisabled(),true)
   const failed=await download('Скачать проект JSON',`failed-${requests}.json`)
   assert.deepEqual(failed.sketches[0].points,original.sketches[0].points);assert.equal(failed.sketches.length,1)
   if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
   await page.screenshot({path:path.join(directory,`retry-${requests}.png`)})
   await activate(solid.getByRole('button',{name:'Повторить вычисление',exact:true}))
  }
  await apply.click({trial:true})
  assert.equal(await page.evaluate(()=>window.__sketchEditRequests),requests+(process.argv.includes('--fail-preview')?2:1))
  assert.ok(await solid.locator(preview).count()>0)
 }

 await activate(solid.getByRole('button',{name:'Square',exact:true}))
 await command('Fillet')
 const radius=solid.getByLabel(/^Радиус, мм/)
 await input(radius,'20')
 await solid.getByRole('alert').filter({hasText:'Радиус превышает'}).waitFor()
 assert.equal(await apply.isDisabled(),true)
 await input(radius,'2');await apply.click({trial:true})
 await invalidateQuantity(radius,'2','path[fill="#77eac5"][stroke-width="3"]')
 await page.screenshot({path:path.join(directory,'corner-preview.png')})
 const beforeApply=await page.evaluate(()=>window.__sketchEditRequests)
 await activate(apply);assert.equal(await page.evaluate(()=>window.__sketchEditRequests),beforeApply)
 const rounded=await download('Скачать проект JSON','rounded.json')
 assert.equal(rounded.sketches[0].id,'square');assert.ok(rounded.sketches[0].points.length>4)
 assert.ok(Math.abs(rounded.sketches[0].points[0][1]-2)<1e-9)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 await command('DogEar');await apply.click({trial:true});await invalidateQuantity(radius,'2','path[fill="#77eac5"][stroke-width="3"]');await activate(apply)
 const dogear=await download('Скачать проект JSON','dogear.json');assert.ok(dogear.sketches[0].points.length>rounded.sketches[0].points.length)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 await command('Circular copies')
 const count=solid.locator('input[type="number"][min="2"][max="64"]')
 await input(count,'65');await solid.getByRole('alert').filter({hasText:'Задайте 2–64'}).waitFor();assert.equal(await apply.isDisabled(),true)
 await input(count,'4');await apply.click({trial:true})
 await invalidateQuantity(solid.getByLabel(/^Угол/),'360','path[stroke="#b894ff"]')
 await command('Fit')
 const sketchPane=solid.getByRole('region',{name:'2D — эскизы',exact:true})
 const canvasBounds=await sketchPane.locator('.canvas-viewport').boundingBox(),paneBounds=await sketchPane.boundingBox()
 assert.ok(canvasBounds.width>paneBounds.width*.9);assert.ok(canvasBounds.height>=120)
 await page.screenshot({path:path.join(directory,'array-preview.png')})
 const requests=await page.evaluate(()=>window.__sketchEditRequests)
 await activate(apply);assert.equal(await page.evaluate(()=>window.__sketchEditRequests),requests)
 const changed=await download('Скачать проект JSON','copies.json');assert.equal(changed.sketches.length,4)
 assert.equal(new Set(changed.sketches.map(s=>s.id)).size,4)
 for(let i=0;i<4;i++)for(let j=0;j<4;j++){
  const [x,y]=original.sketches[0].points[j],a=i*Math.PI/2,[u,v]=changed.sketches[i].points[j]
  assert.ok(Math.abs(u-(x*Math.cos(a)-y*Math.sin(a)))<1e-8)
  assert.ok(Math.abs(v-(x*Math.sin(a)+y*Math.cos(a)))<1e-8)
 }
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone.sketches[0].points,original.sketches[0].points);assert.equal(undone.sketches.length,1)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','redone.json');assert.deepEqual(redone.sketches,changed.sketches)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,retryFailure:process.argv.includes('--fail-preview'),applyWithoutNewWorkerRequest:true,invalidQuantities:['fillet','dogear','array'],undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'sketch-edit-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
