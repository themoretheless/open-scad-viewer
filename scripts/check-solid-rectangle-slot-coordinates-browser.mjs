import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {createHash} from 'node:crypto'
import {readdir} from 'node:fs/promises'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-rectangle-slot-coordinates')
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
const errors=[]
const chunks=(await readdir(path.join(root,'assets'))).filter(name=>/^DirectModeler-[^/]+\.js$/.test(name));assert.equal(chunks.length,1)
const sha256=async file=>createHash('sha256').update(await readFile(file)).digest('hex')
const artifacts={directModeler:{file:chunks[0],sha256:await sha256(path.join(root,'assets',chunks[0]))},wasm:{sha256:await sha256(path.join(root,'wasm/geometry-kernel.wasm'))},sourceSha256:await sha256('src/features/DirectModeler.vue')}

try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))});page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})

 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__slotRequests=0;window.__slotTerminated=0;window.__slotJobs=[]
  window.Worker=class extends NativeWorker{
   constructor(...args){super(...args);this.addEventListener('message',event=>{if(this.__slot&&event.data?.ok===true)window.__slotLastReply=structuredClone(event.data)})}
   postMessage(message,...args){
    if(message?.job?.kind==='profilePrepare'){
     this.__slot=true;window.__slotRequests++;window.__slotJobs.push(structuredClone(message.job))
     if(window.__failSlots){window.__failSlots=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:'profilePrepare',ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'PRIVATE SLOT FAILURE'}}}));return}
     if(window.__holdSlots){this.__held=true;(window.__slotHeld??=[]).push({message,callback:this.onmessage});return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__slotTerminated++;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){
  for(let i=0;i<1500;i++){
   if(await locator.evaluate(e=>e===document.activeElement))return
   await page.keyboard.press('Tab');tabs++
  }
  throw new Error('Keyboard target was not reachable by Tab')
 }
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function choose(locator,value){
  if(!keyboard){await locator.selectOption(value);return}
  const label=await locator.locator('option').evaluateAll((nodes,value)=>nodes.find(n=>n.value===value)?.textContent,value)
  assert.ok(label);await focusByTab(locator)
  await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');await focusByTab(locator)
  // Headless macOS native select popups ignore arrow navigation. Real char
  // events retain native type-ahead, including non-ASCII option labels.
  const cdp=await page.context().newCDPSession(page)
  try {for(const letter of label)await cdp.send('Input.dispatchKeyEvent',{type:'char',text:letter,key:letter})}finally{await cdp.detach()}
  assert.equal(await locator.inputValue(),value)

 }
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu);const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function command(name){await closeMenu();await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await search.press('ControlOrMeta+A');await search.pressSequentially(name)}else await search.fill(name);await search.press('Enter')}

 const facePlane=process.argv.includes('--face-plane'),rotated=process.argv.includes('--rotated-plane');let plane=rotated?{origin:[5,10,15],u:[0,1,0],v:[0,0,1]}:{origin:[0,0,0],u:[1,0,0],v:[0,1,0]}
 const seed={version:1,sketches:rotated?[{id:'plane-seed',name:'Rotated plane',closed:false,points:[[0,0],[1,0]],plane}]:[],bodies:[]}
 if(facePlane){const fixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-scene-edit/mouse/before.json','utf8'));seed.bodies=[{...fixture.bodies[0],id:'base',name:'Base'}]}
 await ready();await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'fixture.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(seed))});await closeMenu();await ready()
 if(rotated)await activate(solid.getByRole('button',{name:'Rotated plane',exact:true}))
 if(facePlane){
  await activate(solid.getByRole('button',{name:'Base',exact:true}));await solid.getByRole('status',{name:'topology-preparation',exact:true}).waitFor({state:'hidden'})
  await activate(solid.getByRole('button',{name:'Грани',exact:true}));const faces=solid.getByRole('combobox',{name:'Выбрать грань',exact:true})
  if(keyboard){await focusByTab(faces);await page.keyboard.press('Home');for(let i=0;i<3;i++)await page.keyboard.press('ArrowDown');assert.equal(await faces.inputValue(),'2')}else await faces.selectOption('2')
  assert.match(await faces.locator('option:checked').innerText(),/0, -10, 10 mm/)
  await command('Sketch on face');await solid.getByRole('status',{name:'face-sketch-preparation',exact:true}).waitFor({state:'hidden'});await solid.getByText('Рисуйте на выделенной грани в 3D или в панели эскиза. Затем нажмите «Выдавить».',{exact:true}).waitFor()
 }


 const gestureSlots=process.argv.includes('--gesture-slot');assert.ok(!gestureSlots||!keyboard&&!facePlane)
 const before=await exportDoc('before.json'),diagonal=process.argv.includes('--diagonal'),length=diagonal?5:10
 async function input(name,value){const field=solid.getByRole('textbox',{name,exact:true});if(keyboard){await focusByTab(field);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await field.fill(value)}
 async function slotReady(){await solid.getByRole('status',{name:'numeric-slot-preparation',exact:true}).waitFor({state:'hidden'});await solid.locator('[data-preview="numeric-slot"]').waitFor();assert.equal(await solid.locator('[data-preview="numeric-slot"]').getAttribute('stroke'),'#77eac5')}
 const rectangle=solid.getByRole('button',{name:'Создать прямоугольник',exact:true}),slot=solid.getByRole('button',{name:'Создать паз',exact:true})
 await command('Rectangle')
 for(const value of ['bad','0','-1','1000001']){await input('Размер прямоугольника Ширина',value);assert.equal(await rectangle.isDisabled(),true);assert.equal(await solid.locator('[data-preview="numeric-rectangle"]').count(),0)}
 await input('Размер прямоугольника Ширина','10 mm');await input('Начальная координата X','1000000');assert.equal(await rectangle.isDisabled(),true)
 await input('Начальная координата X','2 cm');await input('Начальная координата Y','-5 mm');await input('Размер прямоугольника Высота','6 mm');await solid.locator('[data-preview="numeric-rectangle"]').waitFor()
 assert.equal(await solid.locator('[data-preview="numeric-rectangle"]').getAttribute('stroke'),'#77eac5');assert.deepEqual(await exportDoc('rectangle-draft.json'),before);await page.screenshot({path:path.join(directory,'rectangle-draft.png')});await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('rectangle-cancelled.json'),before)
 await page.evaluate(()=>window.__failSlots=true);await command('Slot');await solid.getByRole('alert').filter({hasText:'Не удалось подготовить паз.'}).waitFor();assert.equal(await solid.getByRole('alert').filter({hasText:'Не удалось подготовить паз.'}).evaluate(el=>{const r=el.getBoundingClientRect(),p=el.closest('.operation-card').getBoundingClientRect();return r.top>=p.top&&r.bottom<=p.bottom&&r.left>=p.left&&r.right<=p.right}),true);assert.equal(await slot.isDisabled(),true);assert.equal(await solid.innerText().then(t=>t.includes('PRIVATE SLOT FAILURE')),false);await page.screenshot({path:path.join(directory,'slot-failure.png')})
 await activate(solid.getByRole('button',{name:'Повторить расчёт паза',exact:true}));await slotReady()
 for(const value of ['bad','0','-1','0.01','1000001']){const requests=await page.evaluate(()=>window.__slotRequests);await input('Ширина паза, мм',value);assert.equal(await slot.isDisabled(),true);assert.equal(await solid.locator('[data-preview="numeric-slot"]').count(),0);assert.equal(await page.evaluate(()=>window.__slotRequests),requests)}
 await input('Ширина паза, мм','4 mm');await input('Конечная координата X','0');assert.equal(await slot.isDisabled(),true);await input('Конечная координата X','10');await slotReady()
 await page.evaluate(()=>window.__holdSlots=true);await input('Конечная координата X','11');await page.waitForFunction(()=>window.__slotHeld?.length===1);await input('Конечная координата X','12');await page.waitForFunction(()=>window.__slotHeld?.length===2)
 await page.evaluate(()=>{const held=window.__slotHeld[0];held.callback({data:{...window.__slotLastReply,id:held.message.id}})});assert.equal(await slot.isDisabled(),true);assert.equal(await solid.locator('[data-preview="numeric-slot"]').count(),0)
 await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>window.__slotTerminated),2);await page.evaluate(()=>{window.__holdSlots=false;const held=window.__slotHeld[1];held.callback({data:{...window.__slotLastReply,id:held.message.id}})})
 assert.deepEqual(await exportDoc('slot-late-cancelled.json'),before);assert.equal(await solid.locator('[data-preview="numeric-slot"]').count(),0)
 const gestureSpecs=[]
 for(let i=0;i<10;i++){
  await command('Rectangle');await input('Начальная координата X',String(20+2*i)+' mm');await input('Начальная координата Y','-5 mm');await input('Размер прямоугольника Ширина','10 mm');await input('Размер прямоугольника Высота','6 mm');await activate(rectangle)
  await command('Slot');await input('Начальная координата X',String(2*i)+' mm');await input('Начальная координата Y','0 mm');await input('Конечная координата X',String(2*i+(diagonal?3:10))+' mm');await input('Конечная координата Y',diagonal?'4 mm':'0 mm');await input('Ширина паза, мм','0.4 cm');await slotReady()
  const requests=await page.evaluate(()=>window.__slotRequests)
  if(gestureSlots){
   const svg=solid.locator('svg[aria-label="Холст эскизов 2D"]'),[a,b]=await svg.evaluate((el,points)=>points.map(p=>{const q=new DOMPoint(p[0],-p[1]).matrixTransform(el.getScreenCTM());return {x:q.x,y:q.y}}),[[2*i,0],[2*i+(diagonal?3:10),diagonal?4:0]])
   await page.keyboard.down('Alt');await page.mouse.move(a.x,a.y);await page.mouse.down();await page.mouse.move(b.x,b.y,{steps:5});await page.mouse.up();await page.keyboard.up('Alt');await slot.waitFor({state:'hidden'});assert.ok(await page.evaluate(()=>window.__slotRequests)>requests)
   const job=await page.evaluate(()=>window.__slotJobs.at(-1));gestureSpecs.push({a:job.document.sketches.find(s=>s.id==='slot-cap-1').analytic.center,b:job.document.sketches.find(s=>s.id==='slot-cap-0').analytic.center})
  }else{await activate(slot);assert.equal(await page.evaluate(()=>window.__slotRequests),requests)}
 }
 const after=await exportDoc('created-20.json');assert.equal(after.sketches.length,before.sketches.length+20);assert.equal(new Set(after.sketches.map(s=>s.id)).size,after.sketches.length);assert.deepEqual(after.bodies,before.bodies)
 for(let i=0;i<10;i++){
  const rect=after.sketches[before.sketches.length+2*i],capsule=after.sketches[before.sketches.length+2*i+1]
  assert.deepEqual(rect.points,[[20+2*i,-5],[30+2*i,-5],[30+2*i,1],[20+2*i,1]]);assert.equal(rect.closed,true)
  assert.equal(capsule.closed,true);assert.equal(capsule.retainedProfile.loops[0].filter(c=>c.degree===1).length,2);assert.equal(capsule.retainedProfile.loops[0].filter(c=>c.degree===2).length,4);assert.ok(Math.abs(capsule.retainedProfile.areaMm2-((gestureSlots?Math.hypot(gestureSpecs[i].b[0]-gestureSpecs[i].a[0],gestureSpecs[i].b[1]-gestureSpecs[i].a[1]):length)*4+4*Math.PI))<1e-9)
  if(facePlane){assert.equal(rect.supportBodyId,'base');assert.equal(capsule.supportBodyId,'base');plane=rect.plane;assert.deepEqual(capsule.plane,plane);for(const item of [rect,capsule])for(const p of item.points)assert.ok(Math.abs(item.plane.origin[1]+item.plane.u[1]*p[0]+item.plane.v[1]*p[1]+10)<1e-9)}else{assert.deepEqual(rect.plane,plane);assert.deepEqual(capsule.plane,plane)}
 }
 for(let i=0;i<20;i++)await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await exportDoc('undone-20.json'),before)
 for(let i=0;i<20;i++)await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await exportDoc('redone-20.json'),after)
 const pointerNominalDeviationMm=gestureSlots?Math.max(...gestureSpecs.flatMap((spec,i)=>[Math.hypot(spec.a[0]-2*i,spec.a[1]),Math.hypot(spec.b[0]-2*i-(diagonal?3:10),spec.b[1]-(diagonal?4:0))])):0
 assert.ok(pointerNominalDeviationMm<1e-4)
 // Extrude the last rectangle and retained slot and export their CURRENT B-rep.
 const parts=[]
 async function download(name,file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu);const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name,exact:true}));await (await pending).saveAs(path.join(directory,file));await closeMenu();return readFile(path.join(directory,file))}
 function bounds(local){const n=[plane.u[1]*plane.v[2]-plane.u[2]*plane.v[1],plane.u[2]*plane.v[0]-plane.u[0]*plane.v[2],plane.u[0]*plane.v[1]-plane.u[1]*plane.v[0]],points=[];for(const x of [local[0][0],local[1][0]])for(const y of [local[0][1],local[1][1]])for(const z of [0,7])points.push(plane.origin.map((p,k)=>p+plane.u[k]*x+plane.v[k]*y+n[k]*z));return [points[0].map((_,k)=>Math.min(...points.map(p=>p[k]))),points[0].map((_,k)=>Math.max(...points.map(p=>p[k])))]}
 const lastGesture=gestureSpecs.at(-1),slotBounds=gestureSlots?[[Math.min(lastGesture.a[0],lastGesture.b[0])-2,Math.min(lastGesture.a[1],lastGesture.b[1])-2],[Math.max(lastGesture.a[0],lastGesture.b[0])+2,Math.max(lastGesture.a[1],lastGesture.b[1])+2]]:[[16,-2],[diagonal?23:30,diagonal?6:2]],slotArea=(gestureSlots?Math.hypot(lastGesture.b[0]-lastGesture.a[0],lastGesture.b[1]-lastGesture.a[1]):length)*4+4*Math.PI
 for(const [kind,sketch,local,area] of [['rectangle',after.sketches.at(-2),[[38,-5],[48,1]],60],['slot',after.sketches.at(-1),slotBounds,slotArea]]){
  await activate(solid.getByRole('button',{name:sketch.name,exact:true}));await command('Extrude');await activate(solid.getByRole('button',{name:'Новое',exact:true}));await input('Высота, мм','7 mm');await activate(solid.getByRole('button',{name:'Готово · Enter',exact:true}));await ready()
  const file=kind+'.step',step=await download('STEP выбранного тела · текущая геометрия',file);parts.push({name:'Numeric '+kind,file,sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:area*7,boundsMm:bounds(local)}})
 }
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
 const final=await exportDoc('extruded.json');assert.deepEqual(final.sketches,after.sketches);assert.equal(final.bodies.length,before.bodies.length+2)
 const requests=await page.evaluate(()=>window.__slotRequests);await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),final);assert.deepEqual(errors,[])
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await page.screenshot({path:path.join(directory,'rectangle-slot.png')});const report={artifacts,browser:browser.version(),keyboard,tabs,gpuActive,rotated,facePlane,diagonal,plane,previewMatchesLegend:true,rectangleDimensions:[10,6],slotAreaMm2:slotArea,nominalSlotAreaMm2:length*4+4*Math.PI,slotWidthMm:4,gestureSlots,gestureSpecs,pointerNominalDeviationMm,preparedApplyNoRecompute:!gestureSlots,requests,localizedWorkerFailure:true,retry:true,lateCancelledReplyIgnored:true,supersededReplyIgnored:true,invalidInputNoDispatch:true,twentySequentialCreations:true,twentyUndoRedo:true,reloadExact:true,currentStepExports:2};await writeFile(path.join(directory,'rectangle-slot-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)

}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
