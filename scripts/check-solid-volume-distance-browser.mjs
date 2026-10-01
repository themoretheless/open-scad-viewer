import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-volume-distance')
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
const errors=[]
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='solidDistance'&&e.data.ok)window.__distanceResults.push(e.data.result)})}
   postMessage(message,...args){if(message?.job?.kind==='solidDistance'){window.__distanceRequests++;if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__distanceTerminated=true;return super.terminate()}
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
 async function activate(locator){if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
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
 const native=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/solid-distance-2026-09-30/contract-fixtures.json','utf8')).cases
 function documentFor(request){
  return {version:1,sketches:[],bodies:[request.a,request.b].map((brep,i)=>{
   const positions=brep.vertices.flatMap(v=>v.point),indices=[]
   for(const shell of brep.shells)for(const use of shell.faces){
    const face=brep.faces[use.face],vertices=brep.loops[face.outer].coedges.map(c=>brep.edges[c.edge].vertices[c.reversed?1:0])
    for(let j=1;j<vertices.length-1;j++)indices.push(...(use.reversed?[vertices[0],vertices[j+1],vertices[j]]:[vertices[0],vertices[j],vertices[j+1]]))
   }
   return {id:i?'b':'a',name:i?'Body B':'Body A',brep,mesh:{positions,indices}}
  })}
 }
 const results=[]
 for(const [index,c] of native.entries()){
  await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)
  await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'volumes.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(documentFor(c.request)))})
  await closeMenu();await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
  await activate(solid.getByRole('button',{name:'Body A',exact:true}))
  const before=await exportDoc(`before-${index}.json`)
  await command('Measure vertices / edge');await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'b')
  const toggle=solid.getByRole('button',{name:'Расстояние между объёмами',exact:true})
  if(await toggle.getAttribute('aria-pressed')!=='true')await activate(toggle)
  const field=solid.getByRole('group',{name:'solid-volume-distance',exact:true})
  await field.getByText('Проверяю объёмы и расстояние…',{exact:true}).waitFor({state:'hidden',timeout:120000})
  if(index===1){await field.getByText('Body B: ориентация оболочек не подтверждена',{exact:true}).waitFor();assert.equal(await field.locator('[data-solid-distance]').count(),0)}
  else if(index>=3){await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor();assert.equal(await field.locator('[data-solid-witness]').count(),2);const marker=solid.locator('[data-measurement="volume-separation"]');assert.equal(await marker.locator('circle').count(),2);assert.equal(await marker.locator('line').count(),1)}
  else {await field.getByText('Общие точки тел подтверждены. Расстояние — 0 мм.',{exact:true}).waitFor();assert.equal(await field.locator('[data-solid-distance]').innerText(),'0 … 0 mm')}
  assert.equal(await solid.locator('[data-measurement="distance"]').count(),0)
  assert.equal(await solid.locator('[data-measurement="volume-contact"]').count(),index===2?1:0)
  const result=await page.evaluate(()=>window.__distanceResults.at(-1))
  if(index>=3){const gap=index===3?3:1;assert.equal(result.reason,'separated-volumes');assert.ok(result.distanceIntervalMm[0]<=gap&&result.distanceIntervalMm[1]>=gap);assert.ok(result.separationWitness)}
  results.push(result)
  assert.deepEqual(await exportDoc(`after-${index}.json`),before)
  await field.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,`volume-${index}.png`)})
  if(index===0){
   await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'a')
   await field.getByRole('alert').filter({hasText:'Выберите два разных тела B-rep'}).waitFor()
   assert.equal(await solid.locator('[data-measurement="volume-contact"]').count(),0)
   await page.evaluate(()=>window.__holdDistance=true)
   await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'b');await page.waitForFunction(()=>window.__distanceHeld)
   await focusByTab(field.getByRole('button',{name:'Закрыть · Esc',exact:true}));await page.keyboard.press('Escape')
   await page.waitForFunction(()=>window.__distanceTerminated);await page.evaluate(()=>window.__holdDistance=false)
  }
 }
 assert.deepEqual(errors,[])
 const artifact={geometryWasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex'),indexSha256:createHash('sha256').update(await readFile(path.join(root,'index.html'))).digest('hex')}
 const report={artifact,browser:browser.version(),keyboard,tabs,results,requests:await page.evaluate(()=>window.__distanceRequests),cancelledWorkerTerminated:true,invalidTargetLocalized:true,documentUnchanged:true}
 await writeFile(path.join(directory,'solid-volume-distance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
