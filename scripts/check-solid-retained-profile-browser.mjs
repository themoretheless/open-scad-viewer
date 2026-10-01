import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-seam-preparation')
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
 if(process.argv.includes('--profile-provenance'))await page.addInitScript(()=>{
  const OriginalWorker=window.Worker;window.__profileProvenance=null
  window.Worker=class extends OriginalWorker{
   constructor(...args){super(...args);this.addEventListener('message',event=>{if(event.data?.kind==='profilePrepare'&&event.data.ok&&event.data.result.report.accepted)window.__profileProvenance=event.data.result.report})}
  }
 })
 await page.addInitScript(()=>{
  if(!navigator.gpu)return
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const make=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const device=await make(...args);window.__qualificationGpuDevice=device;return device}}return adapter}
 })
 if(process.argv.includes('--hold-profile-display'))await page.addInitScript(()=>{
  const OriginalWorker=window.Worker
  window.__profileDisplayHeld=false;window.__profileDisplayTerminated=false
  window.Worker=class extends OriginalWorker {
   postMessage(message,...rest){
    if(message?.job?.kind==='profileDisplay'&&!window.__profileDisplayHeld){window.__profileDisplayHeld=true;this.heldProfile=true;return}
    return super.postMessage(message,...rest)
   }
   terminate(){if(this.heldProfile)window.__profileDisplayTerminated=true;return super.terminate()}
  }
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
  await locator.waitFor()
  await page.waitForFunction(el=>!(el instanceof HTMLButtonElement)||!el.disabled,await locator.elementHandle())
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
 const arcs=process.argv.includes('--arcs'),generalNurbs=process.argv.includes('--general-nurbs'),preparation=arcs||generalNurbs
 const regionOperation=process.argv.find(a=>a.startsWith('--profile-operation='))?.split('=')[1]??'union'
 assert.ok(['union','difference','intersection'].includes(regionOperation));assert.ok(!preparation||regionOperation==='union')
 const operation=preparation?'Prepare profile':regionOperation==='difference'?'Subtract profile regions':regionOperation==='intersection'?'Intersect exact profiles':'Union exact profiles'
 const original={version:1,sketches:[{id:'rect',name:'Rectangle',closed:true,points:[[-4,-3],[3,-3],[3,3],[-4,3]]},{id:'circle',name:'Circle',closed:true,points:[],analytic:{kind:'circle',center:[3,0],radius:2,start:0,sweep:360}}],bodies:[]}
 if(regionOperation!=='union')original.sketches=[{id:'rect',name:'Plate',closed:true,points:[[-4,-3],[4,-3],[4,3],[-4,3]]},{id:'circle',name:'Circular cutter',closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius:2,start:0,sweep:360}}]
 if(arcs)original.sketches=[{id:'rect',name:'Diameter',closed:false,points:[[-2,0],[2,0]]},{id:'circle',name:'Semicircle',closed:false,points:[],analytic:{kind:'arc',center:[0,0],radius:2,start:0,sweep:180}}]
 if(generalNurbs){original.sketches=[{id:'line',name:'Profile lines',closed:false,points:[[2,0],[2,2],[0,2],[0,0]]}];original.curves=[{id:'nurbs',name:'General NURBS',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,-1],[2,0]],weights:[1,1,1]}}]}
 const inputs=generalNurbs?[original.curves[0],original.sketches[0]]:original.sketches
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'profiles.json' ,mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 await solid.getByRole('button',{name:inputs[0].name,exact:true}).waitFor()
 const before=await download('Скачать проект JSON','before.json')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:inputs[0].name,exact:true}))
 const second=solid.getByRole('button',{name:inputs[1].name,exact:true})
 if(keyboard){await tabTo(second);await page.keyboard.press('Shift+Enter')}else await second.click({modifiers:['Shift']})
 await activate(solid.getByRole('region',{name:'2D — эскизы',exact:true}).getByRole('button',{name:'Вписать',exact:true}))
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command(operation)
 if(regionOperation==='difference'){
  const target=solid.getByLabel('Основной профиль',{exact:true})
  await target.selectOption('circle');assert.equal(await apply.isDisabled(),true)
  await solid.getByText('Общей области нет или основной профиль полностью вырезан. Измените входы или основной профиль.',{exact:true}).first().waitFor()
  await page.screenshot({path:path.join(directory,'empty-result.png')})
  await target.selectOption('rect');assert.match(await solid.getByTestId('profile-operands').textContent(),/Circular cutter/)
 }
 await apply.click({trial:true});assert.equal(await solid.locator(preparation?'[data-preview="prepared-profile"]':'[data-preview="retained-profile"]').count(),1)
 if(process.argv.includes('--profile-provenance')){
  assert.ok(preparation,'Provenance fixture requires preparation')
  const report=await page.evaluate(()=>window.__profileProvenance)
  assert.ok(report?.accepted);assert.equal(report.curveSources.length,generalNurbs?4:3)
  assert.equal(report.curveSources.filter(s=>s.chain===0).length,1)
  assert.equal(report.curveSources.filter(s=>s.chain===1).length,generalNurbs?3:2)
  assert.ok(report.curveSources.every(s=>s.connector===false&&typeof s.reversed==='boolean'))
  await writeFile(path.join(directory,'profile-provenance.json'),JSON.stringify(report,null,2)+'\n')
 }
 if(process.argv.includes('--hold-profile-display')){
  const status=solid.getByRole('status',{name:'profile-display',exact:true})
  await status.waitFor();await page.waitForFunction(()=>window.__profileDisplayHeld)
  assert.equal(await solid.locator('[data-preview="retained-profile"]').getAttribute('d'),'')
  await status.getByRole('button',{name:'Esc',exact:true}).click()
  assert.equal(await page.evaluate(()=>window.__profileDisplayTerminated),true)
  await solid.getByRole('button',{name:'Обновить профили',exact:true}).click()
 }
 await solid.getByRole('status',{name:'profile-display',exact:true}).waitFor({state:'hidden'})
 if(!preparation)assert.ok((await solid.locator('[data-preview="retained-profile"]').getAttribute('d'))?.includes(' L '),'Retained preview must contain sampled geometry')
 const heading=solid.locator('.operation-card>strong'),headingBox=await heading.boundingBox(),cardBox=await solid.locator('.operation-card').boundingBox()
 assert.ok(headingBox&&cardBox&&headingBox.y>=cardBox.y&&headingBox.y+headingBox.height<=cardBox.y+cardBox.height,'Command heading must remain visible')
 if(generalNurbs){const helpBox=await solid.locator('.operation-card>small').first().boundingBox();await writeFile(path.join(directory,'command-layout.json'),JSON.stringify({headingBox,helpBox,cardBox,scrollTop:await solid.locator('.operation-card').evaluate(el=>el.scrollTop)}));assert.ok(helpBox&&helpBox.y>=headingBox.y+headingBox.height,'Command heading must not cover help text')}
 await page.screenshot({path:path.join(directory,'retained-preview.png')})
 await activate(solid.getByRole('button',{name:'Esc',exact:true}))
 const canceled=await download('Скачать проект JSON','canceled.json');assert.deepEqual(canceled.sketches,before.sketches);if(generalNurbs)assert.deepEqual(canceled.curves,before.curves)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command(operation);await activate(apply)
 const joined=await download('Скачать проект JSON','joined.json');assert.equal(joined.sketches.length,1);if(generalNurbs){assert.equal(joined.sketches[0].id,'nurbs');assert.deepEqual(joined.curves,[]);assert.ok(joined.sketches[0].retainedProfile.areaIntervalMm2)}
 assert.ok(joined.sketches[0].retainedProfile.loops.flat().some(c=>c.degree===2))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 await activate(solid.getByRole('button',{name:'Повтор · Shift R',exact:true}));await activate(apply)
 const repeated=await download('Скачать проект JSON','repeated.json');assert.deepEqual(repeated.sketches,joined.sketches)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Extrude')
 await solid.getByLabel('Высота, мм',{exact:true}).fill('5 mm');await activate(apply)
 const extruded=await download('Скачать проект JSON','extruded.json');assert.equal(extruded.bodies.length,1)
 assert.ok(extruded.bodies[0].brep.faces.some(f=>f.surface.degreeU===2||f.surface.degreeV===2))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('region',{name:'3D — тела',exact:true}).getByRole('button',{name:'Вписать',exact:true}))
 const gpuDeviceDestroyed=await page.evaluate(()=>{const device=window.__qualificationGpuDevice;if(!device)return false;device.destroy();return true})
 await solid.locator('[data-body]').first().waitFor({state:'visible'})
 const renderedTriangles=await solid.locator('[data-body]').count();assert.ok(renderedTriangles>0);assert.deepEqual(renderErrors,[])
 await writeFile(path.join(directory,'render-state.json'),JSON.stringify({errors:renderErrors,canvases:await solid.locator('canvas').evaluateAll(nodes=>nodes.map(n=>({width:n.width,height:n.height,rect:n.getBoundingClientRect().toJSON(),display:getComputedStyle(n).display}))),svgPolygons:await solid.locator('[data-body]').count()},null,2))
 if(await solid.locator('canvas.gpu-layer').isVisible())await solid.locator('canvas.gpu-layer').screenshot({path:path.join(directory,'gpu-only.png')})
 await page.screenshot({path:path.join(directory,'retained-extruded.png')})
 const step=await download('STEP выбранного тела · текущая геометрия','browser-retained.step',false)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Browser retained union',file:'browser-retained.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:(generalNurbs?14/3:arcs?2*Math.PI:regionOperation==='difference'?48-4*Math.PI:regionOperation==='intersection'?4*Math.PI:42+2*Math.PI)*5,boundsMm:generalNurbs?[[0,-.5,0],[2,2,5]]:arcs?[[-2,0,0],[2,2,5]]:regionOperation==='difference'?[[-4,-3,0],[4,3,5]]:regionOperation==='intersection'?[[-2,-2,0],[2,2,5]]:[[-4,-3,0],[5,3,5]]}}]}))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 await solid.getByRole('button',{name:extruded.bodies[0].name,exact:true}).waitFor({state:'hidden'})
 const undoBody=await download('Скачать проект JSON','undo-body.json');assert.deepEqual(undoBody.sketches,joined.sketches);assert.equal(undoBody.bodies.length,0)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 if(generalNurbs)await solid.getByRole('button',{name:'Profile lines',exact:true}).waitFor()
 const undone=await download('Скачать проект JSON','undone.json');assert.deepEqual(undone.sketches,before.sketches);if(generalNurbs)assert.deepEqual(undone.curves,before.curves)
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'joined.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(joined))})
 if(generalNurbs)await solid.getByRole('button',{name:'Profile lines',exact:true}).waitFor({state:'hidden'})
 const reloaded=await download('Скачать проект JSON','reloaded.json');assert.deepEqual(reloaded.sketches,joined.sketches)
 const report={browser:browser.version(),profileWorkerCancelled:await page.evaluate(()=>window.__profileDisplayTerminated===true),operation,retainedArcs:!generalNurbs,retainedGeneralNurbs:generalNurbs,exactExtrusion:true,gpuFallback:gpuDeviceDestroyed,renderedTriangles,cancel:true,undo:true,repeat:true,jsonReload:true,keyboard,tabPresses,downloads}
 await writeFile(path.join(directory,'retained-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
