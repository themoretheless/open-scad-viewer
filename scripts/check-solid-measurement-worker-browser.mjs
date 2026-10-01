import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
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
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))});page.on('console',message=>{if(message.type()==='error')errors.push(message.text())})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__workerRoundTrips=[];const awaitingDom=[]
  new MutationObserver(()=>{for(let i=awaitingDom.length-1;i>=0;i--){const entry=awaitingDom[i],selector=entry.kind==='measureVertices'?'[data-measurement="distance"]':'[data-measurement="curvature"]';if(!document.querySelector(selector))continue;awaitingDom.splice(i,1);entry.domPublicationMs=performance.now()-entry.receivedAt;requestAnimationFrame(()=>entry.nextAnimationFrameMs=performance.now()-entry.receivedAt)}}).observe(document,{subtree:true,childList:true,characterData:true,attributes:true})
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.__timingStarted=performance.now();this.__jobs=new Map();this.addEventListener('message',event=>{const data=event.data,job=this.__jobs.get(data?.id);if(!job||data?.kind!==job.kind)return;this.__jobs.delete(data.id);const receivedAt=performance.now(),entry={...job,ok:data.ok===true,timing:data.timing,receivedAt,roundTripMs:receivedAt-job.dispatchAt};window.__workerRoundTrips.push(entry);if(entry.ok&&['measureVertices','measureEdge'].includes(entry.kind))awaitingDom.push(entry)})}
   postMessage(message,...args){const start=performance.now();const job={kind:message?.job?.kind,id:message?.id,firstRequest:this.__jobs.size===0&&!this.__hasPosted,workerAgeMs:start-this.__timingStarted,dispatchAt:start,postMessageMs:0};this.__jobs.set(message.id,job);this.__hasPosted=true;const result=super.postMessage(message?.job?{...message,traceTiming:true}:message,...args);job.postMessageMs=performance.now()-start;return result}
   terminate(){for(const job of this.__jobs.values())window.__workerRoundTrips.push({...job,cancelled:true,elapsedUntilTerminationMs:performance.now()-job.dispatchAt});this.__jobs.clear();return super.terminate()}
  }
 })
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__measurementRequests=0;window.__holdMeasurement=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(['measureVertices','measureEdge'].includes(message?.job?.kind)){
     window.__measurementRequests++
     if(window.__failMeasurement===message.job.kind){window.__failMeasurement=null;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private measurement transport failure'}}}));return}
     if(window.__holdMeasurement&&message.job.kind==='measureVertices'){this.__held=true;window.__measurementHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__measurementTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 let tabPresses=0
 async function tabTo(locator){for(let i=0;i<300;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabPresses++}throw Error('Unreachable keyboard control: '+await locator.getAttribute('aria-label'))}
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await tabTo(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 async function select(locator,value){if(keyboard){await tabTo(locator);const index=await locator.evaluate((e,v)=>Array.from(e.options).findIndex(o=>o.value===v),value);assert.ok(index>=0);await page.keyboard.press('Home');for(let i=0;i<index;i++)await page.keyboard.press('ArrowDown');assert.equal(await locator.inputValue(),value)}else await locator.selectOption(value)}
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu);const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function command(name){await closeMenu();await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,name);await search.press('Enter')}
 await ready();await command('Cylinder');await ready()
 const before=await exportDoc('before.json'),body=before.bodies.at(-1)
 await command('Measure vertices / edge');await page.waitForFunction(()=>window.__measurementHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__measurementTerminated)
 assert.equal(await solid.locator('[data-measurement="distance"]').count(),0)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdMeasurement=false);await command('Measure vertices / edge')
 await solid.getByRole('status',{name:'vertex-measurement',exact:true}).waitFor({state:'hidden'})
 await solid.locator('[data-measurement="distance"]').waitFor()
 const a=body.brep.vertices[0].point,b=body.brep.vertices[1].point,expected=Math.hypot(...a.map((v,i)=>b[i]-v))
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes(expected.toFixed(6)+' mm')))
 await activate(solid.getByRole('button',{name:'Рёбра',exact:true}))
 const edge=body.brep.edges.findIndex(e=>e.curve.degree===2)
 await select(solid.getByRole('combobox',{name:'Выбрать ребро',exact:true}),String(edge))
 await solid.getByRole('status',{name:'edge-measurement',exact:true}).waitFor({state:'hidden'})
 await solid.locator('[data-measurement="curvature"]').waitFor()
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('10.000000 mm')))
 await input(solid.getByRole('spinbutton',{name:'Параметр ребра',exact:true}),'0.25')
 await solid.getByRole('status',{name:'edge-measurement',exact:true}).waitFor({state:'hidden'})
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('10.000000 mm')))
 const vertexB=solid.getByRole('spinbutton',{name:'Вершина B',exact:true}),parameter=solid.getByRole('spinbutton',{name:'Параметр ребра',exact:true})
 await input(vertexB,'9999');await solid.getByRole('alert').filter({hasText:'Укажите существующую вершину B.'}).waitFor()
 assert.equal(await vertexB.getAttribute('aria-invalid'),'true');assert.equal(await vertexB.getAttribute('aria-describedby'),'vertex-measurement-error');await page.screenshot({path:path.join(directory,'invalid-vertex.png')})
 assert.equal(await solid.locator('[data-measurement="distance"]').count(),0)
 await page.evaluate(()=>window.__failMeasurement='measureVertices');await input(vertexB,'2')
 await solid.getByRole('alert').filter({hasText:'Не удалось измерить вершины.'}).waitFor()
 assert.equal((await solid.innerText()).includes('Private measurement'),false)
 await activate(solid.getByRole('button',{name:'Повторить измерение вершин',exact:true}));await solid.locator('[data-measurement="distance"]').waitFor()
 await input(parameter,'2');await solid.getByRole('alert').filter({hasText:'Задайте параметр ребра от 0 до 1.'}).waitFor()
 assert.equal(await parameter.getAttribute('aria-invalid'),'true');assert.equal(await parameter.getAttribute('aria-describedby'),'curvature-measurement-error');await page.screenshot({path:path.join(directory,'invalid-parameter.png')})
 assert.equal(await solid.locator('[data-measurement="curvature"]').count(),0)
 await page.evaluate(()=>window.__failMeasurement='measureEdge');await input(parameter,'0.25')
 await solid.getByRole('alert').filter({hasText:'Не удалось измерить кривизну.'}).waitFor()
 await activate(solid.getByRole('button',{name:'Повторить измерение кривизны',exact:true}));await solid.locator('[data-measurement="curvature"]').waitFor()
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('10.000000 mm')))
 assert.deepEqual(await exportDoc('measured.json'),before)
 const requests=await page.evaluate(()=>window.__measurementRequests);assert.ok(requests>=4)
 await page.screenshot({path:path.join(directory,'measurements.png')})
 const roundTrips=await page.evaluate(()=>window.__workerRoundTrips);assert.ok(roundTrips.some(r=>r.kind==='measureVertices'&&Number.isFinite(r.domPublicationMs)));assert.ok(roundTrips.some(r=>r.kind==='measureEdge'&&Number.isFinite(r.domPublicationMs)));await writeFile(path.join(directory,'worker-round-trips.json'),JSON.stringify(roundTrips,null,2)+'\n')
 await page.keyboard.press('Escape')
 await activate(solid.getByRole('button',{name:'↶',exact:true}));await ready()
 const undone=await exportDoc('measurement-undone.json');assert.equal(undone.bodies.length,before.bodies.length-1)
 await activate(solid.getByRole('button',{name:'↷',exact:true}));await ready();assert.deepEqual(await exportDoc('measurement-redone.json'),before)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready()
 assert.deepEqual(await exportDoc('measurement-reloaded.json'),before)
 assert.deepEqual(errors,[])
 const gpuActive=await solid.locator('.gpu-layer').evaluate(canvas=>canvas.style.visibility==='visible')
 if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 const report={historyUntouched:true,undoRedo:true,reloadExact:true,gpuActive,keyboard,tabPresses,fieldErrors:true,localizedWorkerFailures:true,retryVertex:true,retryCurvature:true,browser:browser.version(),workerRequests:requests,cancelledWorkerTerminated:true,vertexDistanceMm:expected,edgeRadiusMm:10,documentUnchanged:true}
 await writeFile(path.join(directory,'measurement-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
