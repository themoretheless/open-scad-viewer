import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
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
  const NativeWorker=window.Worker;window.__measurementRequests=0;window.__holdMeasurement=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(['measureVertices','measureEdge'].includes(message?.job?.kind)){
     window.__measurementRequests++
     if(window.__holdMeasurement&&message.job.kind==='measureVertices'){this.__held=true;window.__measurementHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__measurementTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function command(name){await closeMenu();await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 await ready();await command('Cylinder');await ready()
 const before=await exportDoc('before.json'),body=before.bodies.at(-1)
 await command('Measure vertices / edge');await page.waitForFunction(()=>window.__measurementHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__measurementTerminated)
 assert.equal(await solid.locator('[data-measurement="distance"]').count(),0)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdMeasurement=false);await command('Measure vertices / edge')
 await solid.getByRole('status',{name:'vertex-measurement',exact:true}).waitFor({state:'hidden'})
 await solid.locator('[data-measurement="distance"]').waitFor()
 const a=body.brep.vertices[0].point,b=body.brep.vertices[1].point,expected=Math.hypot(...a.map((v,i)=>b[i]-v))
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes(expected.toFixed(6)+' mm')))
 await solid.getByRole('button',{name:'Рёбра',exact:true}).click()
 const edge=body.brep.edges.findIndex(e=>e.curve.degree===2)
 await solid.getByRole('combobox',{name:'Выбрать ребро',exact:true}).selectOption(String(edge))
 await solid.getByRole('status',{name:'edge-measurement',exact:true}).waitFor({state:'hidden'})
 await solid.locator('[data-measurement="curvature"]').waitFor()
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('10.000000 mm')))
 await solid.getByRole('spinbutton',{name:'Параметр ребра',exact:true}).fill('0.25')
 await solid.getByRole('status',{name:'edge-measurement',exact:true}).waitFor({state:'hidden'})
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('10.000000 mm')))
 assert.deepEqual(await exportDoc('measured.json'),before)
 const requests=await page.evaluate(()=>window.__measurementRequests);assert.ok(requests>=4)
 await page.screenshot({path:path.join(directory,'measurements.png')});assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerRequests:requests,cancelledWorkerTerminated:true,vertexDistanceMm:expected,edgeRadiusMm:10,documentUnchanged:true}
 await writeFile(path.join(directory,'measurement-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
