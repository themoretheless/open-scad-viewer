import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {createHash} from 'node:crypto'
import {readdir} from 'node:fs/promises'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-arc-coordinates')
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

 const sweep=process.argv.includes('--clockwise')?-120:120
 const before=await exportDoc('before.json');await command('Arc')
 async function input(name,value){const field=solid.getByRole('textbox',{name,exact:true});if(keyboard){await focusByTab(field);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await field.fill(value)}
 const create=solid.getByRole('button',{name:'Создать дугу',exact:true}),preview=solid.locator('[data-preview="numeric-arc"]')
 for(const value of ['bad','0','-1','1000001']){
  await input('Радиус дуги',value);assert.equal(await create.isDisabled(),true);assert.equal(await preview.count(),0)
 }
 await input('Координата точки X','2 cm');await input('Координата точки Y','-5 mm');await input('Радиус дуги','6 mm')
 for(const invalid of ['bad','0','0.01','-0.05','360','-360']){await input('Разворот дуги',invalid);assert.equal(await create.isDisabled(),true);assert.equal(await preview.count(),0)}
 await input('Разворот дуги',String(sweep)+' deg');await input('Начальный угол дуги','bad');assert.equal(await create.isDisabled(),true);assert.equal(await preview.count(),0)
 const invalidStart=solid.getByRole('textbox',{name:'Начальный угол дуги',exact:true});assert.equal(await invalidStart.getAttribute('aria-invalid'),'true');assert.ok(await invalidStart.getAttribute('aria-errormessage'));await page.screenshot({path:path.join(directory,'invalid-start.png')})
 await input('Начальный угол дуги','30 deg');await preview.waitFor();assert.equal(await create.isDisabled(),false)
 await input('Координата точки X','bad');assert.equal(await create.isDisabled(),true);assert.equal(await preview.count(),0);await input('Координата точки X','2 cm');await preview.waitFor()
 assert.deepEqual(await exportDoc('draft.json'),before);await page.screenshot({path:path.join(directory,'draft.png')})
 await activate(create);const after=await exportDoc('created.json'),circle=after.sketches.at(-1);assert.equal(after.sketches.length,before.sketches.length+1);assert.deepEqual(circle.analytic,{kind:'arc',center:[20,-5],radius:6,start:30,sweep});if(facePlane){
  plane=circle.plane;assert.equal(circle.supportBodyId,'base');assert.ok(Math.abs(plane.origin[1]+10)<1e-9)
  for(const axis of [plane.u,plane.v]){assert.ok(Math.abs(axis[1])<1e-9);assert.ok(Math.abs(Math.hypot(...axis)-1)<1e-9)}
  assert.ok(Math.abs(plane.u.reduce((sum,x,i)=>sum+x*plane.v[i],0))<1e-9)
  for(const p of circle.points){const world=plane.origin.map((x,i)=>x+plane.u[i]*p[0]+plane.v[i]*p[1]);assert.ok(Math.abs(world[1]+10)<1e-9)}
 }else assert.deepEqual(circle.plane,plane);assert.equal(circle.closed,false)
 const expectedPoint=angle=>[20+6*Math.cos(angle*Math.PI/180),-5+6*Math.sin(angle*Math.PI/180)]
 for(const [actual,expected] of [[circle.points[0],expectedPoint(30)],[circle.points.at(-1),expectedPoint(30+sweep)]])for(let i=0;i<2;i++)assert.ok(Math.abs(actual[i]-expected[i])<1e-9)
 assert.ok(circle.points.length>2)
 assert.equal(await preview.count(),0)
 await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await exportDoc('undone.json'),before);await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await exportDoc('redone.json'),after)
 await command('Arc');await input('Радиус дуги','9');await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('cancelled.json'),after);assert.equal(await preview.count(),0)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),after);assert.deepEqual(errors,[])
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await page.screenshot({path:path.join(directory,'arc.png')});const report={artifacts,browser:browser.version(),keyboard,tabs,gpuActive,rotated,facePlane,plane,sweep,exactCoordinates:true,invalidInput:true,invalidRadiusValues:['bad',0,-1,1000001],invalidSweepValues:['bad',0,.01,-.05,360,-360],endpointToleranceMm:1e-9,cancelledDraftUnchanged:true,undoRedo:true,reloadExact:true};await writeFile(path.join(directory,'arc-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)

}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
