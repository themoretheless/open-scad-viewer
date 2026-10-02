import assert from 'node:assert/strict'
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
   constructor(...args){super(...args);this.__starts=new Map();this.addEventListener('message',e=>{if(e.data?.kind==='faceDistance'&&e.data.ok){window.__distanceResults.push(e.data.result);const start=this.__starts.get(e.data.id);if(start!==undefined){window.__distanceTimings.push({workerRoundTripMs:performance.now()-start,cells:e.data.result.cells,domainCells:e.data.result.domainCells,converged:e.data.result.converged});this.__starts.delete(e.data.id)}}})}
   postMessage(message,...args){if(message?.job?.kind==='faceDistance'){window.__distanceRequests++;if(window.__failDistance){window.__failDistance=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private distance worker failure'}}}));return}if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}this.__starts.set(message.id,performance.now())}return super.postMessage(message,...args)}
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
 async function command(name){await closeMenu();await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,name);await search.press('Enter')}
 const fixturePath=process.argv.find(a=>a.startsWith('--fixture='))?.slice('--fixture='.length)??'tests/fixtures/face-distance.json'
 const fixture=JSON.parse(await readFile(fixturePath,'utf8'))
 await ready();await activate(menu)
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'faces.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture.document))})
 await closeMenu();await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
 await activate(solid.getByRole('button',{name:fixture.bodyAName??'Plate with hole',exact:true}))
 const before=await exportDoc('before.json')
 await command('Measure vertices / edge')
 await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),fixture.bodyBId??'probe')
 await activate(solid.getByRole('button',{name:'Расстояние между гранями',exact:true}))
 const field=solid.getByRole('group',{name:'face-distance',exact:true})
 const edgeA=fixture.faceA,edgeB=fixture.faceB
 await input(field.getByRole('spinbutton',{name:'Грань A',exact:true}),String(edgeA+1))
 await input(field.getByRole('spinbutton',{name:'Грань B',exact:true}),String(edgeB+1))
 await choose(field.getByRole('combobox'),'100000')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const interval=(await field.locator('[data-face-distance]').innerText()).replace(' mm','').split(' … ').map(Number)
 const expected=fixture.expectedMm
 assert.ok(interval[0]<=expected&&interval[1]>=expected&&interval[1]-interval[0]<=.001)
 assert.equal(await solid.locator('[data-measurement="face-distance"] circle').count(),2)
 await input(field.getByRole('spinbutton',{name:'Грань B',exact:true}),'9999')
 await field.getByRole('alert').filter({hasText:'Укажите существующую грань B.'}).waitFor()
 assert.equal(await solid.locator('[data-measurement="face-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=true)
 await input(field.getByRole('spinbutton',{name:'Грань B',exact:true}),String(edgeB+1))
 await page.waitForFunction(()=>window.__distanceHeld)
 await field.getByRole('spinbutton',{name:'Грань B',exact:true}).press('Escape')
 await page.waitForFunction(()=>window.__distanceTerminated)
 assert.equal(await solid.locator('[data-measurement="face-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=false)
 await command('Measure vertices / edge')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 await page.evaluate(()=>window.__failDistance=true);await activate(solid.getByRole('button',{name:'Расстояние между гранями',exact:true}));await activate(solid.getByRole('button',{name:'Расстояние между гранями',exact:true}))
 await field.getByRole('alert').waitFor();assert.equal((await field.innerText()).includes('Private distance'),false);assert.equal(await solid.locator('[data-measurement="face-distance"]').count(),0)
 await activate(field.getByRole('button',{name:'Повторить измерение граней',exact:true}));await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 await field.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,'face-distance.png')})
 assert.deepEqual(errors,[])
 const nativeResult=await page.evaluate(()=>window.__distanceResults.at(-1)),requests=await page.evaluate(()=>window.__distanceRequests);assert.ok(nativeResult);assert.ok(requests>=3)
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),before);assert.deepEqual(errors,[])
 const timings=await page.evaluate(()=>window.__distanceTimings)
 const report={timings,timingScope:'Post request through receipt of worker response; includes startup and messaging, excludes subsequent rendering.',workerFailureRetry:true,keyboard,tabs,gpuActive,reloadExact:true,browser:browser.version(),fixturePath,faceA:edgeA,faceB:edgeB,expectedMm:expected,displayedIntervalMm:interval,nativeResult,requests,cancelledWorkerTerminated:true,invalidFaceLocalized:true,documentUnchanged:true}
 await writeFile(path.join(directory,'face-distance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
