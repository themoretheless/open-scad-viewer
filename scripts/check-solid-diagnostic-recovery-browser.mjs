import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-diagnostic-recovery')
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
  const NativeWorker=window.Worker;window.__diagnosticRequests=0
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='brepTool'&&message.job.options.kind==='display'){
     window.__diagnosticRequests++
     if(window.__failDiagnostics){window.__failDiagnostics=false;queueMicrotask(()=>this.onerror?.({message:'Injected worker failure'}));return}
     if(window.__holdDiagnostics){this.__held=true;window.__diagnosticHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__diagnosticTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const download=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await download).saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function openDiagnostics(){await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Диагностика тела');await search.press('Enter')}
 const fixture={version:1,sketches:[],bodies:[{id:'test-cube',name:'Diagnostic cube',mesh:{positions:[0,0,0,10,0,0,10,10,0,0,10,0,0,0,10,10,0,10,10,10,10,0,10,10],indices:[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}}]}
 await ready();await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'diagnostics.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await solid.getByRole('button',{name:'Diagnostic cube',exact:true}).click();const before=await exportDoc('before.json')
 await page.evaluate(()=>window.__failDiagnostics=true);await openDiagnostics()
 await solid.getByText('Не удалось проверить сетку выбранного тела. Повторите проверку.',{exact:true}).waitFor()
 assert.equal(await solid.locator('[data-testid="diagnostic-body"]').getAttribute('data-body-id'),'test-cube')
 const retry=solid.getByRole('button',{name:'Повторить диагностику',exact:true})
 await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent.trim()==='Повторить диагностику'&&!b.disabled))
 assert.equal(await solid.locator('section[aria-label="Body diagnostics"]').evaluate(e=>e===document.activeElement),true)
 let tabs=0
 while(!await retry.evaluate(e=>e===document.activeElement)){
  assert.ok(tabs++<250,'Retry must be reachable by Tab');await page.keyboard.press('Tab')
 }
 await page.keyboard.press('Enter')
 await solid.locator('[data-diagnostic="section"]').waitFor()
 const normalX=solid.getByRole('textbox',{name:'Нормаль X',exact:true}),normalZ=solid.getByRole('textbox',{name:'Нормаль Z',exact:true})
 const requests=await page.evaluate(()=>window.__diagnosticRequests)
 await normalX.fill('bad');await solid.getByText('Сечение не проверено. Исправьте выделенные поля плоскости.',{exact:true}).waitFor()
 assert.equal(await normalX.getAttribute('aria-invalid'),'true');assert.equal(await solid.locator('[data-diagnostic="section"]').count(),0)
 assert.equal(await page.evaluate(()=>window.__diagnosticRequests),requests)
 await normalX.fill('0');await normalZ.fill('0')
 await solid.getByText('Нормаль плоскости нулевая или вне допустимого диапазона. Измените X, Y или Z либо выберите плоскость XY.',{exact:true}).waitFor()
 assert.equal(await solid.getByRole('group',{name:'Нормаль плоскости',exact:true}).getAttribute('aria-invalid'),'true')
 await page.screenshot({path:path.join(directory,'zero-normal.png')})
 await solid.getByRole('button',{name:'Плоскость XY',exact:true}).click();await solid.locator('[data-diagnostic="section"]').waitFor()
 await page.evaluate(()=>window.__holdDiagnostics=true)
 const offset=solid.getByRole('textbox',{name:'Смещение сечения, мм',exact:true})
 await offset.fill('6 mm');await page.waitForFunction(()=>window.__diagnosticHeld)
 await offset.press('Escape');assert.equal(await page.evaluate(()=>window.__diagnosticTerminated),true)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdDiagnostics=false);await openDiagnostics();await solid.locator('[data-diagnostic="section"]').waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 await page.screenshot({path:path.join(directory,'recovered.png')})
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerFailureRetry:true,keyboardRetry:true,tabs,invalidInputPreventsRequest:true,zeroNormalLocalized:true,fieldAssociated:true,planeReset:true,escapeTerminatesWorker:true,documentUnchanged:true}
 await writeFile(path.join(directory,'diagnostics-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
