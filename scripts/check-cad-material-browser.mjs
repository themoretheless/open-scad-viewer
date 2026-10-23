import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-face-distance')
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
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))});page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[];window.__distanceTimings=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.__starts=new Map();this.addEventListener('message',e=>{if(['materialChord','materialSegment','materialWall'].includes(e.data?.kind)&&e.data.ok){window.__distanceResults.push(e.data.result);const start=this.__starts.get(e.data.id);if(start!==undefined){window.__distanceTimings.push({workerRoundTripMs:performance.now()-start,cells:e.data.result.cells,domainCells:e.data.result.domainCells,converged:e.data.result.converged});this.__starts.delete(e.data.id)}}})}
   postMessage(message,...args){if(['materialChord','materialSegment','materialWall'].includes(message?.job?.kind)){window.__distanceRequests++;if(window.__failDistance){window.__failDistance=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private distance worker failure'}}}));return}if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}this.__starts.set(message.id,performance.now())}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__distanceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){for(let i=0;i<300;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabs++}throw Error('Unreachable keyboard control')}
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function input(locator,value){if(keyboard){await focusByTab(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
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
 async function command(name){await closeMenu();await activate(page.getByRole('button',{name:'Команды',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,name);await search.press('Enter')}

 const documentFixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/boundary-embedding-wasm/browser-document.json','utf8'))
 const placed=process.argv.includes('--placed')
 if(placed){
  const contract=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/material-wall/contract.json','utf8'))
  const specimen=contract.cases.find(c=>c.name==='placed-annular-wall');assert.ok(specimen)
  documentFixture.bodies[0].brep=specimen.request.model
  const points=documentFixture.bodies[0].mesh.positions
  for(let i=0;i<points.length;i+=3){const [x,y,z]=points.slice(i,i+3);points.splice(i,3,123-y,-45-z,67+x)}
 }

 await ready();await activate(menu)
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'material.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(documentFixture))})
 await closeMenu();await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
 await activate(solid.getByRole('button',{name:'Quotient annular',exact:true}))
 const before=await exportDoc('before.json');await command('Measure vertices / edge')
 const toggle=solid.getByRole('button',{name:'Проверка материала вдоль линии',exact:true})
 await activate(toggle)
 const field=solid.getByRole('group',{name:'Материал вдоль линии',exact:true})
 async function coords(origin,direction){
  if(placed){origin=[123-origin[1],-45-origin[2],67+origin[0]];direction=[-direction[1],-direction[2],direction[0]]}
  for(const [values,name] of [[origin,'origin'],[direction,'direction']])for(let i=0;i<3;i++)await input(field.getByRole('spinbutton',{name:(name==='origin'?'Начало ':'Смещение ')+['X','Y','Z'][i],exact:true}),String(values[i]))}
 async function run(){await activate(field.getByRole('button',{name:/^(Проверить|Повторить проверку)/}))}
 async function proven(){await field.locator('[data-material-proven="true"]').waitFor({timeout:120000})}
 await coords([25,2,3],[-24,0,0]);await run();await proven()
 await field.locator('[data-material-normal]').filter({hasText:'Линия наклонена'}).waitFor()
 const length=await field.locator('[data-material-length]').innerText()
 assert.equal(await solid.locator('[data-material-face]').count(),2)
 await page.screenshot({path:path.join(directory,'material-chord.png'),fullPage:true})
 await coords([15,20,3],[-14.4,-19.2,0]);await run();await proven()
 await field.locator('[data-material-normal]').filter({hasText:'перпендикулярно обеим граням'}).waitFor()
 const radialLength=await field.locator('[data-material-length]').innerText()
 const radialBounds=radialLength.replace(' mm','').split(' … ').map(Number)
 assert.ok(radialBounds[0]<=15&&radialBounds[1]>=15)
 await page.screenshot({path:path.join(directory,'material-radial-normal.png'),fullPage:true})
 if(process.argv.includes('--wall')){
  await choose(field.getByRole('combobox',{name:'Режим проверки материала',exact:true}),'wall')
  const contract=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/material-wall/contract.json','utf8'))
  const wallCase=contract.cases.find(c=>c.name==='annular-wall')
  assert.ok(wallCase)
  for(let group=0;group<2;group++)for(const face of wallCase.request.faceGroups[group]){
   const checkbox=field.getByRole('group',{name:'Группа граней '+(group+1),exact:true}).getByRole('checkbox',{name:'Грань '+(face+1),exact:true})
   if(keyboard){await focusByTab(checkbox);await page.keyboard.press('Space')}else await checkbox.check()
  }
  await coords([25,2,3],[-24,0,0]);await run()
  await field.locator('[data-wall-converged="false"]').waitFor({timeout:120000})
  await field.locator('[data-material-normal]').filter({hasText:'Линия наклонена'}).waitFor()
  assert.equal(await field.locator('[data-wall-thickness]').count(),0)
  assert.equal(await solid.locator('[data-diagnostic="material-path"] line').getAttribute('stroke'),'#ff6978')
  await coords([15,20,3],[-14.4,-19.2,0])
  await run();await field.locator('[data-wall-converged="true"]').waitFor({timeout:120000})
  const bounds=(await field.locator('[data-wall-thickness]').innerText()).replace(' mm','').split(' … ').map(Number)
  assert.ok(bounds[0]<=15&&bounds[1]>=15)
  assert.equal(await solid.locator('[data-material-face]').count(),2)
  await page.screenshot({path:path.join(directory,'wall-thickness.png'),fullPage:true})
  await page.evaluate(()=>{window.__holdDistance=true;window.__distanceHeld=false;window.__distanceTerminated=false});await run();await page.waitForFunction(()=>window.__distanceHeld===true)
  await input(field.getByRole('spinbutton',{name:'Допуск, мм',exact:true}),'0.02')
  await page.waitForFunction(()=>window.__distanceTerminated===true)
  assert.equal(await field.locator('[data-wall-thickness]').count(),0)
  await page.evaluate(()=>{window.__holdDistance=false;window.__failDistance=true});await run()
  await field.getByRole('alert').filter({hasText:'Проверка не выполнена'}).waitFor()
  await run();await field.locator('[data-wall-converged="true"]').waitFor({timeout:120000})
  const auto=field.getByRole('checkbox',{name:'Автоматически искать тонкие участки',exact:true})
  if(keyboard){await focusByTab(auto);await page.keyboard.press('Space')}else await auto.check()
  await page.evaluate(()=>{window.__holdDistance=true;window.__distanceHeld=false;window.__distanceTerminated=false})
  await run();await page.waitForFunction(()=>window.__distanceHeld===true)
  await input(field.getByRole('spinbutton',{name:'Допуск, мм',exact:true}),'0.01')
  await page.waitForFunction(()=>window.__distanceTerminated===true)
  assert.equal(await field.locator('[data-wall-thickness]').count(),0)
  await page.evaluate(()=>{window.__holdDistance=false})
  await run();await field.locator('[data-wall-converged="true"]').waitFor({timeout:120000})
  const autoBounds=(await field.locator('[data-wall-thickness]').innerText()).replace(' mm','').split(' … ').map(Number)
  assert.ok(autoBounds[0]<=15&&autoBounds[1]>=15)
  await page.screenshot({path:path.join(directory,'automatic-wall.png'),fullPage:true})
  await choose(field.getByRole('combobox',{name:'Режим проверки материала',exact:true}),'chord')
 }
 await coords([25,2,3],[-50,0,0]);await run()
 await field.getByRole('status').filter({hasText:'Нужны два пересечения'}).waitFor({timeout:120000})
 assert.equal(await solid.locator('[data-material-face]').count(),4)
 await page.screenshot({path:path.join(directory,'cavity-refusal.png'),fullPage:true})
 await choose(field.getByRole('combobox',{name:'Режим проверки материала',exact:true}),'segment')
 await coords([10,0,3],[5,0,0]);await run();await proven()
 await field.getByRole('status').filter({hasText:'Весь отрезок находится внутри'}).waitFor()
 await coords([10,0,3],[0,0,0]);await run()
 await field.getByRole('alert').filter({hasText:'ненулевое смещение'}).waitFor()
 assert.equal(await solid.locator('[data-diagnostic="material-path"]').count(),0)
 await choose(field.getByRole('combobox',{name:'Режим проверки материала',exact:true}),'chord')
 await coords([25,2,3],[-24,0,0])
 await page.evaluate(()=>{window.__holdDistance=true;window.__distanceHeld=false;window.__distanceTerminated=false});await run()
 await page.waitForFunction(()=>window.__distanceHeld===true)
 await page.keyboard.press('Escape');await field.waitFor({state:'hidden'})
 await page.waitForFunction(()=>window.__distanceTerminated===true)
 await page.evaluate(()=>{window.__holdDistance=false;window.__failDistance=true})
 await activate(toggle);await run()
 await field.getByRole('alert').filter({hasText:'Проверка не выполнена'}).waitFor()
 assert.ok(!(await field.innerText()).includes('Private distance worker failure'))
 await run();await proven()
 const results=await page.evaluate(()=>window.__distanceResults)
 assert.ok(results.some(r=>r.normalAlignment==='angular-tolerance'&&r.normalEvidence.aligned===true))
 assert.ok(results.some(r=>r.normalAlignment==='oblique'&&r.normalEvidence.aligned===false))
 assert.ok(results.some(r=>r.method==='continuous-material-chord'&&r.proven))
 assert.ok(results.some(r=>r.method==='continuous-material-chord'&&!r.proven))
 assert.ok(results.some(r=>r.method==='continuous-material-segment'&&r.proven))
 const after=await exportDoc('after.json');assert.deepEqual(after,before)
 await page.reload();await ready();const restored=await exportDoc('reloaded.json');assert.deepEqual(restored,before)
 assert.deepEqual(errors,[])
 await writeFile(path.join(directory,'report.json'),JSON.stringify({passed:true,placed,wasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex'),wall:process.argv.includes('--wall'),keyboard,tabs,length,radialLength,results,cancelledHeldRequest:true,retry:true,invalidLineLocalized:true,documentUnchanged:true,reloadExact:true,errors},null,2)+'\n')
 console.log(JSON.stringify({passed:true,keyboard,cases:results.length,length}))
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
