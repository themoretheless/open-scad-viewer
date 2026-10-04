import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-face-distance')
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
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1100}});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))});page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[];window.__distanceTimings=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.__starts=new Map();this.addEventListener('message',e=>{if(['materialChord','materialSegment','materialWall','wholeWall'].includes(e.data?.kind)&&e.data.ok){window.__distanceResults.push(e.data.result);const start=this.__starts.get(e.data.id);if(start!==undefined){window.__distanceTimings.push({workerRoundTripMs:performance.now()-start,cells:e.data.result.cells,domainCells:e.data.result.domainCells,converged:e.data.result.converged});this.__starts.delete(e.data.id)}}})}
   postMessage(message,...args){if(['materialChord','materialSegment','materialWall','wholeWall'].includes(message?.job?.kind)){window.__distanceRequests++;if(window.__failDistance){window.__failDistance=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private distance worker failure'}}}));return}if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}this.__starts.set(message.id,performance.now())}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__distanceTerminated=true;return super.terminate()}
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

 const fixturePath=process.env.CAD_WHOLE_WALL_FIXTURES??'/private/tmp/cad-whole-browser-fixtures.json'
 const fixtures=JSON.parse(await readFile(fixturePath,'utf8'))
 const outcomes=[]
 for(const specimen of fixtures){
  await ready();await activate(menu)
  await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'whole-wall.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(specimen.document))})
  await closeMenu();await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
  await activate(solid.getByRole('button',{name:'Whole wall specimen',exact:true}))
  const before=await exportDoc(specimen.name+'-before.json');await command('Measure vertices / edge')
  const toggle=solid.getByRole('button',{name:'Проверка материала вдоль линии',exact:true});await activate(toggle)
  const field=solid.getByRole('group',{name:'Материал вдоль линии',exact:true})
  await choose(field.getByRole('combobox',{name:'Режим проверки материала',exact:true}),'whole')
  const auto=field.getByRole('checkbox',{name:'Автоматически искать тонкие участки',exact:true})
  async function run(){await activate(field.getByRole('button',{name:/^(Проверить|Повторить проверку)/}))}
  if(specimen.name==='curved-self-unresolved'){
   if(keyboard){await focusByTab(auto);await page.keyboard.press('Space')}else await auto.uncheck()
   assert.equal(await auto.isChecked(),false)
   for(const [values,name] of [[specimen.origin,'Начало '],[specimen.direction,'Смещение ']])for(let i=0;i<3;i++)await input(field.getByRole('spinbutton',{name:name+['X','Y','Z'][i],exact:true}),String(values[i]))
   await run();await field.locator('[data-whole-wall-converged="false"]').waitFor({timeout:120000})
   assert.ok((await field.innerText()).includes('Непроверенные грани'))
   assert.equal(await field.locator('[data-whole-wall-converged="true"]').count(),0)
   await field.getByRole('status').filter({hasText:'Проверяю материал…'}).waitFor({state:'hidden'})
   const manual=await page.evaluate(()=>window.__distanceResults.at(-1))
   assert.deepEqual(manual.candidate.origin,specimen.origin);assert.deepEqual(manual.candidate.direction,specimen.direction)
  }else{
   await run();await field.locator('[data-whole-wall-converged="true"]').waitFor({timeout:120000})
   const interval=(await field.locator('[data-whole-wall-thickness]').innerText()).replace(' mm','').split(' … ').map(Number)
   assert.ok(interval[0]<=specimen.minimum&&interval[1]>=specimen.minimum)
   // Supersede an in-flight whole-wall request by changing its tolerance.
   await page.evaluate(()=>{window.__holdDistance=true;window.__distanceHeld=false;window.__distanceTerminated=false});await run()
   await page.waitForFunction(()=>window.__distanceHeld===true)
   await input(field.getByRole('spinbutton',{name:'Допуск, мм',exact:true}),'0.001')
   await page.waitForFunction(()=>window.__distanceTerminated===true)
   assert.equal(await field.locator('[data-whole-wall-thickness]').count(),0)
   await page.evaluate(()=>{window.__holdDistance=false;window.__failDistance=true});await run()
   await field.getByRole('alert').filter({hasText:'Поиск не выполнен'}).waitFor()
   assert.ok(!(await field.innerText()).includes('Private distance worker failure'))
   await run();await field.locator('[data-whole-wall-converged="true"]').waitFor({timeout:120000})
  }
  await field.getByRole('combobox',{name:'Режим проверки материала',exact:true}).scrollIntoViewIfNeeded()
  await page.screenshot({path:path.join(directory,specimen.name+'.png'),fullPage:true})
  const result=await page.evaluate(()=>window.__distanceResults.at(-1))
  assert.equal(result.method,'bounded-whole-material-wall')
  assert.equal(result.coverage.enumerationComplete,true)
  await page.keyboard.press('Escape');await field.waitFor({state:'hidden'})
  assert.deepEqual(await exportDoc(specimen.name+'-after.json'),before)
  await page.reload();await ready();assert.deepEqual(await exportDoc(specimen.name+'-reload.json'),before)
  outcomes.push({name:specimen.name,result})
 }
 assert.deepEqual(errors,[])
 const report={passed:true,keyboard,tabs,outcomes,errors,cancelledHeldRequest:true,retry:true,unchanged:true,reloadExact:true,
 wasmSha256:createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex')}
 await writeFile(path.join(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
 console.log(JSON.stringify({passed:true,keyboard,cases:outcomes.length}))
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
