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
 const fixture=await readFile('tests/fixtures/solid-surface-boundary.json','utf8')
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'surfaces.json',mimeType:'application/json',buffer:Buffer.from(fixture)});await closeMenu();await ready()
 await page.waitForFunction(()=>window.__surfaceHeld)
 const before=await exportDoc('before.json')
 assert.equal(await solid.locator('[data-surface]').count(),0)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__surfaceTerminated)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdSurface=false)
 await solid.getByRole('button',{name:'Обновить поверхности',exact:true}).click()
 const displayReady=async()=>{await ready();await solid.getByRole('status',{name:'surface-display',exact:true}).waitFor({state:'hidden'})}
 await displayReady();assert.equal(await solid.locator('[data-surface="surface-a"]').count(),64);assert.equal(await solid.locator('[data-surface="surface-b"]').count(),64)
 await solid.locator('[data-surface="surface-a"]').first().click()
 assert.equal(await solid.getByRole('button',{name:'Surface A',exact:true}).getAttribute('aria-pressed'),'true')
 const detail=solid.locator('label').filter({hasText:/^U segments/}).locator('input')
 await detail.fill('8');await detail.press('Tab');await displayReady()
 assert.equal(await solid.locator('[data-surface="surface-a"]').count(),128)
 const changed=await exportDoc('detail.json');assert.equal(changed.surfaces[0].segmentsU,8);assert.deepEqual(changed.surfaces[0].surface,before.surfaces[0].surface)
 await solid.getByRole('button',{name:'↶',exact:true}).click();await displayReady();assert.equal(await solid.locator('[data-surface="surface-a"]').count(),64);assert.deepEqual(await exportDoc('undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();await displayReady();assert.equal(await solid.locator('[data-surface="surface-a"]').count(),128)
 assert.deepEqual(await exportDoc('redone.json'),changed)
 const requests=await page.evaluate(()=>window.__surfaceRequests);assert.equal(requests,4)
 await page.evaluate(()=>sessionStorage.setItem('surface-no-hold','1'));await page.reload();await displayReady();assert.deepEqual(await exportDoc('reloaded.json'),changed)
 await page.screenshot({path:path.join(directory,'surface-display.png')});assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerRequestsBeforeReload:requests,cancelledWorkerTerminated:true,picking:true,initialTriangles:64,refinedTriangles:128,undoRedoAndReload:true,authoredSurfaceUnchanged:true}
 await writeFile(path.join(directory,'surface-display-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
