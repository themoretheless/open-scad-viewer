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
  const NativeWorker=window.Worker;window.__topologyRequests=0;window.__holdTopology=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='faceSketch'&&window.__holdFaceSketch){this.__heldFaceSketch=true;window.__faceSketchHeld=true;return}
    if(message?.job?.kind==='bodyEdges'&&window.__holdEdges){this.__heldEdge=true;window.__edgeHeld=true;return}
    if(message?.job?.kind==='topology'){
     window.__topologyRequests++
     if(window.__holdTopology){this.__held=true;window.__topologyHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__heldFaceSketch)window.__faceSketchTerminated=true;if(this.__heldEdge)window.__edgeTerminated=true;if(this.__held)window.__topologyTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 const fixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-scene-edit/mouse/before.json','utf8'))
 const body={...fixture.bodies[0],id:'base',name:'Base'}
 const secondMesh=structuredClone(body.mesh);secondMesh.positions=secondMesh.positions.map((v,i)=>i%3===0?v+50:v)
 const source={version:1,sketches:[],bodies:[body,{id:'second',name:'Second',mesh:secondMesh}]}
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'body.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(source))});await closeMenu();await ready()
 await solid.getByRole('button',{name:'Base',exact:true}).click()
 await page.waitForFunction(()=>window.__topologyHeld)
 const before=await exportDoc('before.json')
 await solid.getByRole('status',{name:'topology-preparation',exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 assert.equal(await page.evaluate(()=>window.__topologyTerminated),true)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 // Start a second held request and cancel it by selecting another body.
 await solid.getByRole('button',{name:'Second',exact:true}).click()
 await solid.getByRole('status',{name:'topology-preparation',exact:true}).waitFor()
 await page.evaluate(()=>{window.__topologyTerminated=false;window.__holdTopology=false})
 await solid.getByRole('button',{name:'Base',exact:true}).click()
 await solid.getByRole('status',{name:'topology-preparation',exact:true}).waitFor({state:'hidden'})
 assert.equal(await page.evaluate(()=>window.__topologyTerminated),true)
 assert.equal(await solid.getByRole('button',{name:'Base',exact:true}).getAttribute('aria-pressed'),'true')
 assert.equal(await solid.getByRole('button',{name:'Second',exact:true}).getAttribute('aria-pressed'),'false')
 assert.deepEqual(await exportDoc('switched.json'),before)
 await solid.getByRole('button',{name:'Грани',exact:true}).click()
 async function facePoint(unselected=false){
  return solid.locator('[data-body="base"]').evaluateAll((elements,unselected)=>{
   for(const el of elements){
    if(unselected&&el.getAttribute('fill')?.startsWith('hsl(40 '))continue
    const points=el.getAttribute('points').trim().split(' ').map(p=>p.split(',').map(Number))
    const center=new DOMPoint(points.reduce((n,p)=>n+p[0],0)/points.length,points.reduce((n,p)=>n+p[1],0)/points.length).matrixTransform(el.getScreenCTM())
    if(document.elementFromPoint(center.x,center.y)===el)return {x:center.x,y:center.y}
   }
   return null
  },unselected)
 }
 const first=await facePoint();assert.ok(first);await page.mouse.click(first.x,first.y)
 const selected=solid.locator('[data-body="base"][fill^="hsl(40 "]'),count=await selected.count();assert.ok(count>0)
 const another=await facePoint(true);assert.ok(another)
 await page.keyboard.down('Control');await page.mouse.click(another.x,another.y);await page.keyboard.up('Control')
 assert.ok(await selected.count()>count,'Ctrl adds another face')
 await page.keyboard.down('Meta');await page.mouse.click(another.x,another.y);await page.keyboard.up('Meta')
 assert.equal(await selected.count(),count,'Meta removes the selected face')
 assert.deepEqual(await exportDoc('multiple-faces.json'),before)
 await page.screenshot({path:path.join(directory,'multiple-faces.png')})
 await page.mouse.click(first.x,first.y)
 await page.mouse.move(first.x,first.y);await page.mouse.down()
 await solid.locator('.operation-card>strong').filter({hasText:'Сдвиг грани'}).waitFor()
 await page.keyboard.press('Escape');await page.mouse.up()
 assert.deepEqual(await exportDoc('push-cancelled.json'),before)
 await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
 const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
 await search.fill('Push / Pull');await search.press('Enter')
 await solid.getByLabel('Расстояние, мм',{exact:true}).fill('2 mm')
 await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
 const pushed=await exportDoc('pushed.json')
 assert.notDeepEqual(pushed.bodies[0].mesh.positions,before.bodies[0].mesh.positions)
 assert.equal(pushed.bodies[0].id,before.bodies[0].id)
 assert.deepEqual(pushed.bodies[1],before.bodies[1])
 const bounds=mesh=>[0,1,2].map(axis=>{const values=mesh.positions.filter((_,i)=>i%3===axis);return Math.max(...values)-Math.min(...values)})
 const delta=bounds(pushed.bodies[0].mesh).map((v,i)=>v-bounds(before.bodies[0].mesh)[i])
 assert.equal(delta.filter(v=>Math.abs(v-2)<1e-8).length,1)
 assert.equal(delta.filter(v=>Math.abs(v)<1e-8).length,2)
 await solid.getByRole('button',{name:'↶',exact:true}).click()
 assert.deepEqual(await exportDoc('push-undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click()
 assert.deepEqual(await exportDoc('push-redone.json'),pushed)
 await solid.getByRole('button',{name:'↶',exact:true}).click();await ready()


 await solid.getByRole('button',{name:'Second',exact:true}).click()
 await solid.getByRole('button',{name:'Base',exact:true}).click()
 await solid.getByRole('status',{name:'topology-preparation',exact:true}).waitFor({state:'hidden'})
 await solid.getByRole('button',{name:'Инструменты',exact:true}).click()
 await page.evaluate(()=>window.__holdFaceSketch=true)
 await solid.getByRole('button',{name:'На грани',exact:true}).click()
 await solid.locator('[data-body="base"]').first().click({force:true})
 await solid.getByRole('status',{name:'topology-preparation',exact:true}).waitFor({state:'hidden'})
 await page.waitForFunction(()=>window.__faceSketchHeld)
 await solid.getByRole('status',{name:'face-sketch-preparation',exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 assert.equal(await page.evaluate(()=>window.__faceSketchTerminated),true)
 assert.deepEqual(await exportDoc('plane-cancelled.json'),before)
 await page.evaluate(()=>window.__holdFaceSketch=false)
 await solid.getByRole('button',{name:'На грани',exact:true}).click()
 await solid.getByRole('status',{name:'face-sketch-preparation',exact:true}).waitFor({state:'hidden'})
 await solid.getByText('Рисуйте на выделенной грани в 3D или в панели эскиза. Затем нажмите «Выдавить».',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('face-selected.json'),before)
 await solid.getByRole('button',{name:'Base',exact:true}).click()
 await page.evaluate(()=>window.__holdEdges=true)
 await solid.getByRole('button',{name:'Рёбра',exact:true}).click()
 await page.waitForFunction(()=>window.__edgeHeld)
 await solid.getByRole('status',{name:'body-edges',exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 assert.equal(await page.evaluate(()=>window.__edgeTerminated),true)
 assert.equal(await solid.locator('[data-topology-edge]').count(),0)
 await page.evaluate(()=>window.__holdEdges=false)
 await solid.getByRole('button',{name:'Обновить рёбра',exact:true}).click()
 await solid.getByRole('status',{name:'body-edges',exact:true}).waitFor({state:'hidden'})
 assert.equal(await solid.locator('[data-topology-edge]').count(),body.brep.edges.length)
 const edge=solid.locator('[data-topology-edge]').first()
 await edge.focus();await edge.press('Enter');assert.equal(await edge.getAttribute('aria-pressed'),'true')
 assert.deepEqual(await exportDoc('edge-selected.json'),before)
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'face-selected.png')})
 const report={browser:browser.version(),cancelledWorkerTerminated:true,faceSketch:true,bodySwitchCancels:true,ctrlMetaFaceToggle:true,repeatedClickPushCancel:true,push2mmUndoRedo:true,edgeWorkerCancelRetry:true,edgeKeyboardPick:true,faceSketchWorkerCancelRetry:true,documentUnchanged:true,requests:await page.evaluate(()=>window.__topologyRequests)}
 await writeFile(path.join(directory,'topology-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
