import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-sketch-snaps')
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
  const NativeWorker=window.Worker;window.__snapRequests=0;window.__holdSnaps=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='sketchSnaps'){
     window.__snapRequests++
     if(window.__holdSnaps){this.__held=true;window.__snapHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__snapTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 const fixture={version:1,bodies:[],sketches:[{id:'target',name:'Target',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}]}
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'sketch-snaps.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await page.waitForFunction(()=>window.__snapHeld)
 const cell=solid.getByRole('spinbutton',{name:'Размер клетки',exact:true}).first();await cell.fill('1');await cell.press('Tab')
 const before=await exportDoc('before.json'),canvas=solid.locator('svg[aria-label="Холст эскизов 2D"]')
 async function rectangle(){await solid.getByRole('button',{name:'Прямоугольник · R',exact:true}).click()}
 async function drag(releaseAlt=false){
  const box=await canvas.boundingBox();assert.ok(box)
  await page.mouse.move(box.x+box.width*.65,box.y+box.height*.3);await page.mouse.down()
  await page.mouse.move(box.x+box.width*.85,box.y+box.height*.15,{steps:5})
  if(releaseAlt)await page.keyboard.up('Alt')
  await page.mouse.up()
 }
 await rectangle();await drag();assert.deepEqual(await exportDoc('blocked.json'),before)
 await solid.getByText('Привязки эскизов ещё не готовы.',{exact:false}).first().waitFor()
 await page.keyboard.down('Alt');await drag(true);assert.deepEqual(await exportDoc('alt-cancelled.json'),before)
 await solid.getByRole('status',{name:'sketch-snap-preparation',exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 assert.equal(await page.evaluate(()=>window.__snapTerminated),true)
 await page.evaluate(()=>window.__holdSnaps=false)
 await solid.getByRole('button',{name:'Обновить привязки эскизов',exact:true}).click()
 await solid.getByRole('status',{name:'sketch-snap-preparation',exact:true}).waitFor({state:'hidden'})
 await rectangle();await drag();const changed=await exportDoc('changed.json')
 assert.equal(changed.sketches.length,2);assert.deepEqual(changed.sketches[0],before.sketches[0])
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('redone.json'),changed)
 await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),changed)
 await solid.getByRole('status',{name:'sketch-snap-preparation',exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 await page.evaluate(()=>window.__holdSnaps=false)
 await solid.getByRole('button',{name:'Обновить привязки эскизов',exact:true}).click()
 await solid.getByRole('status',{name:'sketch-snap-preparation',exact:true}).waitFor({state:'hidden'})
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'sketch-snaps.png')})
 const report={browser:browser.version(),pendingDrawingBlocked:true,altReleaseCancelled:true,workerTerminated:true,retry:true,drawing:true,undoRedo:true,reload:true}
 await writeFile(path.join(directory,'sketch-snap-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
