import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-mesh-clearance')
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
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='shellDistance'&&e.data.ok)window.__distanceResults.push(e.data.result)})}
   postMessage(message,...args){if(message?.job?.kind==='inspect'){window.__distanceRequests++;if(window.__failDistance){window.__failDistance=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private distance worker failure'}}}));return}if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}}return super.postMessage(message,...args)}
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
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
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
 const mesh={positions:[0,0,0,10,0,0,10,10,0,0,10,0,0,0,10,10,0,10,10,10,10,0,10,10],indices:[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}
 const fixture={version:1,sketches:[],bodies:[{id:'a',name:'Mesh A',mesh},{id:'b',name:'Mesh B',mesh:{...mesh,positions:mesh.positions.map((v,i)=>i%3===0?v+13:v)}}]}
 await ready();await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'mesh.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:'Mesh A',exact:true}));const before=await exportDoc('before.json')
 await command('Measure vertices / edge');await choose(solid.getByRole('combobox',{name:'Тело B',exact:true}),'b');await page.evaluate(()=>window.__failDistance=true);await activate(solid.getByRole('button',{name:'Зазор тел по сетке',exact:true}))
 const alert=solid.getByRole('alert').filter({hasText:'Не удалось вычислить зазор по сетке.'});await alert.waitFor();assert.equal((await solid.innerText()).includes('Private distance'),false);await page.screenshot({path:path.join(directory,'failure-localized.png')})
 await activate(solid.getByRole('button',{name:'Повторить расчёт зазора',exact:true}));await solid.locator('output').filter({hasText:'Зазор: 3.000000 mm'}).waitFor();assert.equal(await solid.locator('[data-measurement="clearance"] circle').count(),2)
 assert.deepEqual(await exportDoc('after.json'),before);await page.screenshot({path:path.join(directory,'recovered.png')})
 const requests=await page.evaluate(()=>window.__distanceRequests);assert.ok(requests>=2)
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),before);assert.deepEqual(errors,[])
 const report={browser:browser.version(),keyboard,tabs,gpuActive,requests,expectedMm:3,localizedFailure:true,retry:true,documentUnchanged:true,reloadExact:true};await writeFile(path.join(directory,'mesh-clearance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
