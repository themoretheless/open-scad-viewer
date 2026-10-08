import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-face-distance')
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
 browser=await playwright.chromium.launch({headless:true,channel:'chromium',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1100}});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))});page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})
 // Exercise CAD behavior without depending on external font DNS. Keep all
 // application console/page errors fatal; only this stylesheet is substituted.
 await page.route('https://fonts.googleapis.com/**',route=>route.fulfill({status:200,contentType:'text/css',body:''}))
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__sourceResults=[];window.__holdSource=false;
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='sourceBodyRestore'&&e.data.ok&&e.data.result?.wallCoverage){window.__sourceResults.push(e.data.result.wallCoverage);window.__sourceReply=e.data.result}})}
   postMessage(message,...args){if(message?.job?.kind==='sourceBodyRestore'&&message.job.options.wallCoverage){
    if(window.__failSource){window.__failSource=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:'sourceBodyRestore',ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private source wall worker failure'}}}));return}
    if(window.__holdSource){window.__sourceHeld=true;this.held=true;const callback=this.onmessage,result=structuredClone(window.__sourceReply);window.__sourceLate=()=>{if(!callback||!result)throw Error('Missing obsolete source callback or result');callback({data:{version:1,id:message.id,kind:'sourceBodyRestore',ok:true,result}})};return}
   }return super.postMessage(message,...args)}
   terminate(){if(this.held)window.__sourceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){for(let i=0;i<300;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabs++}throw Error('Unreachable keyboard control')}
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function input(locator,value){if(keyboard){await focusByTab(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
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
 async function command(name){await closeMenu();await activate(page.getByRole('button',{name:'Команды',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,name);await search.press('Enter')}

 const fixture=process.env.CAD_SOURCE_WALL_UI_FIXTURE;assert.ok(fixture,'Set CAD_SOURCE_WALL_UI_FIXTURE')
 await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles(fixture);await closeMenu();await ready()
 await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:'Annular source wall',exact:true}))
 const before=await exportDoc('before.json');await activate(menu)
 const panel=solid.locator('.source-wall-panel');await activate(panel.locator('summary').first())
 await input(panel.getByLabel('Minimum source wall thickness, mm',{exact:true}),'5.99')
 const run=panel.getByRole('button',{name:'Проверить всю стенку',exact:true})
 await activate(run);const result=panel.locator('[data-source-wall-coverage-result]');await result.waitFor({timeout:120000})
 const coverage=await page.evaluate(()=>window.__sourceResults.at(-1));assert.equal(coverage.wholeWallQualified,false);assert.equal(coverage.totalPairs,378)
 assert.equal(coverage.lowerMm,null)
 assert.deepEqual(coverage.request.normalProducts,{cells:50000,cellsPerPair:128})
 assert.ok(coverage.normalProductCells>0&&coverage.normalProductCells<=50000)
 assert.ok(coverage.pairs.filter(p=>!p.proven).length<=173)
 assert.ok(coverage.adaptiveSelf.faces.every(f=>f.endpointExchangeSymmetric===true&&f.unresolvedPairs.length===f.pending))
 await activate(result.locator('summary').first())
 assert.ok(!(await result.innerText()).includes('source-wall-coverage-'))
 assert.ok((await result.innerText()).includes('Все пары рассмотрены'))
 assert.equal(await panel.locator('[data-source-wall-pending-face]').count(),coverage.adaptiveSelf.faces.filter(f=>!f.qualified).length)
 assert.ok(await solid.locator('[data-source-wall-error="true"]').count()>0)
 await page.screenshot({path:path.join(directory,'unresolved.png'),fullPage:true})
 await page.evaluate(()=>{window.__holdSource=true;window.__sourceHeld=false;window.__sourceTerminated=false})
 await activate(run);await page.waitForFunction(()=>window.__sourceHeld)
 await input(panel.getByLabel('Minimum source wall thickness, mm',{exact:true}),'6.01');await page.waitForFunction(()=>window.__sourceTerminated)
 await page.evaluate(()=>window.__sourceLate());assert.equal(await result.count(),0);assert.equal(await panel.locator('#source-wall-input-error').count(),0)
 await page.evaluate(()=>{window.__holdSource=false;window.__failSource=true});await activate(run)
 const failure=panel.locator('#source-wall-input-error');await failure.waitFor();assert.ok((await failure.innerText()).includes('Расчёт не завершён'))
 await activate(failure.getByRole('button',{name:'Повторить',exact:true}));await result.waitFor({timeout:120000});assert.equal(await failure.count(),0)
 assert.equal((await page.evaluate(()=>window.__sourceResults.at(-1))).wholeWallQualified,false)
 await closeMenu();assert.deepEqual(await exportDoc('after.json'),before);await page.reload();await ready();assert.deepEqual(await exportDoc('reload.json'),before)
 assert.deepEqual(errors,[])
 await writeFile(path.join(directory,'report.json'),JSON.stringify({passed:true,keyboard,tabs,externalFontStylesheetSubstituted:true,coverage,cancelledHeldRequest:true,obsoleteCallbackIgnored:true,localizedTransportFailure:true,retry:true,highlightedUnprovenFaces:true,documentUnchanged:true,reloadExact:true,errors,wasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex')},null,2)+'\n')
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
