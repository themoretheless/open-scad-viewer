import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-mass-errors')
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
  const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='bodyEdit'&&window.__massFailure){
     const code=window.__massFailure;window.__massFailure=null
     queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:'bodyEdit',ok:false,error:{name:'Error',code,message:'internal implementation detail'}}}))
     return
    }
    return super.postMessage(message,...args)
   }
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const download=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();await (await download).saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 async function openDiagnostics(){await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Диагностика тела');await search.press('Enter')}
 const fixture={version:1,sketches:[],bodies:[{id:'test-cube',name:'Diagnostic cube',mesh:{positions:[0,0,0,10,0,0,10,10,0,0,10,0,0,0,10,10,0,10,10,10,10,0,10,10],indices:[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}}]}
 await ready();await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'diagnostics.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await solid.getByRole('button',{name:'Diagnostic cube',exact:true}).click();const before=await exportDoc('before.json')
 const cases=[['BREP_RESOURCE_LIMIT','Исчерпан лимит вычислений'],['BREP_ANALYSIS_INDETERMINATE','Точность расчёта не подтверждена']]
 for(const [code,message] of cases){
  await page.evaluate(code=>window.__massFailure=code,code)
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Разрезать');await search.press('Enter')
  await solid.getByText('Diagnostic cube: '+message,{exact:false}).waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
  const retry=solid.getByRole('button',{name:'Повторить вычисление',exact:true});assert.equal(await retry.isEnabled(),true)
  assert.ok(!(await solid.innerText()).includes('internal implementation detail'))
  await page.screenshot({path:path.join(directory,code+'.png')})
  await page.keyboard.press('Escape')
  assert.deepEqual(await exportDoc(code+'-cancelled.json'),before)
 }
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),injectedFailureCodes:cases.map(c=>c[0]),sourceLocalized:true,applyDisabled:true,retryEnabled:true,escapePreservesDocument:true}
 await writeFile(path.join(directory,'mass-errors-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
