import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-palette-navigation')
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
 browser=await playwright.chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1100}});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))})
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const download=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await download).saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 const fixture={version:1,sketches:[],bodies:[{id:'test-cube',name:'Diagnostic cube',mesh:{positions:[0,0,0,10,0,0,10,10,0,0,10,0,0,0,10,10,0,10,10,10,10,0,10,10],indices:[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}}]}
 await ready();await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'diagnostics.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
 await solid.getByRole('button',{name:'Diagnostic cube',exact:true}).click();const before=await exportDoc('before.json')
 await page.getByRole('button',{name:'Команды',exact:true}).click()
 const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'}),options=page.getByRole('dialog').getByRole('option')
 await search.waitFor();assert.equal(await options.count(),96)
 const rows=await options.evaluateAll(nodes=>nodes.map(n=>({id:n.id,label:n.querySelector('.command-label')?.textContent,disabled:n.getAttribute('aria-disabled')==='true',reason:n.querySelector('.command-disabled-reason')?.textContent,describedBy:n.getAttribute('aria-describedby')})))
 assert.equal(new Set(rows.map(r=>r.id)).size,96)
 for(const row of rows){assert.ok(row.label?.trim());if(row.disabled){assert.ok(row.reason?.trim());assert.ok(row.describedBy)}}
 for(let i=0;i<96;i++){
  await options.nth(i).hover();assert.equal(await options.nth(i).getAttribute('aria-selected'),'true')
 }
 await search.press('Escape');await search.waitFor({state:'hidden'})
 // The application shortcut opens the application palette. Reopen the same
 // Solid palette before qualifying its keyboard navigation.
 await page.getByRole('button',{name:'Команды',exact:true}).click();await search.waitFor()
 assert.equal(await options.count(),96)
 await search.press('ArrowDown')
 const enabled=new Set(await options.evaluateAll(nodes=>nodes.map(n=>n.id))),visited=new Set()
 for(let i=0;i<enabled.size;i++){
  const id=await search.getAttribute('aria-activedescendant');assert.ok(enabled.has(id),'Active '+id+' must be one of '+[...enabled].join(','));assert.ok(!visited.has(id));visited.add(id)
  assert.equal(await search.evaluate(e=>e===document.activeElement),true)
  if(await page.locator('[id='+JSON.stringify(id)+']').getAttribute('aria-disabled')==='true'){await search.press('Enter');assert.equal(await search.isVisible(),true)}
  await search.press('ArrowDown')
 }
 assert.deepEqual(visited,enabled)
 assert.equal(visited.size,96)
 await page.screenshot({path:path.join(directory,'palette.png')})
 await search.press('Escape');assert.deepEqual(await exportDoc('after.json'),before)
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),scope:'palette discovery and navigation only; command execution unverified',options:96,pointerVisited:96,keyboardVisited:visited.size,disabledExplained:rows.filter(r=>r.disabled).length,documentUnchanged:true,rows}
 await writeFile(path.join(directory,'palette-navigation.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
