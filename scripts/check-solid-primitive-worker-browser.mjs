import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
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
  const NativeWorker=window.Worker;window.__primitiveRequests=0;window.__holdPrimitive=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='primitive'){
     window.__primitiveRequests++
     if(window.__holdPrimitive){this.__held=true;const callback=this.onmessage;this.onmessage=event=>{if(event.data?.ok===true){window.__primitiveHeld=true;window.__latePrimitive=()=>callback?.call(this,event)}}}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__primitiveTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready();await solid.getByRole('button',{name:'Инструменты',exact:true}).click()
 const before=await exportDoc('before.json')
 await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
 const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
 await page.waitForFunction(el=>el===document.activeElement,await search.elementHandle())
 await page.keyboard.insertText('Box');await page.keyboard.press('Enter')
 await page.waitForFunction(()=>window.__primitiveHeld)
 await page.keyboard.press('Escape');await page.waitForFunction(()=>window.__primitiveTerminated)
 await page.evaluate(()=>window.__latePrimitive())
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdPrimitive=false)
 const names=['Куб','Клин','Цилиндр','Усечённый конус','Труба','Конус','Сфера','Тор']
 for(const name of names){await solid.getByRole('button',{name,exact:true}).click();await ready()}
 const exact=await exportDoc('exact.json');assert.equal(exact.bodies.length,before.bodies.length+8)
 for(const body of exact.bodies.slice(-8)){assert.ok(body.brep);assert.ok(body.mesh.indices.length>0)}
 const box=exact.bodies.at(-8),coordinates=box.mesh.positions
 for(let axis=0;axis<3;axis++){const values=coordinates.filter((_,i)=>i%3===axis);assert.equal(Math.max(...values)-Math.min(...values),20)}
 await solid.locator('.primitive-bar select').selectOption('faceted')
 for(const name of ['Цилиндр','Конус','Сфера']){await solid.getByRole('button',{name,exact:true}).click();await ready()}
 const completed=await exportDoc('all.json');assert.equal(completed.bodies.length,exact.bodies.length+3)
 const [cylinder,cone,sphere]=completed.bodies.slice(-3);assert.ok(cylinder.brep.faces.length>6);assert.equal(cone.brep,undefined);assert.ok(sphere.brep.faces.length>6)
 await solid.getByRole('button',{name:'↶',exact:true}).click();const undone=await exportDoc('undone.json');assert.deepEqual(undone.bodies,completed.bodies.slice(0,-1))
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('redone.json'),completed)
 const requests=await page.evaluate(()=>window.__primitiveRequests);assert.equal(requests,12)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();assert.deepEqual(await exportDoc('reloaded.json'),completed)
 await page.screenshot({path:path.join(directory,'primitives.png')});assert.deepEqual(errors,[])
 const report={browser:browser.version(),workerRequests:requests,cancelledWorkerTerminated:true,lateSuccessfulReplyIgnored:true,paletteEscape:true,exactPrimitives:8,facetedPrimitives:3,boxSizeMm:20,undoRedoAndReload:true}
 await writeFile(path.join(directory,'primitive-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
