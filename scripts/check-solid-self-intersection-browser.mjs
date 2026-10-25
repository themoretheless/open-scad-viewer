import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-self-intersection-browser')
const fixtures=path.resolve(process.argv[3]??'/tmp/solid-self-intersection-fixtures')
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{try{
 const url=new URL(req.url,'http://localhost');if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}
 const file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
 if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}
 res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream');res.end(await readFile(file))
}catch{res.writeHead(404).end()}})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser,page
const errors=[],checks=[]
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.setDefaultTimeout(120000)
 page.on('pageerror',e=>errors.push(String(e)))
 await page.addInitScript(()=>{
  const Native=window.Worker;window.__selfRequests=0;window.__cancelledSelf=0;window.__selfDurations=[]
  window.Worker=class extends Native{
   constructor(...args){super(...args);this.addEventListener('message',e=>{if(e.data?.kind==='selfIntersection'&&e.data.ok){window.__lastSelf=e.data;if(this.timing?.id===e.data.id)window.__selfDurations.push(performance.now()-this.timing.at)}})}
   postMessage(message,...args){
    if(message?.job?.kind==='selfIntersection'){
     window.__selfRequests++;this.timing={id:message.id,at:performance.now()}
     if(window.__failSelf){window.__failSelf=false;queueMicrotask(()=>this.onerror?.({message:'Injected private diagnostic failure'}));return}
     if(window.__holdSelf){this.held=true;window.__heldSelf={message,callback:this.onmessage};return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.held)window.__cancelledSelf++;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor();console.log("workspace ready")
 const pending=solid.getByRole('status',{name:'face-contacts-pending',exact:true})
 const panel=solid.getByRole('group',{name:'face-contacts',exact:true})
 async function ready(){await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function importCase(name){
  await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click()
  await solid.locator('input[accept=".json,application/json"]').setInputFiles(path.join(fixtures,name+'.json'));await closeMenu();await ready()
  await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
  await solid.getByRole('button',{name,exact:true}).click({timeout:20000});console.log('imported',name)
 }
 async function diagnostics(){
  await solid.focus();await page.keyboard.press('ControlOrMeta+k')
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Диагностика тела');await search.press('Enter');console.log('diagnostics opened')
  if(!await panel.isVisible())await solid.getByRole('button',{name:'Проверить контакты граней',exact:true}).click()
 }
 async function exportDoc(name){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const d=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await d;await download.saveAs(path.join(directory,name));await closeMenu();return JSON.parse(await readFile(path.join(directory,name),'utf8'))}
 const retry=solid.getByRole('button',{name:'Повторить проверку контактов',exact:true})
 for(let i=0;i<4;i++){
  await importCase('annular-'+i);const before=await exportDoc(`before-${i}.json`);await diagnostics()
  await solid.getByRole('spinbutton',{name:'Лимит проверки контактов',exact:true}).fill('100000')
  await panel.getByRole('checkbox',{name:'Проверять внутри граней',exact:true}).check()
  await panel.getByRole('checkbox',{name:'Проверять точное совпадение и замыкание границ',exact:true}).check()
  await panel.getByText('Отсутствие самопересечений подтверждено.',{exact:true}).waitFor()
  await pending.waitFor({state:'hidden'})
  assert.match(await panel.locator('[data-testid="face-contacts-summary"]').innerText(),/Не завершено пар: 0 · Не посещено: 0/)
  assert.equal(await panel.locator('[data-testid="boundary-embedding-summary"]').innerText(),'Границы согласованы, замкнуты и не пересекаются.')
  assert.equal(await solid.locator('[data-diagnostic="face-contact"]').count(),0)
  const native=await page.evaluate(()=>window.__lastSelf.result)
  assert.equal(native.faces.length,31);assert.equal(native.visitedPairs,465);assert.equal(native.absenceProven,true)
  assert.deepEqual(await exportDoc(`after-${i}.json`),before)
  checks.push({case:i,faces:native.faces.length,pairs:native.visitedPairs,absenceProven:native.absenceProven});console.log('qualified',i)
 }
 await page.evaluate(()=>window.__failSelf=true);await retry.click();await panel.getByRole('alert').waitFor()
 assert.equal((await panel.innerText()).includes('Injected private'),false)
 await retry.focus();await page.keyboard.press('Enter');await panel.getByText('Отсутствие самопересечений подтверждено.',{exact:true}).waitFor()
 checks.push({retryAfterFailure:true})
 await page.evaluate(()=>window.__holdSelf=true);await retry.click();await pending.waitFor()
 await page.evaluate(()=>window.__obsoleteSelf=window.__heldSelf)
 await solid.focus();await page.keyboard.press('Escape');await pending.waitFor({state:'hidden'})
 assert.ok(await page.evaluate(()=>window.__cancelledSelf>=1))
 await diagnostics();await pending.waitFor()
 await page.evaluate(()=>{const held=window.__obsoleteSelf;held.callback({data:{...window.__lastSelf,id:held.message.id}})})
 assert.equal(await panel.locator('[data-testid="face-contacts-summary"]').count(),0);assert.equal(await pending.isVisible(),true)
 await page.evaluate(()=>window.__holdSelf=false);await pending.getByRole('button',{name:'Esc',exact:true}).click();await diagnostics()
 await panel.getByText('Отсутствие самопересечений подтверждено.',{exact:true}).waitFor();checks.push({cancelEsc:true,lateReplyIgnored:true,freshRetry:true})
 await importCase('crossing');await diagnostics()
 await panel.getByRole('checkbox',{name:'Проверять внутри граней',exact:true}).uncheck()
 await solid.locator('[data-diagnostic="face-contact"]').waitFor()
 assert.match(await panel.getByRole('combobox',{name:'Найденный контакт',exact:true}).innerText(),/Грани/)
 assert.match(await solid.locator('[data-diagnostic="face-contact"]').first().getAttribute('data-faces'),/^\d+,\d+$/)
 await page.screenshot({path:path.join(directory,'contact-highlight.png')});checks.push({realContactHighlighted:true})
 assert.deepEqual(errors,[])
 await writeFile(path.join(directory,'result.json'),JSON.stringify({checks,errors,requests:await page.evaluate(()=>window.__selfRequests),requestToReplyMs:await page.evaluate(()=>window.__selfDurations)},null,2)+'\n')
 console.log(JSON.stringify({checks,errors}))
}catch(error){await page?.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});if(page)await writeFile(path.join(directory,'failure.html'),await page.content());throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
