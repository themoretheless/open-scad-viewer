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
  const NativeWorker=window.Worker;window.__snapRequests=0;window.__holdSnaps=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='bodySnaps'){
     window.__snapRequests++
     if(window.__holdSnaps){this.__held=true;window.__snapHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__snapTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 const fixture=JSON.parse(await readFile('tests/fixtures/solid-surface-boundary.json','utf8'))
 const stock=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-scene-edit/mouse/before.json','utf8')).bodies[0]
 fixture.bodies=[stock]
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'snaps.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await page.waitForFunction(()=>window.__snapHeld)
 await solid.getByRole('button',{name:'Surface A',exact:true}).click()
 await solid.getByRole('status',{name:'surface-display',exact:true}).waitFor({state:'hidden'})
 const before=await exportDoc('before.json')
 async function drag(){const box=await solid.locator('.nurbs-cage circle').first().boundingBox();assert.ok(box);await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+100,box.y+box.height/2+65,{steps:5});await page.mouse.up();await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))}
 await drag()
 await solid.getByText('Привязки к телам ещё не готовы.',{exact:false}).first().waitFor()
 assert.deepEqual(await exportDoc('blocked.json'),before)
 await page.keyboard.down('Alt');await drag();await page.keyboard.up('Alt')
 assert.notDeepEqual((await exportDoc('alt-bypass.json')).surfaces[0].surface.controlPoints,before.surfaces[0].surface.controlPoints)
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('alt-undone.json'),before)
 await solid.getByRole('button',{name:'Surface A',exact:true}).click()
 // Releasing Alt reinstates required snaps, even when the drag already has a preview.
 for(const releaseAt of ['move','up']){
  const requestsBefore=await page.evaluate(()=>window.__snapRequests)
  const box=await solid.locator('.nurbs-cage circle').first().boundingBox();assert.ok(box)
  const x=box.x+box.width/2,y=box.y+box.height/2
  await page.keyboard.down('Alt');await page.mouse.move(x,y);await page.mouse.down()
  await page.mouse.move(x+100,y+65,{steps:5})
  await page.keyboard.up('Alt')
  if(releaseAt==='move')await page.mouse.move(x+110,y+70)
  await page.mouse.up()
  await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
  assert.deepEqual(await exportDoc(`alt-release-${releaseAt}.json`),before)
  // A cancelled drag must preserve the previous redo branch too.
  assert.equal(await solid.getByRole('button',{name:'↷',exact:true}).isEnabled(),true)
  assert.equal(await page.evaluate(()=>window.__snapRequests),requestsBefore,'Transient preview/cancellation must not restart committed body snap preparation')
  await solid.getByRole('button',{name:'Surface A',exact:true}).click()
 }
 await solid.getByRole('status',{name:'snap-preparation',exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 assert.equal(await page.evaluate(()=>window.__snapTerminated),true)
 await page.evaluate(()=>window.__holdSnaps=false)
 await solid.getByRole('button',{name:'Обновить привязки',exact:true}).click()
 await solid.getByRole('status',{name:'snap-preparation',exact:true}).waitFor({state:'hidden'})
 await drag();const changed=await exportDoc('changed.json')
 assert.notDeepEqual(changed.surfaces[0].surface.controlPoints,before.surfaces[0].surface.controlPoints)
 assert.deepEqual(changed.bodies,before.bodies)
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('redone.json'),changed)
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'snaps-ready.png')})
 const report={browser:browser.version(),pendingGestureBlocked:true,altBypass:true,altReleaseOnMoveCancelled:true,altReleaseOnUpCancelled:true,redoBranchPreserved:true,workerTerminated:true,retry:true,cvEdit:true,undoRedo:true,requests:await page.evaluate(()=>window.__snapRequests)}
 await writeFile(path.join(directory,'snap-preparation-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
