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
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1100}});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))});page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[];window.__distanceTimings=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.__starts=new Map();this.addEventListener('message',e=>{if(['materialChord','materialSegment','materialWall','wholeWall'].includes(e.data?.kind)&&e.data.ok){window.__distanceResults.push(e.data.result);const start=this.__starts.get(e.data.id);if(start!==undefined){window.__distanceTimings.push({workerRoundTripMs:performance.now()-start,cells:e.data.result.cells,domainCells:e.data.result.domainCells,converged:e.data.result.converged});this.__starts.delete(e.data.id)}}})}
   postMessage(message,...args){if(['materialChord','materialSegment','materialWall','wholeWall'].includes(message?.job?.kind)){window.__distanceRequests++;if(window.__failDistance){window.__failDistance=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private distance worker failure'}}}));return}if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}this.__starts.set(message.id,performance.now())}return super.postMessage(message,...args)}
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

 const source=process.env.CAD_MIXED_UI_EVIDENCE,reference=process.env.CAD_MIXED_UI_STEP_OUTPUT
 assert.ok(source&&reference,'Set CAD_MIXED_UI_EVIDENCE and CAD_MIXED_UI_STEP_OUTPUT')
 const manifest=JSON.parse(await readFile(path.join(reference,'manifest.json'),'utf8')),parts=[]
 async function importJson(document){await ready();await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'part.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(document))});await closeMenu();await ready()}
 async function step(file){await ready();await activate(menu);const event=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'STEP выбранного тела · текущая геометрия',exact:true}));const download=await event;await download.saveAs(path.join(directory,file));await closeMenu();return readFile(path.join(directory,file))}
 for(const name of ['bracket','enclosure','flange']){
  const document=JSON.parse(await readFile(path.join(source,name+'-20.json'),'utf8'))
  await importJson(document);await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:document.bodies[0].name,exact:true}))
  const before=await exportDoc(name+'-before.json')
  for(let cycle=0;cycle<2;cycle++){
   const part=name+'-ui20-'+cycle,file=part+'.step',bytes=await step(file)
   const expected=manifest.parts.find(p=>p.name===part).expected
   parts.push({name:part,file,sha256:createHash('sha256').update(bytes).digest('hex'),expected})
   if(cycle===0){
    assert.deepEqual(await exportDoc(name+'-after.json'),before)
    await importJson({version:1,sketches:[],bodies:[]});await activate(menu)
    await solid.locator('input[accept=".step,.stp"]').setInputFiles(path.join(directory,file))
    await page.waitForFunction(()=>document.querySelector('input[accept=".step,.stp"]').value==='')
    await closeMenu();await ready()
    const imported=await exportDoc(name+'-imported.json');assert.equal(imported.bodies.length,1)
    await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:imported.bodies[0].name,exact:true}))
   }
  }
 }
 assert.deepEqual(errors,[])
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({...manifest,parts},null,2)+'\n')
 await writeFile(path.join(directory,'report.json'),JSON.stringify({passed:true,actualStepMenu:true,sourceUnchanged:true,keyboard,errors,files:parts.length,wasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex')},null,2)+'\n')
 console.log(JSON.stringify({passed:true,keyboard,actualStepMenu:true,files:parts.length}))
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
