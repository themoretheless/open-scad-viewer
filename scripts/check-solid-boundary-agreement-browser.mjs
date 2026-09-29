import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-boundary-agreement')
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
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>errors.push(String(e)))
 await page.addInitScript(()=>{
  const Native=window.Worker;window.__boundaryRequests=0
  window.Worker=class extends Native {
   postMessage(message,...args){
    if(message?.job?.kind==='boundaryAgreement'){
     window.__boundaryRequests++
     if(window.__boundaryFail){window.__boundaryFail=false;queueMicrotask(()=>this.onerror?.({message:'Injected boundary failure'}));return}
     if(window.__boundaryHold){this.held=true;window.__boundaryHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.held)window.__boundaryTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 let tabs=0
 async function focus(locator){while(!await locator.evaluate(e=>e===document.activeElement)){assert.ok(tabs++<2500,'Keyboard target must be reachable');await page.keyboard.press('Tab')}}
 async function act(locator){if(keyboard){await focus(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function number(locator,value){if(keyboard){await focus(locator);await page.keyboard.press('ControlOrMeta+a');await page.keyboard.insertText(value)}else await locator.fill(value)}
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await act(menu)}
 async function exportDoc(name){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await act(menu);const download=page.waitForEvent('download');await act(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await (await download).saveAs(path.join(directory,name));await closeMenu();return JSON.parse(await readFile(path.join(directory,name),'utf8'))}
 async function openDiagnostics(){if(keyboard)await page.keyboard.press('Control+k');else await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.waitFor();if(keyboard){await focus(search);await page.keyboard.insertText('Диагностика тела')}else await search.fill('Диагностика тела');assert.equal(await search.inputValue(),'Диагностика тела');await search.press('Enter')}
 const fixture=JSON.parse(await readFile('tests/fixtures/boundary-agreement.json','utf8')).cases.find(c=>c.name==='defect')
 await ready();await act(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'boundary.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture.document))});await closeMenu();await ready()
 await act(solid.getByRole('button',{name:'Boundary fixture',exact:true}));const before=await exportDoc('before.json')
 await openDiagnostics();const toggle=solid.getByRole('button',{name:'Проверить границы B-rep',exact:true});await act(toggle)
 const summary=solid.locator('[data-testid="boundary-summary"]'),panel=solid.locator('fieldset[aria-label="boundary-agreement"]')
 await summary.filter({hasText:'Расхождения: 2 · Не проверено: 0'}).waitFor()
 await solid.locator('[data-diagnostic="boundary-agreement"][data-edge="0"]').waitFor()
 const selector=solid.getByRole('combobox',{name:'Расхождение границы',exact:true})
 if(keyboard){await focus(selector);const label=await selector.locator('option').nth(1).textContent();const cdp=await page.context().newCDPSession(page);for(const letter of label)await cdp.send('Input.dispatchKeyEvent',{type:'char',text:letter,key:letter});await page.keyboard.press('Tab');await cdp.detach()}else await selector.selectOption('1')
 assert.equal(await selector.inputValue(),'1')
 await page.screenshot({path:path.join(directory,'defect.png')})
 const limit=solid.getByRole('spinbutton',{name:'Лимит проверки границ',exact:true})
 await number(limit,'1');await summary.filter({hasText:`Не проверено: ${fixture.partial.unresolvedCount}`}).waitFor()
 assert.equal(await solid.locator('[data-diagnostic="boundary-agreement"]').count(),0)
 await page.screenshot({path:path.join(directory,'partial.png')})
 await number(limit,'0');await panel.getByRole('alert').filter({hasText:'Введите целый лимит'}).waitFor();assert.equal(await limit.getAttribute('aria-invalid'),'true')
 const requests=await page.evaluate(()=>window.__boundaryRequests);await number(limit,'-1');assert.equal(await page.evaluate(()=>window.__boundaryRequests),requests)
 await page.evaluate(()=>window.__boundaryFail=true);await number(limit,'10000');await panel.getByRole('alert').filter({hasText:'Не удалось проверить границы'}).waitFor()
 await act(panel.getByRole('button',{name:'Повторить проверку границ',exact:true}));await summary.filter({hasText:'Расхождения: 2 · Не проверено: 0'}).waitFor()
 await page.evaluate(()=>window.__boundaryHold=true);await act(panel.getByRole('button',{name:'Повторить проверку границ',exact:true}));await page.waitForFunction(()=>window.__boundaryHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__boundaryTerminated)
 assert.equal(await solid.locator('[data-diagnostic="boundary-agreement"]').count(),0)
 await page.evaluate(()=>window.__boundaryHold=false);await openDiagnostics();await summary.filter({hasText:'Расхождения: 2 · Не проверено: 0'}).waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 assert.deepEqual(errors,[])
 const artifact={geometryWasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex'),indexSha256:createHash('sha256').update(await readFile(path.join(root,'index.html'))).digest('hex')}
 await writeFile(path.join(directory,'result.json'),JSON.stringify({artifact,ok:true,keyboard,tabs,mismatchCount:2,partial:true,invalidInput:true,retry:true,cancel:true,restart:true,unchanged:true,errors},null,2))
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
