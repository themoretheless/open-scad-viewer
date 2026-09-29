import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-shell-distance')
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
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='shellDistance'&&e.data.ok)window.__distanceResults.push(e.data.result)})}
   postMessage(message,...args){if(message?.job?.kind==='shellDistance'){window.__distanceRequests++;if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__distanceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){
  for(let i=0;i<1500;i++){
   if(await locator.evaluate(e=>e===document.activeElement))return
   await page.keyboard.press('Tab');tabs++
  }
  throw new Error('Keyboard target was not reachable by Tab')
 }
 async function activate(locator){if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function choose(locator,value){
  if(!keyboard){await locator.selectOption(value);return}
  const label=await locator.locator('option').evaluateAll((nodes,value)=>nodes.find(n=>n.value===value)?.textContent,value)
  assert.ok(label);await focusByTab(locator)
  await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');await focusByTab(locator)
  // Headless macOS native select popups ignore arrow navigation. Real char
  // events retain native type-ahead, including non-ASCII option labels.
  const cdp=await page.context().newCDPSession(page)
  try {for(const letter of label)await cdp.send('Input.dispatchKeyEvent',{type:'char',text:letter,key:letter})}finally{await cdp.detach()}
  assert.equal(await locator.inputValue(),value)

 }
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu);const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function command(name){await closeMenu();await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await search.press('ControlOrMeta+A');await search.pressSequentially(name)}else await search.fill(name);await search.press('Enter')}
 const fixture=JSON.parse(await readFile('tests/fixtures/face-distance.json','utf8'))
 await ready();await activate(menu)
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'faces.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture.document))})
 await closeMenu();await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
 await activate(solid.getByRole('button',{name:'Plate with hole',exact:true}))
 const before=await exportDoc('before.json')
 await command('Measure vertices / edge')
 await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'probe')
 await activate(solid.getByRole('button',{name:'Расстояние между оболочками',exact:true}))
 const field=solid.getByRole('group',{name:'shell-distance',exact:true})
 await choose(field.getByRole('combobox'),'1000000')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const interval=(await field.locator('[data-shell-distance]').innerText()).replace(' mm','').split(' … ').map(Number)
 const expected=fixture.expectedMm
 assert.ok(interval[0]<=expected&&interval[1]>=expected&&interval[1]-interval[0]<=.001)
 assert.equal(await solid.locator('[data-measurement="shell-distance"] circle').count(),2)
 await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'plate')
 await field.getByRole('alert').filter({hasText:'Выберите другое тело B.'}).waitFor()
 assert.equal(await solid.locator('[data-measurement="shell-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=true)
 await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'probe')
 await page.waitForFunction(()=>window.__distanceHeld)
 await page.evaluate(()=>window.__distanceTerminated=false)
 if(keyboard){await focusByTab(field.getByRole('combobox'));await page.keyboard.press('Escape')}else await field.getByRole('combobox').press('Escape')
 await page.waitForFunction(()=>window.__distanceTerminated)
 assert.equal(await solid.locator('[data-measurement="shell-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=false)
 await command('Measure vertices / edge')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 await field.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,'shell-distance.png')})
 const holeResult=await page.evaluate(()=>window.__distanceResults.at(-1))
 const nested=JSON.parse(await readFile('tests/fixtures/shell-distance.json','utf8'))
 if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'nested.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(nested.document))})
 await closeMenu();await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
 await activate(solid.getByRole('button',{name:'Inner sphere',exact:true}))
 const nestedBefore=await exportDoc('nested-before.json')
 await command('Measure vertices / edge')
 await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'outer')
 const button=solid.getByRole('button',{name:'Расстояние между оболочками',exact:true})
 if(await button.getAttribute('aria-pressed')!=='true')await activate(button)
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const nestedInterval=(await field.locator('[data-shell-distance]').innerText()).replace(' mm','').split(' … ').map(Number)
 assert.ok(nestedInterval[0]<=3&&nestedInterval[1]>=3&&nestedInterval[1]-nestedInterval[0]<=.001)
 await field.getByText('Измерены границы тел. Вложение и пересечение объёмов не классифицированы.',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('nested-after.json'),nestedBefore)
 await field.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,'nested-shell-distance.png')})
 assert.deepEqual(errors,[])
 const artifact={geometryWasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex'),indexSha256:createHash('sha256').update(await readFile(path.join(root,'index.html'))).digest('hex')}
 const report={artifact,browser:browser.version(),keyboard,tabs,expectedMm:expected,displayedIntervalMm:interval,holeResult,nestedInterval,nativeResult:await page.evaluate(()=>window.__distanceResults.at(-1)),requests:await page.evaluate(()=>window.__distanceRequests),cancelledWorkerTerminated:true,invalidTargetLocalized:true,documentUnchanged:true}
 await writeFile(path.join(directory,'shell-distance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
