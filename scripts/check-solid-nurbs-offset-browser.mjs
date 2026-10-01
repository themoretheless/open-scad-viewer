import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-nurbs-offset')
const expectedCurrentCrossings=Number(process.argv.find(argument=>argument.startsWith('--expected-current-crossings='))?.split('=')[1]??4)
assert.ok(Number.isInteger(expectedCurrentCrossings)&&expectedCurrentCrossings>=0)
const tabNavigation=process.argv.includes('--tab-navigation');let tabSteps=0
assert.ok(!tabNavigation||process.argv.includes('--keyboard'),'Tab navigation requires keyboard mode')
const evenOdd=process.argv.includes('--even-odd'),keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
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
let browser,page,captureExports=false
const errors=[],consoleMessages=[]
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage({acceptDownloads:true});page.on('console',m=>consoleMessages.push({type:m.type(),text:m.text()}));page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))})
 await page.addInitScript(()=>{
  const nativeObjectUrl=URL.createObjectURL,nativeAnchorClick=HTMLAnchorElement.prototype.click
  const exports=new Map();window.__captureExport=false;window.__capturedExport=null
  URL.createObjectURL=function(blob){const url=nativeObjectUrl.call(this,blob);if(window.__captureExport)exports.set(url,blob.text());return url}
  HTMLAnchorElement.prototype.click=function(){
   if(window.__captureExport&&this.download==='solid-model.json'&&exports.has(this.href)){
    exports.get(this.href).then(text=>window.__capturedExport=text);exports.delete(this.href);return
   }
   return nativeAnchorClick.call(this)
  }
  const NativeWorker=window.Worker
  window.__offsetFault=false;window.__offsetFaults=0;window.__offsetHold=false;window.__offsetHeld=[];window.__offsetTerminated=0;window.__offsetReleased=0
  window.Worker=class extends NativeWorker {
   set onmessage(handler){this.__handler=handler;super.onmessage=handler?event=>{
    if(window.__offsetFault&&['curveOffset','trimmedCurveOffset'].includes(event.data?.kind)&&event.data.ok===true){window.__offsetFault=false;window.__offsetFaults++;handler({data:{...event.data,version:99}});return}
    if(window.__offsetHold&&['curveOffset','trimmedCurveOffset','curveChainInspection'].includes(event.data?.kind)&&event.data.ok===true){
     this.__held=true;window.__offsetHeld.push(()=>{window.__offsetReleased++;handler(event)});return
    }
    handler(event)
   }:null}
   get onmessage(){return this.__handler}
   terminate(){if(this.__held)window.__offsetTerminated++;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'curve-display',exact:true}).waitFor({state:'hidden'})}
 const crossing=process.argv.includes('--crossing'),trimmed=process.argv.includes('--trimmed'),bevel=process.argv.includes('--bevel')||trimmed
 const fixture={version:1,sketches:[],bodies:[],curves:[{id:'source',name:'Offset source',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,7],[10,10,7],[20,0,7]],weights:[1,.8,1]}}]}
 if(crossing)fixture.curves[0].curve={degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:[[0,0,7],[10,20,7],[-10,20,7],[1,0,7]],weights:[1,1,1,1]}
 if(bevel)fixture.curves[0].curve={degree:1,knots:[0,1,2,3,4,5,6],controlPoints:[[0,0,7],[10,0,7],[10,10,7],[0,10,7],[0,0,7]],weights:[1,1,1,1,1],periodic:true}
 if(trimmed&&crossing)fixture.curves[0].curve={degree:1,knots:[0,0,1,2,3,4,4],controlPoints:[[0,0,7],[4,4,7],[0,4,7],[4,0,7],[0,0,7]],weights:[1,1,1,1,1]}
 await ready();await menu.click()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'offset.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))})
 await solid.getByRole('button',{name:'Offset source',exact:true}).waitFor();await menu.click();await ready()
 async function focusControl(control){
  if(!tabNavigation){await control.focus();return}
  await control.waitFor({state:'visible'});await page.waitForFunction(element=>!element.disabled,await control.elementHandle())
  for(let i=0;i<400;i++){
   if(await control.evaluate(element=>element===document.activeElement))return
   await page.keyboard.press('Tab');tabSteps++
  }
  throw new Error('Control is unreachable by Tab: '+await control.evaluate(element=>element.outerHTML.slice(0,200)))
 }
 async function activate(control){
  if(keyboard){await control.waitFor({state:'visible'});await page.waitForFunction(element=>!element.disabled,await control.elementHandle());await focusControl(control);await control.press('Enter')}
  else await control.click()
 }
 async function historyStep(redo=false){
  if(keyboard){await focusControl(solid.getByRole('button',{name:'Offset source',exact:true}));await page.keyboard.press(redo?'ControlOrMeta+Shift+z':'ControlOrMeta+z')}
  else await solid.getByRole('button',{name:redo?'↷':'↶',exact:true}).click()
 }
 async function exportDoc(name){
  await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)
  let text
  if(captureExports){
   await page.evaluate(()=>{window.__captureExport=true;window.__capturedExport=null})
   await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}))
   await page.waitForFunction(()=>window.__capturedExport!==null)
   text=await page.evaluate(()=>window.__capturedExport)
   await writeFile(path.join(directory,name),text)
  }else{
   const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}))
   const item=await pending;await item.saveAs(path.join(directory,name));text=await readFile(path.join(directory,name),'utf8')
  }
  await activate(menu);return JSON.parse(text)
 }

 const sequence=process.argv.includes('--sequence20')
 let before=await exportDoc('before.json')
 async function command(wait=true){
  const sourceButton=solid.getByRole('button',{name:'Offset source',exact:true})
  if(keyboard){await focusControl(sourceButton);await sourceButton.press('Enter');await page.keyboard.press('Control+k')}
  else {await sourceButton.click();await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()}
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Offset NURBS curve');await search.press('Enter')
  if(bevel){const joins=solid.getByLabel('Соединения',{exact:true});if(keyboard&&trimmed&&evenOdd){await focusControl(joins);await joins.press('End');assert.equal(await joins.inputValue(),'trim-evenodd')}else await joins.selectOption(trimmed?(evenOdd?'trim-evenodd':'trim-nonzero'):'bevel')}
  const distance=solid.getByLabel('Смещение, мм',{exact:true})
  if(trimmed||crossing){const text=crossing?'0.1 mm':'-2 mm';if(keyboard){await focusControl(distance);await distance.press('ControlOrMeta+a');await page.keyboard.insertText(text)}else await distance.fill(text)}
  if(!wait)return
  await solid.getByTestId(trimmed?'trimmed-offset-report':'curve-offset-report').waitFor()
  if(!trimmed)await solid.getByTestId('curve-offset-diagnostics').waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isEnabled(),true);await focusControl(distance)
 }

 await page.evaluate(()=>window.__offsetHold=true)
 await command(false);await page.waitForFunction(()=>window.__offsetHeld.length===1)
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 await page.keyboard.press('Escape');await page.evaluate(()=>{window.__offsetHold=false;window.__offsetHeld.splice(0).forEach(release=>release())})
 assert.deepEqual(await exportDoc('late-cancel.json'),before)
 assert.equal(await solid.locator('[data-preview="curve-offset"]').count(),0)
 await page.evaluate(()=>window.__offsetHold=true)
 await command(false);await page.waitForFunction(()=>window.__offsetHeld.length===1)
 const replacement=structuredClone(fixture);replacement.curves[0].id='replacement';replacement.curves[0].name='Replacement source';replacement.curves[0].curve.controlPoints[1]=[10,5,7]
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'replacement.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(replacement))});await menu.click();await ready()
 const replaced=await exportDoc('replacement-before-late.json')
 await page.evaluate(()=>{window.__offsetHold=false;window.__offsetHeld.splice(0).forEach(release=>release())})
 assert.deepEqual(await exportDoc('replacement-after-late.json'),replaced)
 assert.equal(await solid.locator('[data-preview="curve-offset"]').count(),0)
 const delayed=await page.evaluate(()=>({terminated:window.__offsetTerminated,released:window.__offsetReleased}))
 assert.equal(delayed.terminated,2);assert.equal(delayed.released,2)
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'offset.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await menu.click();await ready()
 before=await exportDoc('reimported-before.json')
 await command();assert.ok(await solid.locator('[data-preview="curve-offset"]').count());if(!trimmed&&(crossing||bevel))assert.ok(await solid.locator('[data-diagnostic="curve-offset-error"]').count());await page.screenshot({path:path.join(directory,'preview.png')});await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('cancel.json'),before)
 await command();await page.keyboard.press('Enter');const applied=await exportDoc('applied.json');assert.ok(applied.curves.length>before.curves.length);assert.deepEqual(applied.curves[0],before.curves[0])
 await historyStep();assert.deepEqual(await exportDoc('undo.json'),before)
 await historyStep(true);assert.deepEqual(await exportDoc('redo.json'),applied)
 await page.reload();await ready();assert.deepEqual(await exportDoc('reload.json'),applied)
 if(process.argv.includes('--construction')){await solid.getByRole('button',{name:applied.curves.at(-1).name,exact:true}).click();const info=solid.getByTestId('offset-construction');await info.waitFor();assert.match(await info.innerText(),/при построении/);assert.match(await info.innerText(),/После правок нужна новая проверка/);await page.screenshot({path:path.join(directory,'construction.png')});if(process.argv.includes('--inspect-current')){await solid.getByRole('button',{name:'Проверить текущую цепочку',exact:true}).click();const report=solid.getByTestId('current-chain-report');await report.waitFor();assert.match(await report.innerText(),new RegExp('пересечения '+expectedCurrentCrossings+' ·'));assert.ok(await solid.locator('[data-diagnostic="curve-offset-error"]').count());await page.screenshot({path:path.join(directory,'current-chain.png')});await page.evaluate(()=>window.__offsetHold=true);await solid.getByRole('button',{name:'Проверить текущую цепочку',exact:true}).click();await page.waitForFunction(()=>window.__offsetHeld.length===1);await page.keyboard.press('Escape');await page.evaluate(()=>{window.__offsetHold=false;window.__offsetHeld.splice(0).forEach(release=>release())});assert.equal(await solid.getByTestId('current-chain-report').count(),0);assert.equal(await solid.locator('[data-diagnostic="curve-offset-error"]').count(),0);await solid.getByRole('button',{name:'Проверить текущую цепочку',exact:true}).click();await report.waitFor();await solid.locator('[data-cv-field="x"]').fill('1');await solid.getByRole('button',{name:'Применить CV',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('[data-testid="current-chain-report"]'));assert.equal(await solid.locator('[data-diagnostic="curve-offset-error"]').count(),0);await solid.getByRole('button',{name:'Offset source',exact:true}).click();assert.equal(await solid.getByTestId('current-chain-report').count(),0);assert.equal(await solid.locator('[data-diagnostic="curve-offset-error"]').count(),0)}}
 let sequenceCount=0
 if(sequence){
  captureExports=true
  const snapshots=[await exportDoc('sequence-start.json')]
  for(let i=1;i<=20;i++){
   await command();await page.keyboard.press('Enter');const next=await exportDoc(`sequence-${i}.json`)
   assert.ok(next.curves.length>snapshots.at(-1).curves.length)
   assert.deepEqual(next.curves.slice(0,snapshots.at(-1).curves.length),snapshots.at(-1).curves)
   assert.equal(new Set(next.curves.map(c=>c.id)).size,next.curves.length)
   snapshots.push(next);sequenceCount++
  }
  await command();await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('sequence-cancel.json'),snapshots.at(-1))
  for(let i=19;i>=0;i--){await historyStep();assert.deepEqual(await exportDoc(`sequence-undo-${i}.json`),snapshots[i])}
  for(let i=1;i<=20;i++){await historyStep(true);assert.deepEqual(await exportDoc(`sequence-redo-${i}.json`),snapshots[i])}
  await page.reload();await ready();assert.deepEqual(await exportDoc('sequence-reload.json'),snapshots.at(-1))
 }
 if(process.argv.includes('--error-feedback')){
  const snapshot=await exportDoc('before-error.json');await command()
  const joins=solid.getByLabel('Соединения',{exact:true});await focusControl(joins);await joins.press('Home')
  await solid.getByRole('alert').filter({hasText:'Offset source: В кривой есть излом. Выберите Bevel или обрезку в поле «Соединения».'}).waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
  await joins.press('End');await solid.getByTestId('trimmed-offset-report').waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isEnabled(),true)
  await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('after-error-recovery.json'),snapshot)
 }
 if(process.argv.includes('--worker-failure')){
  const snapshot=await exportDoc('before-worker-failure.json')
  await page.evaluate(()=>window.__offsetFault=true);await command(false)
  await solid.getByRole('alert').filter({hasText:'Не удалось получить корректный результат вычисления.'}).waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
  await activate(solid.getByRole('button',{name:'Повторить вычисление',exact:true}))
  await solid.getByTestId(trimmed?'trimmed-offset-report':'curve-offset-report').waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isEnabled(),true)
  await page.keyboard.press('Escape')
  assert.equal(await solid.locator('[data-preview="curve-offset"]').count(),0)
  assert.deepEqual(await exportDoc('after-worker-retry-cancel.json'),snapshot)
  assert.equal(await page.evaluate(()=>window.__offsetFaults),1)
 }
 assert.deepEqual(errors,[]);await writeFile(path.join(directory,'contract.json'),JSON.stringify({workerFailureRetry:process.argv.includes('--worker-failure'),keyboard,evenOdd,trimmed,bevel,crossing,sequenceCount,tabNavigation,tabSteps,keyboardHistory:keyboard,keyboardExportActivation:keyboard,keyboardFocusEvidence:tabNavigation?'Controls reached by real Tab presses; import uses fixture setup':'Controls focused by qualification harness; full Tab traversal not qualified',sequenceExportEvidence:'JSON Blob from real export button; no repeated browser downloads',cancel:true,apply:true,undo:true,redo:true,reload:true,lateCancel:true,lateDocumentSwitch:true,delayed,errors},null,2));console.log('Offset browser lifecycle passed')
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''));await writeFile(path.join(directory,'console.json'),JSON.stringify(consoleMessages,null,2))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
