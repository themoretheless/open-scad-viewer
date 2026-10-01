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
  const NativeWorker=window.Worker;window.__diagnosticRequests=0;window.__contactRequests=0
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='brepTool'&&message.job.options.kind==='display'){
     window.__diagnosticRequests++
     if(window.__failDiagnostics){window.__failDiagnostics=false;queueMicrotask(()=>this.onerror?.({message:'Injected worker failure'}));return}
     if(window.__holdDiagnostics){this.__held=true;window.__diagnosticHeld=true;return}
    }
    if(message?.job?.kind==='meshContacts'){window.__contactRequests++;if(window.__failContacts){window.__failContacts=false;queueMicrotask(()=>this.onerror?.({message:'Private contact worker failure'}));return}}
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__diagnosticTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 let tabPresses=0
 async function tabTo(locator){for(let i=0;i<300;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabPresses++}throw Error('Unreachable keyboard control')}
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await tabTo(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu);const download=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await (await download).saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function openDiagnostics(){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,'Диагностика тела');await search.press('Enter')}
 const fixture={version:1,sketches:[],bodies:[{id:'test-cube',name:'Diagnostic cube',mesh:{positions:[0,0,0,10,0,0,10,10,0,0,10,0,0,0,10,10,0,10,10,10,10,0,10,10],indices:[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}}]}
 await ready();await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'diagnostics.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await activate(solid.getByRole('button',{name:'Diagnostic cube',exact:true}));const before=await exportDoc('before.json')
 await page.evaluate(()=>window.__failDiagnostics=true);await openDiagnostics()
 await solid.getByText('Не удалось проверить сетку выбранного тела. Повторите проверку.',{exact:true}).waitFor()
 assert.equal(await solid.locator('[data-testid="diagnostic-body"]').getAttribute('data-body-id'),'test-cube')
 const retry=solid.getByRole('button',{name:'Повторить диагностику',exact:true})
 await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent.trim()==='Повторить диагностику'&&!b.disabled))
 assert.equal(await solid.locator('section[aria-label="Body diagnostics"]').evaluate(e=>e===document.activeElement),true)
 await activate(retry)
 await solid.locator('[data-diagnostic="section"]').waitFor()
 const normalX=solid.getByRole('textbox',{name:'Нормаль X',exact:true}),normalZ=solid.getByRole('textbox',{name:'Нормаль Z',exact:true})
 const requests=await page.evaluate(()=>window.__diagnosticRequests)
 await input(normalX,'bad');await solid.getByText('Сечение не проверено. Исправьте выделенные поля плоскости.',{exact:true}).waitFor()
 assert.equal(await normalX.getAttribute('aria-invalid'),'true');assert.equal(await solid.locator('[data-diagnostic="section"]').count(),0)
 assert.equal(await page.evaluate(()=>window.__diagnosticRequests),requests)
 await input(normalX,'0');await input(normalZ,'0')
 await solid.getByText('Нормаль плоскости нулевая или вне допустимого диапазона. Измените X, Y или Z либо выберите плоскость XY.',{exact:true}).waitFor()
 assert.equal(await solid.getByRole('group',{name:'Нормаль плоскости',exact:true}).getAttribute('aria-invalid'),'true')
 await page.screenshot({path:path.join(directory,'zero-normal.png')})
 await activate(solid.getByRole('button',{name:'Плоскость XY',exact:true}));await solid.locator('[data-diagnostic="section"]').waitFor()
 await page.evaluate(()=>window.__holdDiagnostics=true)
 const offset=solid.getByRole('textbox',{name:'Смещение сечения, мм',exact:true})
 await input(offset,'6 mm');await page.waitForFunction(()=>window.__diagnosticHeld)
 await offset.press('Escape');assert.equal(await page.evaluate(()=>window.__diagnosticTerminated),true)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdDiagnostics=false);await openDiagnostics();await solid.locator('[data-diagnostic="section"]').waitFor()
 await page.evaluate(()=>window.__failContacts=true);await activate(retry)
 await solid.getByRole('alert').filter({hasText:'Самопересечения не проверены.'}).waitFor()
 await page.screenshot({path:path.join(directory,'contact-failure.png')})
 await activate(retry);await solid.getByText('Недопустимых контактов сетки при относительном допуске 1e−9 не найдено.',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 await page.screenshot({path:path.join(directory,'recovered.png')})
 assert.deepEqual(errors,[])
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),before)
 const launcher=solid.getByRole('button',{name:'Команда… Ctrl K',exact:true});await activate(launcher);await page.keyboard.press('Escape');await page.waitForFunction(()=>document.activeElement?.textContent?.includes('Ctrl K'))
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),gpuActive,keyboard,tabPresses,contactFailureRetry:true,paletteCancelRestoresFocus:true,commandFocusPreserved:true,reloadExact:true,workerFailureRetry:true,keyboardRetry:keyboard,invalidInputPreventsRequest:true,zeroNormalLocalized:true,fieldAssociated:true,planeReset:true,escapeTerminatesWorker:true,documentUnchanged:true}
 await writeFile(path.join(directory,'diagnostics-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
