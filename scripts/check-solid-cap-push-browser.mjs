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
const errors=[],stepParts=[]
try{
 const {playwright}=await loadQualificationPlaywrightPackage();browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.on('pageerror',e=>errors.push(String(e)))
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker
  window.__holdCap=false;window.__lateCapReplies=[];window.__capTerminations=0;window.__capSuccessReplies=0
  window.Worker=class extends NativeWorker{
   postMessage(message,...args){
    if(window.__holdCap&&message?.job?.kind==='bodyEdit'&&message.job.options?.operation==='push'){
     this.__capHeld=true
     const callback=this.onmessage
     this.onmessage=event=>{if(event.data?.ok===true)window.__capSuccessReplies++;window.__lateCapReplies.push(()=>callback?.call(this,event))}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__capHeld)window.__capTerminations++;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){
  for(let i=0;i<500;i++){
   if(await locator.evaluate(e=>e===document.activeElement))return
   await page.keyboard.press('Tab');tabs++
  }
  throw new Error('Control is not reachable by Tab: '+await locator.getAttribute('aria-label'))
 }
 async function activate(locator){if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(name){await ready();if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu);const event=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await(await event).saveAs(path.join(directory,name));await activate(menu);return JSON.parse(await readFile(path.join(directory,name),'utf8'))}
 const fixtures=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/parts-history/cap-api-fixtures.json','utf8')).cases
 for(const fixture of fixtures){
  await ready();if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'part.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[fixture.request.body]}))})
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu);await ready()
  await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:fixture.name,exact:true}));await ready()
  const before=await exportDoc(fixture.name+'-before.json')
  await activate(solid.getByRole('button',{name:'Грани',exact:true}))
  if(keyboard){
   const selector=solid.getByRole('combobox',{name:'Выбрать грань',exact:true});await focusByTab(selector)
   await page.keyboard.press('Home');for(let i=0;i<=fixture.request.faces[0];i++)await page.keyboard.press('ArrowDown')
   await page.keyboard.press('Enter');await page.keyboard.press('Tab');assert.equal(await selector.inputValue(),String(fixture.request.faces[0]))
  }else{
  const mesh=fixture.request.body.mesh,candidates=[],topZ=Math.max(...fixture.request.body.brep.vertices.map(v=>v.point[2]))
  for(let i=0;i<mesh.indices.length;i+=3){const p=mesh.indices.slice(i,i+3).map(j=>mesh.positions.slice(j*3,j*3+3));if(p.every(v=>Math.abs(v[2]-topZ)<1e-8))candidates.push([0,1,2].map(k=>p.reduce((sum,v)=>sum+v[k],0)/3))}
  const point=await solid.locator('svg[aria-label="Холст тел 3D"]').evaluate((svg,{candidates,id})=>{
   for(const [x,y,z] of candidates){const cy=Math.SQRT1_2,sp=1/Math.sqrt(3),cp=Math.sqrt(2/3);const p=new DOMPoint((x-y)*cy,(x+y)*cy*sp-z*cp).matrixTransform(svg.getScreenCTM());const hit=document.elementFromPoint(p.x,p.y);if(hit?.getAttribute('data-body')===id)return {x:p.x,y:p.y}}
   return null
  },{candidates,id:fixture.request.body.id})
  assert.ok(point,'Visible top cap must be selectable');await page.mouse.click(point.x,point.y)
  }
  await page.evaluate(()=>{window.__holdCap=true;window.__lateCapReplies=[];window.__capTerminations=0;window.__capSuccessReplies=0})
  await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await focusByTab(search);await page.keyboard.insertText('Push / Pull');await page.keyboard.press('Enter')}else{await search.fill('Push / Pull');await search.press('Enter')}
  const amount=solid.getByLabel('Расстояние, мм',{exact:true});if(keyboard){await focusByTab(amount);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText('1 mm')}else await amount.fill('1 mm')
  await page.waitForFunction(()=>window.__lateCapReplies.length>0&&window.__capSuccessReplies>0)
  if(process.argv.includes('--selection-change')){
   await page.evaluate(()=>{window.__oldCapReplies=[...window.__lateCapReplies];window.__capTerminations=0})
   const selector=solid.getByRole('combobox',{name:'Выбрать грань',exact:true})
   await focusByTab(selector);await page.keyboard.press('Home');await page.keyboard.press('ArrowDown')
   assert.notEqual(fixture.request.faces[0],0)
   assert.equal(await selector.inputValue(),'0')
   await page.waitForFunction(()=>window.__capTerminations>0)
   await page.evaluate(()=>{for(const reply of window.__oldCapReplies)reply()})
   assert.deepEqual(await exportDoc(fixture.name+'-selection-changed.json'),before)
   assert.equal(await solid.locator('[data-preview-body]').count(),0)
   await focusByTab(selector);await page.keyboard.press('Home')
   for(let i=0;i<=fixture.request.faces[0];i++)await page.keyboard.press('ArrowDown')
   await page.keyboard.press('Tab')
  }
  if(process.argv.includes('--document-change')){
   await page.evaluate(()=>{window.__oldCapReplies=[...window.__lateCapReplies];window.__capTerminations=0})
   const replacement=structuredClone(fixture.request.body);replacement.id='replacement-'+fixture.name;replacement.name=replacement.id
   async function importBody(body,document={version:1,sketches:[],bodies:[body]}){
    if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu)
    await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'replacement.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(document))})
    if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
    await solid.getByRole('button',{name:body.name,exact:true}).waitFor({state:'visible'});await ready()
   }
   await importBody(replacement)
   await page.waitForFunction(()=>window.__capTerminations>0)
   const replaced=await exportDoc(fixture.name+'-replacement-before.json')
   assert.equal(replaced.bodies[0].id,replacement.id)
   await page.evaluate(()=>{for(const reply of window.__oldCapReplies)reply()})
   assert.deepEqual(await exportDoc(fixture.name+'-replacement-after.json'),replaced)
   assert.equal(await solid.locator('[data-preview-body]').count(),0)
   await importBody(fixture.request.body,before)
   await activate(solid.getByRole('button',{name:fixture.name,exact:true}));await ready()
   await activate(solid.getByRole('button',{name:'Грани',exact:true}))
   const selector=solid.getByRole('combobox',{name:'Выбрать грань',exact:true});await focusByTab(selector)
   await page.keyboard.press('Home');for(let i=0;i<=fixture.request.faces[0];i++)await page.keyboard.press('ArrowDown')
   await page.keyboard.press('Tab')
  }
  await page.keyboard.press('Escape')
  await page.waitForFunction(()=>window.__capTerminations>0)
  await page.evaluate(()=>{window.__holdCap=false;for(const reply of window.__lateCapReplies)reply()})
  assert.deepEqual(await exportDoc(fixture.name+'-cancelled.json'),before)
  await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}))
  await focusByTab(search);await page.keyboard.insertText('Push / Pull');await page.keyboard.press('Enter')
  if(keyboard){await focusByTab(amount);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText('1 mm')}else await amount.fill('1 mm')
  const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true});await activate(apply)
  const after=await exportDoc(fixture.name+'-after.json');assert.equal(after.bodies[0].id,before.bodies[0].id)
  assert.deepEqual(after.bodies[0].brep,fixture.response.value.brep)
  if(process.argv.includes('--step')){
   if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu)
   const pending=page.waitForEvent('download')
   await activate(solid.getByRole('button',{name:'STEP выбранного тела · текущая геометрия',exact:true}))
   const file=fixture.name+'.step';await(await pending).saveAs(path.join(directory,file))
   if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
   const bytes=await readFile(path.join(directory,file))
   const expected=fixture.name==='flange'?{volumeMm3:2625*Math.PI,boundsMm:[[-20,-20,0],[20,20,7]]}:{volumeMm3:fixture.name==='bracket'?6825:7416,boundsMm:[[0,0,0],[40,30,21]]}
   stepParts.push({name:fixture.name,file,sha256:createHash('sha256').update(bytes).digest('hex'),expected})
  }
  await page.screenshot({path:path.join(directory,fixture.name+'.png')})
  await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await exportDoc(fixture.name+'-undo.json'),before)
  await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await exportDoc(fixture.name+'-redo.json'),after)
  await page.reload()
  await solid.getByRole('button',{name:fixture.name,exact:true}).waitFor({state:'visible'})
  assert.deepEqual(await exportDoc(fixture.name+'-reload.json'),after)

 }
 if(stepParts.length)await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:stepParts},null,2))
 assert.deepEqual(errors,[]);await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,keyboard,tabs,cases:fixtures.map(f=>f.name),pushMm:1,undoRedo:true,cancelLateReply:true,selectionChange:process.argv.includes('--selection-change'),documentChange:process.argv.includes('--document-change'),reload:true,errors},null,2))
}catch(e){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw e}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
