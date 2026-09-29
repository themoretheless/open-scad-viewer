import assert from 'node:assert/strict'
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
  const NativeWorker=window.Worker;window.__holdDistance=false;window.__distanceRequests=0;window.__distanceResults=[]
  window.Worker=class extends NativeWorker {
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='faceDistance'&&e.data.ok)window.__distanceResults.push(e.data.result)})}
   postMessage(message,...args){if(message?.job?.kind==='faceDistance'){window.__distanceRequests++;if(window.__holdDistance){this.__held=true;window.__distanceHeld=true;return}}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__distanceTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function command(name){await closeMenu();await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 const fixture=JSON.parse(await readFile('tests/fixtures/face-distance.json','utf8'))
 await ready();await menu.click()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'faces.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture.document))})
 await closeMenu();await ready();await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
 await solid.getByRole('button',{name:'Plate with hole',exact:true}).click()
 const before=await exportDoc('before.json')
 await command('Measure vertices / edge')
 await solid.getByRole('combobox',{name:'Тело B',exact:true}).selectOption('probe')
 await solid.getByRole('button',{name:'Расстояние между гранями',exact:true}).click()
 const field=solid.getByRole('group',{name:'face-distance',exact:true})
 const edgeA=fixture.faceA,edgeB=fixture.faceB
 await field.getByRole('spinbutton',{name:'Грань A',exact:true}).fill(String(edgeA+1))
 await field.getByRole('spinbutton',{name:'Грань B',exact:true}).fill(String(edgeB+1))
 await field.getByRole('combobox').selectOption('100000')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 const interval=(await field.locator('[data-face-distance]').innerText()).replace(' mm','').split(' … ').map(Number)
 const expected=fixture.expectedMm
 assert.ok(interval[0]<=expected&&interval[1]>=expected&&interval[1]-interval[0]<=.001)
 assert.equal(await solid.locator('[data-measurement="face-distance"] circle').count(),2)
 await field.getByRole('spinbutton',{name:'Грань B',exact:true}).fill('9999')
 await field.getByRole('alert').filter({hasText:'Укажите существующую грань B.'}).waitFor()
 assert.equal(await solid.locator('[data-measurement="face-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=true)
 await field.getByRole('spinbutton',{name:'Грань B',exact:true}).fill(String(edgeB+1))
 await page.waitForFunction(()=>window.__distanceHeld)
 await field.getByRole('spinbutton',{name:'Грань B',exact:true}).press('Escape')
 await page.waitForFunction(()=>window.__distanceTerminated)
 assert.equal(await solid.locator('[data-measurement="face-distance"]').count(),0)
 await page.evaluate(()=>window.__holdDistance=false)
 await command('Measure vertices / edge')
 await field.getByText('Допуск расстояния достигнут: 0,001 мм.',{exact:true}).waitFor()
 assert.deepEqual(await exportDoc('after.json'),before)
 await field.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(directory,'face-distance.png')})
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),faceA:edgeA,faceB:edgeB,expectedMm:expected,displayedIntervalMm:interval,nativeResult:await page.evaluate(()=>window.__distanceResults.at(-1)),requests:await page.evaluate(()=>window.__distanceRequests),cancelledWorkerTerminated:true,invalidFaceLocalized:true,documentUnchanged:true}
 await writeFile(path.join(directory,'face-distance-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
