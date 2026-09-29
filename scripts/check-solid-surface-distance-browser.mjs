import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-surface-distance')
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
  const NativeWorker=window.Worker;window.__surfaceDistanceRequests=0;window.__holdSurfaceDistance=true;window.__surfaceDistanceResults=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='surfaceDistance'&&e.data.ok)window.__surfaceDistanceResults.push(e.data.result)})}
   postMessage(message,...args){if(message?.job?.kind==='surfaceDistance'){window.__surfaceDistanceRequests++;if(window.__holdSurfaceDistance){this.__held=true;window.__surfaceDistanceHeld=true;return}}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__surfaceDistanceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function importSurfaces(fixture){
  await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click()
  await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'surfaces.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))})
  await closeMenu();await ready();await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
  await solid.getByRole('button',{name:'Surface A',exact:true}).click();await solid.getByRole('button',{name:'Surface B',exact:true}).click({modifiers:['Shift']})
 }
 const fixture=JSON.parse(await readFile('tests/fixtures/solid-surface-boundary.json','utf8'))
 await importSurfaces(fixture)
 const before=await exportDoc('before.json')
 const inspect=solid.getByRole('button',{name:'Расстояние между поверхностями',exact:true}),panel=solid.getByRole('region',{name:'surface-distance',exact:true})
 await inspect.click();await page.waitForFunction(()=>window.__surfaceDistanceHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__surfaceDistanceTerminated)
 assert.equal(await solid.locator('[data-measurement="surface-distance"]').count(),0)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdSurfaceDistance=false)
 await inspect.focus();await page.keyboard.press('Enter')
 await panel.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const planar=await page.evaluate(()=>window.__surfaceDistanceResults.at(-1))
 assert.ok(planar.distanceIntervalMm[0]<=.25&&planar.distanceIntervalMm[1]>=.25)
 assert.equal(await solid.locator('[data-measurement="surface-distance"] circle').count(),2)
 assert.deepEqual(await exportDoc('planar-after.json'),before)
 const q=[.37,.62].map(x=>[x*x,x*x-x,(1-x)*(1-x)])
 const surface={degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i/2,j/2,q[0][i]+q[1][j]])),weights:[[1,1,1],[1,1,1],[1,1,1]]}
 const other=structuredClone(surface);other.controlPoints.flat().forEach(p=>p[2]=-p[2]-2)
 const curved={version:1,sketches:[],bodies:[],surfaces:[{id:'a',name:'Surface A',surface,segmentsU:8,segmentsV:8},{id:'b',name:'Surface B',surface:other,segmentsU:8,segmentsV:8}]}
 await importSurfaces(curved)
 const curvedBefore=await exportDoc('curved-before.json')
 if(await inspect.getAttribute('aria-pressed')!=='true')await inspect.click()
 await panel.getByRole('combobox',{name:'Объём расчёта поверхностей',exact:true}).selectOption('100000')
 await panel.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const curvedResult=await page.evaluate(()=>window.__surfaceDistanceResults.at(-1))
 assert.ok(curvedResult.distanceIntervalMm[0]<=2&&curvedResult.distanceIntervalMm[1]>=2&&curvedResult.distanceIntervalMm[1]-curvedResult.distanceIntervalMm[0]<=.001)
 assert.ok(curvedResult.parameters.flat().every(t=>t>0&&t<1))
 assert.deepEqual(await exportDoc('curved-after.json'),curvedBefore)
 await panel.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,'surface-distance.png')})
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerRequests:await page.evaluate(()=>window.__surfaceDistanceRequests),cancelledWorkerTerminated:true,keyboardRestart:true,planar,curved:curvedResult,documentUnchanged:true}
 await writeFile(path.join(directory,'surface-distance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
