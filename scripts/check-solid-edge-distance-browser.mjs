import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-edge-distance')
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
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='curveDistance'&&e.data.ok)window.__distanceResults.push(e.data.result)})}
   postMessage(message,...args){if(message?.job?.kind==='curveDistance'){window.__distanceRequests++;if(window.__failDistance){window.__failDistance=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:message.job.kind,ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Private distance worker failure'}}}));return}if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__distanceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabPresses=0
 async function activate(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(e=>!e.disabled,await locator.elementHandle());if(keyboard){await tabTo(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function tabTo(locator){for(let i=0;i<300;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabPresses++}throw Error('Unreachable control')}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu);const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function command(name){await closeMenu();await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await input(search,name);await search.press('Enter')}
 await ready();await command('Cylinder');await ready()
 const before=await exportDoc('before.json'),body=before.bodies.at(-1)
 await command('Measure vertices / edge')
 await activate(solid.getByRole('button',{name:'Расстояние между рёбрами',exact:true}))
 const field=solid.getByRole('group',{name:'edge-distance',exact:true})
 const edgeA=body.brep.edges.findIndex(e=>e.curve.degree===2)
 const curveA=body.brep.edges[edgeA].curve
 const edgeB=body.brep.edges.findIndex((e,i)=>i!==edgeA&&e.curve.degree===2&&e.curve.controlPoints.every((p,j)=>p[0]===curveA.controlPoints[j]?.[0]&&p[1]===curveA.controlPoints[j]?.[1]&&p[2]!==curveA.controlPoints[j]?.[2]))
 assert.ok(edgeA>=0&&edgeB>=0,'matching lower and upper cylinder arcs')
 await input(field.getByRole('spinbutton',{name:'Ребро A',exact:true}),String(edgeA+1))
 await input(field.getByRole('spinbutton',{name:'Ребро B',exact:true}),String(edgeB+1))
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const interval=(await field.locator('[data-edge-distance]').innerText()).replace(' mm','').split(' … ').map(Number)
 const expected=Math.abs(body.brep.edges[edgeB].curve.controlPoints[0][2]-curveA.controlPoints[0][2])
 assert.ok(interval[0]<=expected&&interval[1]>=expected&&interval[1]-interval[0]<=.001)
 assert.equal(await solid.locator('[data-measurement="edge-distance"] circle').count(),2)
 await input(field.getByRole('spinbutton',{name:'Ребро B',exact:true}),'9999')
 await field.getByRole('alert').filter({hasText:'Укажите существующее ребро B.'}).waitFor()
 assert.equal(await solid.locator('[data-measurement="edge-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=true)
 await input(field.getByRole('spinbutton',{name:'Ребро B',exact:true}),String(edgeB+1))
 await page.waitForFunction(()=>window.__distanceHeld)
 await field.getByRole('spinbutton',{name:'Ребро B',exact:true}).press('Escape')
 await page.waitForFunction(()=>window.__distanceTerminated)
 assert.equal(await solid.locator('[data-measurement="edge-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=false)
 await command('Measure vertices / edge')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 await page.evaluate(()=>window.__failDistance=true);await activate(solid.getByRole('button',{name:'Расстояние между рёбрами',exact:true}));await activate(solid.getByRole('button',{name:'Расстояние между рёбрами',exact:true}))
 await field.getByRole('alert').waitFor();assert.equal((await field.innerText()).includes('Private distance'),false);assert.equal(await solid.locator('[data-measurement="edge-distance"]').count(),0)
 await activate(field.getByRole('button',{name:'Повторить измерение рёбер',exact:true}));await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 await field.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,'edge-distance.png')})
 assert.deepEqual(errors,[])
 const requests=await page.evaluate(()=>window.__distanceRequests),nativeResult=await page.evaluate(()=>window.__distanceResults.at(-1));assert.ok(requests>=3);assert.ok(nativeResult)
 const gpuActive=await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible');if(process.argv.includes('--require-gpu'))assert.equal(gpuActive,true)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),before);assert.deepEqual(errors,[])
 const report={workerFailureRetry:true,keyboard,tabPresses,gpuActive,reloadExact:true,browser:browser.version(),edgeA,edgeB,expectedMm:expected,displayedIntervalMm:interval,nativeResult,requests,cancelledWorkerTerminated:true,invalidEdgeLocalized:true,documentUnchanged:true}
 await writeFile(path.join(directory,'edge-distance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
