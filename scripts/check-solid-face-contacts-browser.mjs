import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-face-contacts')
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
  const Native=window.Worker;window.__contactsRequests=0
  window.Worker=class extends Native {
   postMessage(message,...args){
    if(message?.job?.kind==='faceContacts'){
     window.__contactsRequests++
     if(window.__contactsFail){window.__contactsFail=false;queueMicrotask(()=>this.onerror?.({message:'Injected contact failure'}));return}
     if(window.__contactsHold){this.held=true;window.__contactsHeld=true;const callback=this.onmessage;window.__contactsLate=result=>callback?.({data:{version:1,id:message.id,kind:'faceContacts',ok:true,result}});return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.held)window.__contactsTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 let tabs=0
 async function focus(locator){while(!await locator.evaluate(e=>e===document.activeElement)){assert.ok(tabs++<3500,'Keyboard target must be reachable');await page.keyboard.press('Tab')}}
 async function act(locator){if(keyboard){await focus(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function number(locator,value){if(keyboard){await focus(locator);await page.keyboard.press('ControlOrMeta+a');await page.keyboard.insertText(value)}else await locator.fill(value)}
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await act(menu)}
 async function exportDoc(name){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await act(menu);const download=page.waitForEvent('download');await act(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await (await download).saveAs(path.join(directory,name));await closeMenu();return JSON.parse(await readFile(path.join(directory,name),'utf8'))}
 async function openDiagnostics(){if(keyboard)await page.keyboard.press('Control+k');else await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.waitFor();if(keyboard){await focus(search);await page.keyboard.insertText('Диагностика тела')}else await search.fill('Диагностика тела');assert.equal(await search.inputValue(),'Диагностика тела');await search.press('Enter')}
 const fixture=JSON.parse(await readFile('tests/fixtures/face-contact-browser.json','utf8'))
 const cube=JSON.parse(await readFile('tests/fixtures/boundary-agreement.json','utf8')).cases.find(c=>c.name==='normal').document
 fixture.document.bodies.push(structuredClone(cube.bodies[0]))
 assert.equal(new Set(fixture.document.bodies.map(b=>b.id)).size,fixture.document.bodies.length)
 await ready();await act(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'contacts.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture.document))});await closeMenu();await ready()
 await act(solid.getByRole('button',{name:'Contact fixture',exact:true}));const before=await exportDoc('before.json')
 await openDiagnostics();await act(solid.getByRole('button',{name:'Проверить контакты граней',exact:true}))
 const summary=solid.locator('[data-testid="face-contacts-summary"]'),panel=solid.locator('fieldset[aria-label="face-contacts"]')
 const expected=`Контактов: ${fixture.result.contactPairCount} · Общих границ: ${fixture.result.sharedBoundaryPairCount} · Не завершено пар: ${fixture.result.unresolvedPairCount} · Не посещено: ${fixture.result.unvisitedPairs}`
 await summary.filter({hasText:expected}).waitFor({timeout:120000})
 await solid.locator('[data-diagnostic="face-contact"]').waitFor()
 const selector=solid.getByRole('combobox',{name:'Найденный контакт',exact:true})
 if(keyboard){await focus(selector);const label=await selector.locator('option').nth(1).textContent();const cdp=await page.context().newCDPSession(page);for(const letter of label)await cdp.send('Input.dispatchKeyEvent',{type:'char',text:letter,key:letter});await page.keyboard.press('Tab');await cdp.detach()}else await selector.selectOption('1')
 assert.equal(await selector.inputValue(),'1')
 const selected=fixture.result.pairs.filter(p=>p.witness)[1].faces.join(',')
 assert.equal(await solid.locator('[data-diagnostic="face-contact"]').getAttribute('data-faces'),selected)
 const marker=await solid.locator('[data-diagnostic="face-contact"] circle').boundingBox();assert.ok(marker&&marker.width>=4&&marker.width<=20&&marker.height>=4&&marker.height<=20,'Contact marker must stay small on screen')
 await page.screenshot({path:path.join(directory,'contacts.png')})
 const limit=solid.getByRole('spinbutton',{name:'Лимит проверки контактов',exact:true})
 await number(limit,'1');await summary.filter({hasText:'Контактов: 0'}).waitFor();assert.equal(await solid.locator('[data-diagnostic="face-contact"]').count(),0)
 await page.screenshot({path:path.join(directory,'partial.png')})
 await number(limit,'0');await panel.getByRole('alert').filter({hasText:'Введите целый лимит'}).waitFor();assert.equal(await limit.getAttribute('aria-invalid'),'true')
 const requests=await page.evaluate(()=>window.__contactsRequests);await number(limit,'-1');assert.equal(await page.evaluate(()=>window.__contactsRequests),requests)
 await page.evaluate(()=>window.__contactsFail=true);await number(limit,'10000');await panel.getByRole('alert').filter({hasText:'Не удалось проверить контакты'}).waitFor()
 await act(panel.getByRole('button',{name:'Повторить проверку контактов',exact:true}));await summary.filter({hasText:expected}).waitFor({timeout:120000})
 await page.evaluate(()=>window.__contactsHold=true);await act(panel.getByRole('button',{name:'Повторить проверку контактов',exact:true}));await page.waitForFunction(()=>window.__contactsHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__contactsTerminated)
 assert.equal(await solid.locator('[data-diagnostic="face-contact"]').count(),0)
 await page.evaluate(()=>window.__contactsHold=false);await openDiagnostics();await summary.filter({hasText:expected}).waitFor({timeout:120000})
 assert.deepEqual(await exportDoc('after.json'),before)
 // A different selection in the same document must invalidate the held result.
 await page.evaluate(()=>{window.__contactsHold=true;window.__contactsHeld=false;window.__contactsTerminated=false});await act(panel.getByRole('button',{name:'Повторить проверку контактов',exact:true}));await page.waitForFunction(()=>window.__contactsHeld)
 await page.evaluate(()=>window.__contactsHold=false)
 await act(solid.getByRole('tab',{name:'Сцена',exact:true}));await act(solid.getByRole('button',{name:'Boundary fixture',exact:true}))
 await page.waitForFunction(()=>window.__contactsTerminated)
 await page.evaluate(result=>window.__contactsLate(result),fixture.result)
 await openDiagnostics()
 await summary.filter({hasText:'Контактов: 0 · Общих границ: 12 · Не завершено пар: 0 · Не посещено: 0'}).waitFor({timeout:120000})
 assert.equal(await solid.locator('[data-diagnostic="face-contact"]').count(),0)
 await act(solid.getByRole('tab',{name:'Сцена',exact:true}));await act(solid.getByRole('button',{name:'Contact fixture',exact:true}));await openDiagnostics()
 await summary.filter({hasText:expected}).waitFor({timeout:120000})
 assert.deepEqual(await exportDoc('selection-after.json'),before)
 await page.evaluate(()=>{window.__contactsHold=true;window.__contactsHeld=false;window.__contactsTerminated=false});await act(panel.getByRole('button',{name:'Повторить проверку контактов',exact:true}));await page.waitForFunction(()=>window.__contactsHeld)
 await page.evaluate(()=>window.__contactsHold=false)
 await act(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'cube.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(cube))});await closeMenu();await ready()
 await page.waitForFunction(()=>window.__contactsTerminated)
 await page.evaluate(result=>window.__contactsLate(result),fixture.result)
 await act(solid.getByRole('tab',{name:'Сцена',exact:true}));await act(solid.getByRole('button',{name:'Boundary fixture',exact:true}))
 const cubeBefore=await exportDoc('cube-before.json')
 if(!await panel.isVisible()){
  await openDiagnostics()
  if(!await panel.isVisible())await act(solid.getByRole('button',{name:'Проверить контакты граней',exact:true}))
 }
 await summary.filter({hasText:'Контактов: 0 · Общих границ: 12 · Не завершено пар: 0 · Не посещено: 0'}).waitFor({timeout:120000})
 await panel.getByText('Пары граней разнесены либо имеют лишь подтверждённые общие границы.',{exact:true}).waitFor()
 assert.equal(await solid.locator('[data-diagnostic="face-contact"]').count(),0)
 await page.screenshot({path:path.join(directory,'cube-classified.png')})
 assert.deepEqual(await exportDoc('cube-after.json'),cubeBefore)
 if(process.argv.includes('--self-intersection')){
  const within=panel.getByRole('checkbox',{name:'Проверять внутри граней',exact:true})
  async function toggle(){if(keyboard){await focus(within);await page.keyboard.press('Space')}else await within.click()}
  await toggle()
  await panel.getByText('Отсутствие самопересечений подтверждено.',{exact:true}).waitFor({timeout:120000})
  await number(limit,'1')
  await panel.getByText('Отсутствие самопересечений не доказано.',{exact:true}).waitFor()
  await panel.getByText('Не проверены или не доказаны грани: 2, 3, 4, 5, 6',{exact:true}).waitFor()
  assert.deepEqual(await exportDoc('self-partial.json'),cubeBefore)
  await number(limit,'10000')
  await panel.getByText('Отсутствие самопересечений подтверждено.',{exact:true}).waitFor({timeout:120000})
  await page.screenshot({path:path.join(directory,'self-proven.png')})
  assert.deepEqual(await exportDoc('self-after.json'),cubeBefore)
  await toggle()
 }

 const curved=JSON.parse(await readFile('tests/fixtures/curved-shared-boundary-browser.json','utf8'))
 await act(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'curved.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(curved.document))});await closeMenu();await ready()
 await act(solid.getByRole('tab',{name:'Сцена',exact:true}));await act(solid.getByRole('button',{name:'Curved shared boundary',exact:true}))
 const curvedBefore=await exportDoc('curved-before.json')
 await openDiagnostics()
 await summary.filter({hasText:'Контактов: 0 · Общих границ: 12 · Не завершено пар: 0 · Не посещено: 0'}).waitFor({timeout:120000})
 assert.equal(await solid.locator('[data-diagnostic="face-contact"]').count(),0)
 await panel.getByText('Пары граней разнесены либо имеют лишь подтверждённые общие границы.',{exact:true}).waitFor()
 await page.screenshot({path:path.join(directory,'curved-shared.png')})
 assert.deepEqual(await exportDoc('curved-after.json'),curvedBefore)
 assert.deepEqual(errors,[])
 const artifact={geometryWasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex'),indexSha256:createHash('sha256').update(await readFile(path.join(root,'index.html'))).digest('hex')}
 await writeFile(path.join(directory,'result.json'),JSON.stringify({artifact,ok:true,selfIntersection:process.argv.includes('--self-intersection'),keyboard,tabs,contactPairCount:fixture.result.contactPairCount,cubeSharedBoundaries:12,cubeClassified:true,curvedSharedBoundaries:12,curvedUnresolvedPairs:0,partial:true,invalidInput:true,retry:true,cancel:true,restart:true,selectionCancellation:true,modelSwitchCancellation:true,staleReplyAfterImport:true,unchanged:true,errors},null,2))
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
