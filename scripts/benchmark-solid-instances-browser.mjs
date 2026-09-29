import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {performance} from 'node:perf_hooks'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const [fixture,output='/tmp/solid-instances-browser']=process.argv.slice(2)
if(!fixture)throw Error('Usage: node scripts/benchmark-solid-instances-browser.mjs compact-document.json output-directory')
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
let browser
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 const context=await browser.newContext({viewport:{width:1280,height:800},acceptDownloads:true}),page=await context.newPage(),errors=[]
 page.setDefaultTimeout(120000);page.on('pageerror',error=>errors.push(String(error)))
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 const saved=async()=>{await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'))}
 async function load(value){await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'benchmark.json',mimeType:'application/json',buffer:Buffer.from(value)});await menu.click();await saved()}
 await load(JSON.stringify({version:1,bodies:[],sketches:[]}))
 const cdp=await context.newCDPSession(page);await cdp.send('Performance.enable')
 const heap=async()=>{const {metrics}=await cdp.send('Performance.getMetrics');return Object.fromEntries(metrics.filter(m=>['JSHeapUsedSize','JSHeapTotalSize','Nodes'].includes(m.name)).map(m=>[m.name,m.value]))}
 const before=await heap(),samples=[]
 const profileAction=process.argv.includes('--profile-import')?'import':'redo'
 const profiling=process.argv.includes('--profile')||process.argv.includes('--profile-import')
 let profileCaptured=false
 if(profiling)await cdp.send('Profiler.enable')
 async function measure(action,run){
  const profile=profiling&&action===profileAction&&!profileCaptured
  if(profile)await cdp.send('Profiler.start')
  await page.evaluate(()=>{window.__instanceFrames=[];window.__instanceFramePrevious=performance.now();const generation=window.__instanceRecording=(window.__instanceRecording??0)+1;const frame=t=>{if(window.__instanceRecording!==generation)return;window.__instanceFrames.push(t-window.__instanceFramePrevious);window.__instanceFramePrevious=t;requestAnimationFrame(frame)};requestAnimationFrame(frame)})
  const start=performance.now();await run();await saved()
  const elapsedMs=performance.now()-start
  const frameGapsMs=await page.evaluate(()=>{window.__instanceRecording++;return window.__instanceFrames})
  if(profile){const {profile:cpu}=await cdp.send('Profiler.stop');await writeFile(path.join(directory,profileAction+'.cpuprofile'),JSON.stringify(cpu));profileCaptured=true}
  samples.push({action,profiled:profile,elapsedMs,frameCount:frameGapsMs.length,maxFrameGapMs:Math.max(0,...frameGapsMs)})
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
 for(let i=0;i<5;i++){
  await measure('undo',()=>solid.getByRole('button',{name:'↶',exact:true}).click())
  await measure('redo',()=>solid.getByRole('button',{name:'↷',exact:true}).click())
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
 await menu.click();const download=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await download).saveAs(path.join(directory,'restored.json'));await menu.click()
 const restored=JSON.parse(await readFile(path.join(directory,'restored.json'),'utf8'))
 assert.deepEqual(restored.bodies.map(b=>({id:b.id,instance:b.instance})),expected.bodies.map(b=>({id:b.id,instance:b.instance})))
 for(const source of expected.bodies.filter(body=>!body.instance))assert.deepEqual(restored.bodies.find(body=>body.id===source.id),source)
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'scene.png')})
 const result={scope:'Headless Chromium UI import and Undo/Redo through durable save; automation latency included. RAF gaps during operations are not orbit FPS; heap is sampled without forced GC.',importCancellationChecked:process.argv.includes('--check-import-cancel'),cpuProfile:profileCaptured?profileAction+'.cpuprofile':null,cancellationChecked:process.argv.includes('--check-cancel'),browser:browser.version(),viewport:{width:1280,height:800},bodies:expected.bodies.length,before,after:await heap(),samples}
 await writeFile(path.join(directory,'measurements.json'),JSON.stringify(result,null,2)+'\n')
 console.log(JSON.stringify(result))
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
