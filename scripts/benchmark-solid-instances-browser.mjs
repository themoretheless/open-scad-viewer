import assert from 'node:assert/strict'
import {workerHeapSampler} from './qualificationWorkerHeap.mjs'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {performance} from 'node:perf_hooks'
import os from 'node:os'
import {execFile} from 'node:child_process'
import {promisify} from 'node:util'
const execFileAsync=promisify(execFile)
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const [fixture,output='/tmp/solid-instances-browser']=process.argv.slice(2)
if(!fixture)throw Error('Usage: node scripts/benchmark-solid-instances-browser.mjs compact-document.json output-directory')
const collectWorkers=process.argv.includes('--worker-memory'),collectRetained=process.argv.includes('--retained-memory')||collectWorkers,retainedSamples=[]
const iterations=Number(process.argv.find(arg=>arg.startsWith('--iterations='))?.split('=')[1]??5)
assert.ok(Number.isInteger(iterations)&&iterations>=1&&iterations<=100,'iterations must be between 1 and 100')
const text=await readFile(fixture,'utf8'),expected=JSON.parse(text),root=path.resolve('dist'),directory=path.resolve(output)
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{
 try{
  const url=new URL(req.url,'http://localhost'),file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
  if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser,memoryTimer,memoryPending
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 const context=await browser.newContext({viewport:{width:1280,height:800},acceptDownloads:true}),page=await context.newPage(),errors=[]
 page.setDefaultTimeout(120000);page.on('pageerror',error=>errors.push(String(error)))
 if(process.argv.includes('--disable-cpu-canvas'))await page.addInitScript(()=>{const original=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(...args){return this.hasAttribute('data-cpu-orbit')?null:original.apply(this,args)}})
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 if(process.argv.includes('--disable-outliner-containment'))await page.addStyleTag({content:'.scene-list .object-row{content-visibility:visible!important;contain-intrinsic-block-size:none!important}'})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 const saved=async()=>{await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'))}
 async function load(value){await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'benchmark.json',mimeType:'application/json',buffer:Buffer.from(value)});await menu.click();await saved()}
 await load(JSON.stringify({version:1,bodies:[],sketches:[]}))
 const cdp=await context.newCDPSession(page);await cdp.send('Performance.enable')
 const heap=async()=>{const {metrics}=await cdp.send('Performance.getMetrics');return Object.fromEntries(metrics.filter(m=>['JSHeapUsedSize','JSHeapTotalSize','Nodes'].includes(m.name)).map(m=>[m.name,m.value]))}
 const workerHeap=collectWorkers?workerHeapSampler(await browser.newBrowserCDPSession()):null
 const before=await heap(),samples=[]
 const memoryReadings=[],memoryErrors=[],memoryIntervalMs=200
 if(process.argv.includes('--sample-memory')){
  const processSession=await browser.newBrowserCDPSession()
  const sample=async()=>{
   const start=performance.now(),[{processInfo},js]=await Promise.all([processSession.send('SystemInfo.getProcessInfo'),heap()])
   const ids=processInfo.map(p=>p.id).filter(id=>Number.isSafeInteger(id)&&id>0)
   assert.ok(ids.length)
   const {stdout}=await execFileAsync('ps',['-o','pid=,rss=','-p',ids.join(',')])
   const processes=stdout.trim().split('\n').filter(Boolean).map(row=>{const [pid,kib]=row.trim().split(/\s+/).map(Number);assert.ok(Number.isFinite(kib)&&kib>=0);return {pid,rssBytes:kib*1024}})
   memoryReadings.push({atMs:start,pollMs:performance.now()-start,...js,aggregateRssBytes:processes.reduce((sum,p)=>sum+p.rssBytes,0),processes})
  }
  const poll=()=>{if(!memoryPending)memoryPending=sample().catch(e=>memoryErrors.push(String(e))).finally(()=>{memoryPending=null})}
  poll();memoryTimer=setInterval(poll,memoryIntervalMs)
 }
 const profileAction=process.argv.includes('--profile-source-edit')?'source-edit':process.argv.includes('--profile-import')?'import':'redo'
 const profiling=process.argv.includes('--profile-source-edit')||process.argv.includes('--profile')||process.argv.includes('--profile-import')||process.argv.includes('--profile-orbit')
 let profileCaptured=false
 if(profiling)await cdp.send('Profiler.enable')
 async function measure(action,run){
  const profile=profiling&&!process.argv.includes('--profile-orbit')&&action===profileAction&&!profileCaptured
  if(profile)await cdp.send('Profiler.start')
  await page.evaluate(()=>{window.__instanceFrames=[];window.__instanceFramePrevious=performance.now();const generation=window.__instanceRecording=(window.__instanceRecording??0)+1;const frame=t=>{if(window.__instanceRecording!==generation)return;window.__instanceFrames.push(t-window.__instanceFramePrevious);window.__instanceFramePrevious=t;requestAnimationFrame(frame)};requestAnimationFrame(frame)})
  const start=performance.now();await run();await saved()
  const elapsedMs=performance.now()-start
  const frameGapsMs=await page.evaluate(()=>{window.__instanceRecording++;return window.__instanceFrames})
  if(profile){const {profile:cpu}=await cdp.send('Profiler.stop');await writeFile(path.join(directory,profileAction+'.cpuprofile'),JSON.stringify(cpu));profileCaptured=true}
  samples.push({action,profiled:profile,elapsedMs,heap:await heap(),frameCount:frameGapsMs.length,maxFrameGapMs:Math.max(0,...frameGapsMs)})
  if(collectRetained){await cdp.send('HeapProfiler.collectGarbage');retainedSamples.push({action,...await heap(),...(workerHeap?{workers:await workerHeap()}: {})})}
 }
 if(process.argv.includes('--check-import-cancel')){
  await page.evaluate(()=>{window.__importPosted=false;const post=Worker.prototype.postMessage;Worker.prototype.postMessage=function(message,...args){if(message?.job?.kind==='restoreDocument')window.__importPosted=true;return post.call(this,message,...args)}})
  await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'cancel-import.json',mimeType:'application/json',buffer:Buffer.from(text)})
  await page.waitForFunction(()=>window.__importPosted)
  await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor()
  await page.keyboard.press('Escape');await saved()
  if(await menu.evaluate(e=>e.parentElement.open))await menu.click()
  await menu.click();const cancelled=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await cancelled).saveAs(path.join(directory,'cancelled-import.json'));await menu.click()
  assert.equal(JSON.parse(await readFile(path.join(directory,'cancelled-import.json'),'utf8')).bodies.length,0)
 }
 await measure('import',()=>load(text))
 for(let i=0;i<iterations;i++){
  await measure('undo',()=>solid.getByRole('button',{name:'↶',exact:true}).click())
  await measure('redo',()=>solid.getByRole('button',{name:'↷',exact:true}).click())
 }
 if(process.argv.includes('--source-edit')){
  const source=expected.bodies.find(body=>!body.instance);assert.ok(source)
  for(let i=0;i<iterations;i++){
   await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
   await solid.getByRole('button',{name:source.name,exact:true}).click()
   await solid.getByRole('tab',{name:'Свойства',exact:true}).click()
   const x=solid.getByRole('textbox',{name:'ΔX',exact:true});await x.fill('1 mm')
   await measure('source-edit',async()=>{
    await solid.getByRole('button',{name:'Применить',exact:true}).click()
    await page.waitForFunction(()=>document.querySelector('input[aria-label="ΔX"]')?.value==='0')
   })
   await measure('source-edit-undo',()=>solid.getByRole('button',{name:'↶',exact:true}).click())
  }
 }
 if(process.argv.includes('--check-cancel')){
  await solid.getByRole('button',{name:'↶',exact:true}).click();await saved()
  await solid.getByRole('button',{name:'↷',exact:true}).click()
  await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor()
  await page.keyboard.press('Escape');await saved()
  await menu.click();const cancelledDownload=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await cancelledDownload).saveAs(path.join(directory,'cancelled.json'));await menu.click()
  assert.equal(JSON.parse(await readFile(path.join(directory,'cancelled.json'),'utf8')).bodies.length,0)
  assert.equal(await solid.getByRole('button',{name:'↷',exact:true}).isEnabled(),true)
  await solid.getByRole('button',{name:'↷',exact:true}).click();await saved()
 }
 if(process.argv.includes('--reimport'))await measure('reimport',()=>load(text))
 if(process.argv.includes('--check-outliner')){
  const first=solid.getByRole('button',{name:expected.bodies[0].name,exact:true})
  await first.focus();await first.press('End')
  const last=solid.getByRole('button',{name:expected.bodies.at(-1).name,exact:true})
  await last.waitFor();assert.equal(await last.evaluate(e=>document.activeElement===e),true)
  await last.press('Enter');assert.equal(await last.getAttribute('aria-pressed'),'true')
  if(process.argv.includes('--check-outliner-controls')){
   const name=expected.bodies.at(-1).name
   const activate=async(label,nextLabel)=>{const button=solid.getByRole('button',{name:label+name,exact:true});await button.focus();await button.press('Enter');await writeFile(path.join(directory,'control-'+nextLabel.trim().replace(':','')+'.json'),JSON.stringify(await page.evaluate(()=>({active:{tag:document.activeElement?.tagName,label:document.activeElement?.getAttribute('aria-label'),class:document.activeElement?.className},rows:[...document.querySelectorAll('.object-row')].map(e=>({key:e.getAttribute('data-scene-key'),buttons:[...e.querySelectorAll('button')].map(b=>({label:b.getAttribute('aria-label'),focused:b===document.activeElement}))}))})),null,2));await page.screenshot({path:path.join(directory,'control-'+nextLabel.trim().replace(':','')+'.png')});await page.waitForFunction(label=>document.activeElement?.getAttribute('aria-label')===label,nextLabel+name,{timeout:3000})}
   await activate('Скрыть: ','Показать: ');assert.equal(await last.isDisabled(),true)
   await activate('Показать: ','Скрыть: ');assert.equal(await last.isDisabled(),false)
   await activate('Заблокировать: ','Разблокировать: ');assert.equal(await last.isDisabled(),true)
   await activate('Разблокировать: ','Заблокировать: ');assert.equal(await last.isDisabled(),false)
   await last.focus();await last.press('Enter');assert.equal(await last.getAttribute('aria-pressed'),'true')
   const group=expected.bodies.at(-1).group
   if(group){
    const isolate=solid.getByRole('button',{name:'Изолировать группу: '+group,exact:true})
    await isolate.focus();await isolate.press('Enter')
    const exit=solid.getByRole('button',{name:'Выйти из изоляции',exact:true})
    await exit.waitFor();assert.equal(await exit.getAttribute('aria-pressed'),'true')
    assert.equal(await last.isDisabled(),false)
    await exit.focus();await exit.press('Enter')
    await solid.getByRole('button',{name:'Изолировать тела',exact:true}).waitFor()
    await last.focus()
   }

  }

  const position=await last.evaluate(e=>{const row=e.closest('li');return {position:Number(row?.getAttribute('aria-posinset')),total:Number(row?.getAttribute('aria-setsize'))}})
  assert.equal(position.position,position.total);assert.ok(position.total>=expected.bodies.length)
  assert.ok(await solid.locator('.object-row').count()<=40,'The large outliner must bound live object rows')
  await page.screenshot({path:path.join(directory,'outliner-last.png')})
  await solid.getByRole('tab',{name:'Свойства',exact:true}).click();await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
  await last.waitFor();assert.equal(await last.getAttribute('aria-pressed'),'true');await last.focus()
  await last.press('Home');await page.waitForFunction(()=>document.activeElement?.classList.contains('group-name'));await page.keyboard.press('ArrowDown');await first.waitFor();await page.waitForFunction(name=>document.activeElement?.getAttribute('aria-label')===name,expected.bodies[0].name);assert.equal(await first.evaluate(e=>document.activeElement===e),true)
  await first.press('Enter');assert.equal(await first.getAttribute('aria-pressed'),'true')
  await page.screenshot({path:path.join(directory,'outliner-first.png')})
  for(let index=1;index<=70;index++){
   await page.keyboard.press('ArrowDown')
   await page.waitForFunction(name=>document.activeElement?.getAttribute('aria-label')===name,expected.bodies[index].name)
  }
  await page.keyboard.press('Tab');await page.keyboard.press('Tab');await page.keyboard.press('Tab')
  await page.waitForFunction(name=>document.activeElement?.getAttribute('aria-label')===name,expected.bodies[71].name)
  await page.keyboard.press('Shift+Tab')
  const pinnedLabel='Заблокировать: '+expected.bodies[70].name
  await page.waitForFunction(name=>document.activeElement?.getAttribute('aria-label')===name,pinnedLabel)
  await solid.locator('.dock-body').evaluate(e=>{e.scrollTop=0})
  await first.waitFor()
  assert.equal(await page.evaluate(name=>document.activeElement?.getAttribute('aria-label')===name,pinnedLabel),true)
  assert.ok(await solid.locator('.object-row').count()<=40,'Scrolled focused row must remain pinned without expanding the window')

 }

 if(process.argv.includes('--check-outliner-delete')){
  await page.keyboard.press('End')
  const last=solid.getByRole('button',{name:expected.bodies.at(-1).name,exact:true})
  await last.waitFor();await last.focus();await last.press('Enter');await last.press('Delete');await saved()
  await last.waitFor({state:'detached'})
  await page.waitForFunction(()=>document.activeElement?.classList.contains('scene-list'))
  await page.screenshot({path:path.join(directory,'outliner-deleted.png')})
  await page.keyboard.press('Home')
  await page.waitForFunction(()=>document.activeElement?.classList.contains('group-name'),{},{timeout:3000})
  await solid.getByRole('button',{name:'↶',exact:true}).click();await saved()
 }

 let orbit=null
 if(process.argv.includes('--orbit')){
  for(const name of ['snap-preparation','sketch-snap-preparation','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})
  const svg=solid.locator('svg[aria-label="Холст тел 3D"]'),box=await svg.boundingBox()
  assert.ok(box)
  const x=box.x+box.width/2,y=box.y+box.height/2
  await page.screenshot({path:path.join(directory,'orbit-before.png')})
  await page.mouse.move(x,y);await page.mouse.down({button:'right'})
  if(process.argv.includes('--profile-orbit'))await cdp.send('Profiler.start')
  await page.evaluate(()=>{window.__orbitFrames=[];window.__orbitStart=performance.now();window.__orbitPrevious=window.__orbitStart;window.__orbitRecording=true;const frame=t=>{if(!window.__orbitRecording)return;window.__orbitFrames.push(t-window.__orbitPrevious);window.__orbitPrevious=t;requestAnimationFrame(frame)};requestAnimationFrame(frame)})
  for(let i=1;i<=180;i++)await page.mouse.move(x+100*Math.sin(i/35),y+40*Math.sin(i/29))
  orbit=await page.evaluate(()=>{window.__orbitRecording=false;return {durationMs:performance.now()-window.__orbitStart,frameGapsMs:window.__orbitFrames}})
  if(process.argv.includes('--disable-cpu-canvas'))assert.ok(await svg.locator('polygon[data-body]').count()>1200,'Unavailable canvas must retain SVG geometry during motion')
  if(process.argv.includes('--check-cpu-canvas')){
   await page.waitForFunction(()=>{const c=document.querySelector('[data-cpu-orbit]');return c&&c.getBoundingClientRect().width>0&&!document.querySelector('polygon[data-body]')})
   const canvasState=await page.evaluate(()=>{const c=document.querySelector('[data-cpu-orbit]'),pixels=c.getContext('2d').getImageData(0,0,c.width,c.height).data;return {width:c.width,height:c.height,paintedPixels:pixels.filter((v,i)=>i%4===3&&v>0).length}})
   assert.ok(canvasState.paintedPixels>0,'Canvas must contain rendered geometry');orbit.canvas=canvasState
   await page.screenshot({path:path.join(directory,'orbit-during.png')})
  }
  if(process.argv.includes('--profile-orbit')){const {profile}=await cdp.send('Profiler.stop');await writeFile(path.join(directory,'orbit.cpuprofile'),JSON.stringify(profile))}
  if(process.argv.includes('--compare-orbit-paint'))await svg.screenshot({path:path.join(directory,'paint-canvas.png')})
  await page.mouse.up({button:'right'});await saved()
  if(process.argv.includes('--compare-orbit-paint'))await svg.screenshot({path:path.join(directory,'paint-svg.png')})
  assert.ok(orbit.frameGapsMs.length>1,'Orbit must produce animation frames')
  orbit.rafFps=orbit.frameGapsMs.length*1000/orbit.durationMs
  orbit.pointerMoves=180
  orbit.scope='RAF cadence during 180 automated right-button orbit moves; includes input automation, headless rendering, not physical display presentation rate.'
  const hit=await svg.evaluate(svg=>{
   for(const polygon of svg.querySelectorAll('polygon[data-body]')){
    const points=[...polygon.points];if(!points.length)continue
    const center=new DOMPoint(points.reduce((n,p)=>n+p.x,0)/points.length,points.reduce((n,p)=>n+p.y,0)/points.length).matrixTransform(svg.getScreenCTM())
    const front=document.elementFromPoint(center.x,center.y)
    if(front===polygon)return {id:polygon.getAttribute('data-body'),x:center.x,y:center.y}
   }
   return null
  })
  assert.ok(hit,'A visible polygon must remain pickable after orbit')
  await page.mouse.click(hit.x,hit.y)
  const body=expected.bodies.find(body=>body.id===hit.id);assert.ok(body)
  assert.equal(await solid.getByRole('button',{name:body.name,exact:true}).getAttribute('aria-pressed'),'true')
  orbit.pickAfterOrbit=hit.id
  await page.screenshot({path:path.join(directory,'orbit-after.png')})
 }
 await menu.click();const download=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await download).saveAs(path.join(directory,'restored.json'));await menu.click()
 const restored=JSON.parse(await readFile(path.join(directory,'restored.json'),'utf8'))
 assert.deepEqual(restored.bodies.map(b=>({id:b.id,instance:b.instance,material:b.material})),expected.bodies.map(b=>({id:b.id,instance:b.instance,material:b.material})))
 for(const source of expected.bodies.filter(body=>!body.instance))assert.deepEqual(restored.bodies.find(body=>body.id===source.id),source)
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'scene.png')})
 const percentile=(values,p)=>{const sorted=[...values].sort((a,b)=>a-b);return sorted[Math.max(0,Math.ceil(sorted.length*p)-1)]??null}
 const timingSummary=Object.fromEntries(['import','undo','redo',...(process.argv.includes('--source-edit')?['source-edit','source-edit-undo']:[]),...(process.argv.includes('--reimport')?['reimport']:[])].map(action=>{
  const measured=samples.filter(sample=>sample.action===action&&!sample.profiled)
  return [action,{count:measured.length,p50Ms:percentile(measured.map(s=>s.elapsedMs),.5),p95Ms:percentile(measured.map(s=>s.elapsedMs),.95),maxFrameGapMs:Math.max(0,...measured.map(s=>s.maxFrameGapMs))}]
 }))
 clearInterval(memoryTimer);await memoryPending
 assert.deepEqual(memoryErrors,[])
 const memory=process.argv.includes('--sample-memory')?{
  intervalMs:memoryIntervalMs,samples:memoryReadings,
  maxObservedJsHeapBytes:Math.max(0,...memoryReadings.map(s=>s.JSHeapUsedSize)),
  maxObservedAggregateRssBytes:Math.max(0,...memoryReadings.map(s=>s.aggregateRssBytes)),
  scope:'Polling during the whole run, including operations and orbit. Main-page JS heap excludes workers; aggregate Chromium RSS includes shared pages more than once. Sampled maxima are not a guaranteed instantaneous peak. CDP and ps instrumentation can affect timings.',
 }:null
 const retainedMemory=collectRetained?{scope:'Main-page heap after explicit GC after each operation, outside its elapsed timer. GC changes subsequent operation conditions; these timings are not comparable with ordinary runs. Native/process allocations are excluded. Optional worker records report each worker isolate separately after GC; ended workers are marked explicitly.',samples:retainedSamples}:null
 const result={outlinerContainmentDisabled:process.argv.includes('--disable-outliner-containment'),outlinerChecked:process.argv.includes('--check-outliner'),outlinerControlsChecked:process.argv.includes('--check-outliner-controls'),outlinerDeletionChecked:process.argv.includes('--check-outliner-delete'),retainedMemory,memory,scope:'Headless Chromium UI import and Undo/Redo through durable save; automation latency included. RAF gaps during operations are not orbit FPS; heap is sampled '+(collectRetained?'with separate forced-GC diagnostics.':'without forced GC.'),importCancellationChecked:process.argv.includes('--check-import-cancel'),cpuProfile:process.argv.includes('--profile-orbit')?'orbit.cpuprofile':profileCaptured?profileAction+'.cpuprofile':null,cancellationChecked:process.argv.includes('--check-cancel'),browser:browser.version(),machine:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0]?.model,logicalCpus:os.cpus().length,ramBytes:os.totalmem()},iterations,timingSummary,orbit,maxObservedHeapBytes:Math.max(before.JSHeapUsedSize,...samples.map(s=>s.heap.JSHeapUsedSize)),heapScope:'Samples after each operation only; not peak process memory.',viewport:{width:1280,height:800},bodies:expected.bodies.length,before,after:await heap(),samples}
 await writeFile(path.join(directory,'measurements.json'),JSON.stringify(result,null,2)+'\n')
 console.log(JSON.stringify(result))
}finally{clearInterval(memoryTimer);await memoryPending;await browser?.close();await new Promise(resolve=>server.close(resolve))}
