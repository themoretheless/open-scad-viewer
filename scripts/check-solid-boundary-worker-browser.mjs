import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-boundary-inspection')
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
  const NativeWorker=window.Worker;window.__boundaryRequests=0;window.__holdBoundary=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='surfaceBoundary'){
     window.__boundaryRequests++;if(window.__failBoundary){window.__failBoundary=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private boundary failure'}}}));return}
     if(window.__holdBoundary){this.__held=true;window.__boundaryHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__boundaryTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){for(let i=0;i<300;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabs++}throw Error('Unreachable keyboard control')}
 async function activate(locator,shift=false){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await focusByTab(locator);await page.keyboard.press(shift?'Shift+Enter':'Enter')}else await locator.click(shift?{modifiers:['Shift']}:undefined)}
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
 await ready()
 const fixture=await readFile('tests/fixtures/solid-surface-boundary.json','utf8')
 await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'surfaces.json',mimeType:'application/json',buffer:Buffer.from(fixture)});await closeMenu();await ready()
 await activate(solid.getByRole('button',{name:'Surface A',exact:true}));await activate(solid.getByRole('button',{name:'Surface B',exact:true}),true)
 const before=await exportDoc('before.json')
 const inspect=solid.getByRole('button',{name:'Проверить стык поверхностей',exact:true})
 await activate(inspect);await page.waitForFunction(()=>window.__boundaryHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__boundaryTerminated)
 assert.equal(await solid.locator('[data-boundary-inspection]').count(),0)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdBoundary=false);await activate(inspect)
 await solid.getByRole('status',{name:'surface-boundary-inspection',exact:true}).waitFor({state:'hidden'})
 await solid.locator('[data-boundary-inspection="a"]').waitFor()
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('Максимальный зазор: 0.250000 mm')))
 assert.equal(await solid.locator('[data-boundary-inspection]').count(),2)
 await choose(solid.getByRole('combobox',{name:'Граница A',exact:true}),'vMax')
 await solid.getByRole('status',{name:'surface-boundary-inspection',exact:true}).waitFor({state:'hidden'})
 assert.ok(!(await solid.locator('output').allTextContents()).some(t=>t.includes('Максимальный зазор: 0.250000 mm')))
 await choose(solid.getByRole('combobox',{name:'Граница A',exact:true}),'uMax')
 await solid.getByRole('status',{name:'surface-boundary-inspection',exact:true}).waitFor({state:'hidden'})
 await page.evaluate(()=>window.__failBoundary=true);await activate(inspect);await activate(inspect);await solid.getByRole('alert').filter({hasText:'Не удалось проверить стык поверхностей.'}).waitFor();assert.equal((await solid.innerText()).includes('Private boundary'),false);await activate(solid.getByRole('button',{name:'Повторить проверку стыка',exact:true}));await solid.locator('[data-boundary-inspection="a"]').waitFor()
 const sampleField=solid.getByRole('spinbutton',{name:'Точек проверки',exact:true}),requestsBefore=await page.evaluate(()=>window.__boundaryRequests)
 await input(sampleField,'1');await solid.getByRole('alert').filter({hasText:'Исправьте выделенные поля:'}).waitFor();assert.equal(await sampleField.getAttribute('aria-invalid'),'true');assert.equal(await sampleField.getAttribute('aria-describedby'),'surface-boundary-error');assert.equal(await page.evaluate(()=>window.__boundaryRequests),requestsBefore);assert.equal(await solid.locator('[data-boundary-inspection]').count(),0);await page.screenshot({path:path.join(directory,'invalid-samples.png')});await input(sampleField,'65');await solid.locator('[data-boundary-inspection="a"]').waitFor()
 assert.deepEqual(await exportDoc('inspected.json'),before)
 const requests=await page.evaluate(()=>window.__boundaryRequests);assert.ok(requests>=7)
 await page.screenshot({path:path.join(directory,'boundary.png')});assert.deepEqual(errors,[])
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),before);assert.deepEqual(errors,[])
 const report={keyboard,tabs,gpuActive,reloadExact:true,workerFailureRetry:true,invalidSamplePreventsDispatch:true,browser:browser.version(),workerRequests:requests,cancelledWorkerTerminated:true,maximumGapMm:.25,boundaryChangeChecked:true,documentUnchanged:true}
 await writeFile(path.join(directory,'boundary-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
