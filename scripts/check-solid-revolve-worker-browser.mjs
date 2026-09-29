import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
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
  window.__revolveRequests=0;const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {postMessage(message,...args){if(message?.job?.kind==='revolve')window.__revolveRequests++;return super.postMessage(message,...args)}}
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
 const original={version:1,sketches:[{id:'profile',name:'Revolve profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'offset-source.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 const name=original.sketches[0].name;await solid.getByRole('button',{name,exact:true}).waitFor()
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name,exact:true}))
 await activate(solid.getByRole('region',{name:'2D — эскизы',exact:true}).getByRole('button',{name:'Вписать',exact:true}))
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}else{await search.fill(name);await search.press('Enter')}}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command('Revolve')
 const geometry=solid.locator('label').filter({hasText:/^Поверхности вращения/}).locator('select')
 if(keyboard){await tabTo(geometry);await page.keyboard.press('End');await page.keyboard.press('Enter');await page.keyboard.press('Tab');assert.equal(await geometry.inputValue(),'exact')}
 else await geometry.selectOption('exact')
 await input(solid.getByLabel(/^Смещение оси, мм/),'-5 mm')
 await input(solid.getByLabel(/^Угол/),'180')
 await input(solid.getByLabel(/^Угол/),'360')
 await apply.click({trial:true})
 const readyRequests=await page.evaluate(()=>window.__revolveRequests)
 assert.ok(await solid.locator('polygon[fill="#75e4b8"]').count()>0)
 await input(solid.getByLabel(/^Угол/),'-')
 assert.equal(await apply.isDisabled(),true)
 await page.waitForFunction(()=>!document.querySelector('polygon[fill="#75e4b8"]'))
 assert.equal(await page.evaluate(()=>window.__revolveRequests),readyRequests)
 await input(solid.getByLabel(/^Угол/),'360')
 await apply.click({trial:true})
 const requests=await page.evaluate(()=>window.__revolveRequests);assert.equal(requests,readyRequests+1)
 await page.screenshot({path:path.join(directory,'revolve-preview.png')})
 await activate(apply)
 assert.equal(await page.evaluate(()=>window.__revolveRequests),requests)
 const changed=await download('Скачать проект JSON','revolved.json');assert.equal(changed.bodies.length,1);assert.ok(changed.bodies[0].brep)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 const step=await download('STEP выбранного тела · текущая геометрия','browser-revolve.step',false)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Worker revolve',file:'browser-revolve.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:2000*Math.PI,boundsMm:[[-20,0,-15],[10,10,15]]}}]}))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone.bodies,before.bodies)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','redone.json');assert.deepEqual(redone.bodies,changed.bodies)
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),workerRequests:requests,applyWithoutRecompute:true,invalidQuantityClearsPreview:true,undoRedo:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'revolve-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
