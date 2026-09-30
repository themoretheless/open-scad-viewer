import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
const gpu=process.argv.includes('--gpu')
function checkFragmentCount(actual,source){if(gpu)assert.equal(actual,source);else assert.ok(actual>=source,'CPU BSP must retain all source triangles or their fragments')}
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
 browser=await playwright.chromium.launch({headless:!gpu,args:gpu?['--enable-unsafe-webgpu']:[]})
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__surfaceRequests=0;window.__holdSurface=!sessionStorage.getItem('surface-no-hold')
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='surfaceMesh'){
     window.__surfaceRequests++
     if(window.__holdSurface){this.__held=true;window.__surfaceHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__surfaceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 let fixture=await readFile('tests/fixtures/solid-surface-boundary.json','utf8')
 if(process.argv.includes('--occlusion')){
  const document=JSON.parse(fixture),yaw=Math.PI/4,pitch=Math.atan(1/Math.sqrt(2)),right=[Math.cos(yaw),-Math.sin(yaw),0],up=[Math.sin(yaw)*Math.sin(pitch),Math.cos(yaw)*Math.sin(pitch),-Math.cos(pitch)],toward=[Math.sin(yaw)*Math.cos(pitch),Math.cos(yaw)*Math.cos(pitch),Math.sin(pitch)]
  const positions=[[-30,-30],[30,-30],[30,30],[-30,30]].flatMap(([x,y])=>right.map((v,i)=>x*v+y*up[i]+(process.argv.includes('--behind')?-100:100)*toward[i]))
  document.bodies.push({id:'occluder',name:'Front occluder',mesh:{positions,indices:[0,1,2,0,2,3]}})
  fixture=JSON.stringify(document)
 }
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'surfaces.json',mimeType:'application/json',buffer:Buffer.from(fixture)});await closeMenu();await ready()
 await page.waitForFunction(()=>window.__surfaceHeld)
 const before=await exportDoc('before.json')
 assert.equal(await solid.locator('[data-surface]').count(),0)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__surfaceTerminated)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdSurface=false)
 await solid.getByRole('button',{name:'Обновить поверхности',exact:true}).click()
 const displayReady=async()=>{await ready();await solid.getByRole('status',{name:'surface-display',exact:true}).waitFor({state:'hidden'})}
 await displayReady();assert.ok((await solid.locator('[data-surface=\"surface-a\"]').count())>=64);assert.ok((await solid.locator('[data-surface=\"surface-b\"]').count())>=64)
 if(gpu||process.argv.includes('--occlusion')){
  const point=await solid.locator('[data-surface="surface-a"]').first().evaluate(n=>{const p=new DOMPoint(Array.from(n.points).reduce((s,p)=>s+p.x,0)/3,Array.from(n.points).reduce((s,p)=>s+p.y,0)/3).matrixTransform(n.getScreenCTM());return {x:p.x,y:p.y}})
  if(process.argv.includes('--occlusion')){
   await page.mouse.click(point.x,point.y)
   assert.equal(await solid.getByRole('button',{name:process.argv.includes('--behind')?'Surface A':'Front occluder',exact:true}).getAttribute('aria-pressed'),'true')
   await page.screenshot({path:path.join(directory,'occluded.png')})
   await solid.getByRole('button',{name:'Скрыть: Front occluder',exact:true}).click()
  }
  await page.mouse.click(point.x,point.y)
 }else {
  const hit=await solid.locator('[data-surface="surface-a"]').evaluateAll(nodes=>{for(const n of nodes){const points=Array.from(n.points),p=new DOMPoint(points.reduce((s,v)=>s+v.x,0)/points.length,points.reduce((s,v)=>s+v.y,0)/points.length).matrixTransform(n.getScreenCTM());if(document.elementFromPoint(p.x,p.y)===n)return {x:p.x,y:p.y}}return null})
  assert.ok(hit,'A visible surface fragment must accept pointer picking');await page.mouse.click(hit.x,hit.y)
 }
 assert.equal(await solid.getByRole('button',{name:'Surface A',exact:true}).getAttribute('aria-pressed'),'true')
 const detail=solid.locator('label').filter({hasText:/^U segments/}).locator('input')
 await detail.fill('8');await detail.press('Tab');await displayReady()
 assert.ok((await solid.locator('[data-surface=\"surface-a\"]').count())>=128)
 const changed=await exportDoc('detail.json');assert.equal(changed.surfaces[0].segmentsU,8);assert.deepEqual(changed.surfaces[0].surface,before.surfaces[0].surface)
 await solid.getByRole('button',{name:'↶',exact:true}).click();await displayReady();assert.ok((await solid.locator('[data-surface=\"surface-a\"]').count())>=64);assert.deepEqual(await exportDoc('undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();await displayReady();assert.ok((await solid.locator('[data-surface=\"surface-a\"]').count())>=128)
 assert.deepEqual(await exportDoc('redone.json'),changed)
 const requests=await page.evaluate(()=>window.__surfaceRequests);assert.equal(requests,4)
 await page.evaluate(()=>sessionStorage.setItem('surface-no-hold','1'));await page.reload();await displayReady();assert.deepEqual(await exportDoc('reloaded.json'),changed)
 if(process.argv.includes('--preview')){
  if(process.argv.includes('--occlusion'))await solid.getByRole('button',{name:'Показать: Front occluder',exact:true}).click()
  const previewBefore=await exportDoc('preview-before.json')
  await solid.getByRole('button',{name:'Surface A',exact:true}).click()
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Rebuild surface');await search.press('Enter')
  await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click({trial:true})
  await displayReady()
  if(gpu)await solid.locator('canvas.gpu-layer').waitFor({state:'visible'})
  assert.ok(await solid.locator('[data-preview-body]').count()>0)
  if(gpu)assert.ok(await solid.locator('[data-preview-body]').evaluateAll(nodes=>nodes.every(n=>getComputedStyle(n).fill==='rgba(0, 0, 0, 0)'&&getComputedStyle(n).stroke==='none')))
  if(!gpu&&process.argv.includes('--occlusion')){
   const top=await solid.locator('[data-preview-body]').first().evaluate(n=>{
    const point=new DOMPoint(Array.from(n.points).reduce((s,p)=>s+p.x,0)/3,Array.from(n.points).reduce((s,p)=>s+p.y,0)/3)
    const candidates=Array.from(n.ownerSVGElement.querySelectorAll('[data-preview-body],[data-body]')).filter(p=>getComputedStyle(p).display!=='none'&&p.isPointInFill(point))
    const last=candidates.at(-1);return {body:last?.getAttribute('data-body'),preview:last?.getAttribute('data-preview-body')}
   })
   assert.equal(process.argv.includes('--behind')?top.preview:top.body,process.argv.includes('--behind')?'surface-a':'occluder')
  }
  await page.screenshot({path:path.join(directory,gpu?'gpu-preview.png':'cpu-preview.png')})
  await solid.getByRole('button',{name:'Esc',exact:true}).click();assert.deepEqual(await exportDoc('preview-cancelled.json'),previewBefore)
 }
 if(gpu){await solid.locator('canvas.gpu-layer').waitFor({state:'visible'});assert.ok(await solid.locator('[data-surface]').evaluateAll(nodes=>nodes.every(n=>getComputedStyle(n).fill==='rgba(0, 0, 0, 0)'&&getComputedStyle(n).stroke==='none')))}
 await page.screenshot({path:path.join(directory,'surface-display.png')});assert.deepEqual(errors,[])
 const report={gpu,previewChecked:process.argv.includes('--preview'),occlusionChecked:process.argv.includes('--occlusion'),bodyBehind:process.argv.includes('--behind'),browser:browser.version(),workerRequestsBeforeReload:requests,cancelledWorkerTerminated:true,picking:true,initialTriangles:64,refinedTriangles:128,undoRedoAndReload:true,authoredSurfaceUnchanged:true}
 await writeFile(path.join(directory,'surface-display-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
