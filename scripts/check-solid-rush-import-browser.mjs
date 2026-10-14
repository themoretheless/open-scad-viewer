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
  const NativeWorker=window.Worker;window.__rushGraphRequests=0;window.__holdRushGraph=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='rushGraphImport'){
     window.__rushGraphRequests++
     if(window.__holdRushGraph){this.__held=true;window.__rushGraphHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__rushGraphTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready();await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Box');await search.press('Enter');await ready()
 const before=await exportDoc('before.json'),text=await readFile('tests/fixtures/solid-rush-import.json','utf8')
 async function load(){await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'rush.json',mimeType:'application/json',buffer:Buffer.from(text)});await closeMenu()}
 await load();await page.waitForFunction(()=>window.__rushGraphHeld);await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__rushGraphTerminated)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdRushGraph=false);await load();await ready()
 const imported=await exportDoc('imported.json')
 assert.deepEqual(imported.bodies,before.bodies);assert.deepEqual(imported.sketches,before.sketches)
 assert.equal(imported.curves.length,1);assert.equal(imported.surfaces.length,1)
 assert.equal(imported.curves[0].name,'path');assert.equal(imported.surfaces[0].name,'skin')
 assert.deepEqual(imported.curves[0].curve.weights,[1,Math.SQRT1_2,1])
 const zs=imported.surfaces[0].surface.controlPoints.flat().map(p=>p[2]);assert.equal(Math.min(...zs),0);assert.equal(Math.max(...zs),5)
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('redone.json'),imported)
 const requests=await page.evaluate(()=>window.__rushGraphRequests);assert.equal(requests,2)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();assert.deepEqual(await exportDoc('reloaded.json'),imported)
 await page.screenshot({path:path.join(directory,'rush.png')});assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerRequests:requests,cancelledWorkerTerminated:true,originalScenePreserved:true,rationalWeightsPreserved:true,extrusionHeight:5,undoRedoAndReload:true}
 await writeFile(path.join(directory,'rush-import-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
