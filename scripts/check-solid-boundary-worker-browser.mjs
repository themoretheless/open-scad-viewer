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
  const NativeWorker=window.Worker;window.__boundaryRequests=0;window.__holdBoundary=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='surfaceBoundary'){
     window.__boundaryRequests++
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
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 const fixture=await readFile('tests/fixtures/solid-surface-boundary.json','utf8')
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'surfaces.json',mimeType:'application/json',buffer:Buffer.from(fixture)});await closeMenu();await ready()
 await solid.getByRole('button',{name:'Surface A',exact:true}).click();await solid.getByRole('button',{name:'Surface B',exact:true}).click({modifiers:['Shift']})
 const before=await exportDoc('before.json')
 const inspect=solid.getByRole('button',{name:'Проверить стык поверхностей',exact:true})
 await inspect.click();await page.waitForFunction(()=>window.__boundaryHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__boundaryTerminated)
 assert.equal(await solid.locator('[data-boundary-inspection]').count(),0)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdBoundary=false);await inspect.click()
 await solid.getByRole('status',{name:'surface-boundary-inspection',exact:true}).waitFor({state:'hidden'})
 await solid.locator('[data-boundary-inspection="a"]').waitFor()
 assert.ok((await solid.locator('output').allTextContents()).some(t=>t.includes('Максимальный зазор: 0.250000 mm')))
 assert.equal(await solid.locator('[data-boundary-inspection]').count(),2)
 await solid.getByRole('combobox',{name:'Граница A',exact:true}).selectOption('vMax')
 await solid.getByRole('status',{name:'surface-boundary-inspection',exact:true}).waitFor({state:'hidden'})
 assert.ok(!(await solid.locator('output').allTextContents()).some(t=>t.includes('Максимальный зазор: 0.250000 mm')))
 await solid.getByRole('combobox',{name:'Граница A',exact:true}).selectOption('uMax')
 await solid.getByRole('status',{name:'surface-boundary-inspection',exact:true}).waitFor({state:'hidden'})
 assert.deepEqual(await exportDoc('inspected.json'),before)
 const requests=await page.evaluate(()=>window.__boundaryRequests);assert.equal(requests,4)
 await page.screenshot({path:path.join(directory,'boundary.png')});assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerRequests:requests,cancelledWorkerTerminated:true,maximumGapMm:.25,boundaryChangeChecked:true,documentUnchanged:true}
 await writeFile(path.join(directory,'boundary-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
