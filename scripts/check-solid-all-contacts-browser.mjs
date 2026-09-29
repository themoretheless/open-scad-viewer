import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-all-contacts')
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
  const NativeWorker=window.Worker;window.__holdContacts=false
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){if(message?.job?.kind==='meshContacts'&&window.__holdContacts){this.__held=true;window.__contactsHeld=true;return}return super.postMessage(message,...args)}
   terminate(){if(this.__held)window.__contactsTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const download=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await download).saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function importMesh(mesh,name){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:name+'.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[{id:name,name,mesh}]}))});await closeMenu();await ready();await solid.getByRole('tab',{name:'Сцена',exact:true}).click();await solid.getByRole('button',{name,exact:true}).click()}
 async function openDiagnostics(){await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Диагностика тела');await search.press('Enter')}
 const mesh={positions:[],indices:[]}
 for(const x of [0,10,20]){const n=mesh.positions.length/3;mesh.positions.push(x,0,0,x+2,0,0,x,2,0);mesh.indices.push(n,n+1,n+2,n,n+1,n+2)}
 await importMesh(mesh,'Three defects');const before=await exportDoc('before.json');await openDiagnostics()
 await solid.getByText('Найдено контактов: 3 · Обход завершён',{exact:true}).waitFor()
 assert.equal(await solid.locator('[data-diagnostic="intersection"]').count(),6)
 assert.equal(await solid.locator('[data-diagnostic="intersection-selected"]').count(),2)
 const firstX=await solid.locator('[data-diagnostic="intersection-point"]').getAttribute('cx')
 await solid.getByRole('button',{name:'Следующий контакт',exact:true}).click()
 assert.ok((await solid.locator('[data-testid="intersection-current"]').innerText()).includes('3 / 4'))
 assert.notEqual(await solid.locator('[data-diagnostic="intersection-point"]').getAttribute('cx'),firstX)
 await solid.getByRole('button',{name:'Следующий контакт',exact:true}).press('Enter')
 assert.ok((await solid.locator('[data-testid="intersection-current"]').innerText()).includes('5 / 6'))
 assert.equal(await solid.getByRole('button',{name:'Следующий контакт',exact:true}).isEnabled(),false)
 await page.screenshot({path:path.join(directory,'three-defects.png')})
 assert.deepEqual(await exportDoc('after.json'),before)
 const dense={positions:[0,0,0,2,0,0,0,2,0],indices:Array.from({length:150},()=>[0,1,2]).flat()}
 await importMesh(dense,'Contact limit');const denseBefore=await exportDoc('dense-before.json');await openDiagnostics()
 await solid.getByText('Найдено контактов: 10000 · Частичный результат',{exact:true}).waitFor({timeout:30000})
 await solid.locator('[data-testid="intersection-incomplete"]').waitFor()
 assert.equal(await solid.getByText('Недопустимых контактов сетки при относительном допуске 1e−9 не найдено.',{exact:true}).count(),0)
 await page.evaluate(()=>window.__holdContacts=true)
 const effort=solid.getByRole('combobox',{name:'Объём проверки контактов',exact:true})
 await effort.selectOption('1000000');await page.waitForFunction(()=>window.__contactsHeld)
 await effort.press('Escape');assert.equal(await page.evaluate(()=>window.__contactsTerminated),true)
 assert.deepEqual(await exportDoc('dense-cancelled.json'),denseBefore)
 await page.evaluate(()=>window.__holdContacts=false);await openDiagnostics()
 await solid.getByText('Найдено контактов: 11175 · Обход завершён',{exact:true}).waitFor({timeout:30000})
 assert.equal(await solid.locator('[data-testid="intersection-incomplete"]').count(),0)
 assert.equal(await solid.locator('[data-diagnostic="intersection"]').count(),150)
 assert.deepEqual(await exportDoc('dense-after.json'),denseBefore)
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),separateContacts:3,allTrianglesHighlighted:6,contactNavigation:true,keyboardNavigation:true,contactCancellation:true,partialContacts:10000,extendedCompleteContacts:11175,uniqueTriangleOutlines:150,documentUnchanged:true}
 await writeFile(path.join(directory,'all-contacts-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
