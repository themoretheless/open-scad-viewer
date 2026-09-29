import {isDeepStrictEqual} from 'node:util'
import {spawn} from 'node:child_process'
import {once} from 'node:events'
import {setTimeout as delay} from 'node:timers/promises'
import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile,mkdtemp,unlink} from 'node:fs/promises'
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
let browser,page,child,exit
const profile=await mkdtemp('/tmp/cad-crash-profile-'),origin=`http://127.0.0.1:${server.address().port}`
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 async function launch(){
  await unlink(path.join(profile,'DevToolsActivePort')).catch(()=>{})
  child=spawn(playwright.chromium.executablePath(),['--headless=new','--no-sandbox','--no-first-run','--no-default-browser-check','--remote-debugging-port=0','--user-data-dir='+profile],{stdio:'ignore'})
  exit=once(child,'exit')
  let port
  for(let attempt=0;attempt<200;attempt++){
   if(child.exitCode!==null||child.signalCode)throw Error('Test Chromium exited during startup')
   try{port=(await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0];if(port)break}catch{}
   await delay(100)
  }
  assert.ok(port,'Chromium must publish its debugging port')
  browser=await playwright.chromium.connectOverCDP('http://127.0.0.1:'+port)
  const context=browser.contexts()[0];page=await context.newPage();await page.setViewportSize({width:1280,height:800});await page.goto(origin)
  return context
 }
 async function kill(){const pid=child.pid;assert.ok(pid);assert.equal(child.kill('SIGKILL'),true);const [code,signal]=await exit;assert.equal(signal,'SIGKILL');browser=null;return {pid,code,signal}}
 const region=()=>page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 const menu=()=>region().locator('summary[title="Файл"]')
 async function importDoc(doc){if(await menu().evaluate(e=>!e.parentElement.open))await menu().click();await region().locator('input[accept=".json,application/json"]').setInputFiles({name:'model.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(doc))})}
 async function saved(){await region().getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await region().getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await region().getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'))}
 async function exportDoc(file){if(await menu().evaluate(e=>!e.parentElement.open))await menu().click();const pending=page.waitForEvent('download');await region().getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const item=await pending;await item.saveAs(path.join(directory,file));return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 let context=await launch()
 const fixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/async-scene-edit/mouse/before.json','utf8'))
 await page.evaluate(text=>localStorage.setItem('scad-solid-modeler-v1',text),JSON.stringify(fixture));await page.reload();await saved()
 const before=await exportDoc('before.json')
 const large={...before,groups:Array.from({length:60},(_,i)=>({name:`Group ${i}`,source:' '.repeat(70000)}))}
 const key=await page.evaluate(()=>['scad-main-modeler-v1','scad-solid-modeler-v1'].find(k=>localStorage.getItem(k)))
 const phases=[]
 for(const phase of ['before','after']){
  const session=await context.newCDPSession(page);await session.send('Debugger.enable')
  await page.evaluate(({key,phase})=>{const original=IDBObjectStore.prototype.put;IDBObjectStore.prototype.put=function(value,...args){if(this.name==='heads'&&value.key===key){if(phase==='before')debugger;if(phase==='after')this.transaction.addEventListener('complete',()=>{debugger});return original.call(this,value,...args)}return original.call(this,value,...args)}},{key,phase})
  const referenceBefore=await page.evaluate(key=>localStorage.getItem(key),key)
  const paused=new Promise(resolve=>session.once('Debugger.paused',resolve))
  await importDoc(large);await paused
  const killed=await kill()
  context=await launch();await saved()
  const recovered=await exportDoc(`recovered-${phase}.json`)
  const referenceAfter=await page.evaluate(key=>localStorage.getItem(key),key)
  const snapshots=await page.evaluate(()=>new Promise((resolve,reject)=>{const r=indexedDB.open('scad-solid-draft-snapshots-v1');r.onerror=()=>reject(r.error);r.onsuccess=()=>{const db=r.result;if(!db.objectStoreNames.contains('snapshots')){db.close();resolve([]);return}const tx=db.transaction('snapshots','readonly'),q=tx.objectStore('snapshots').getAll();q.onsuccess=()=>resolve(q.result.map(row=>({id:row.id,characters:row.text?.length})));q.onerror=()=>reject(q.error);tx.oncomplete=()=>db.close()}}))
  const matches=isDeepStrictEqual(recovered,phase==='before'?before:large)
  phases.push({phase,...killed,recoveredBodies:recovered.bodies.length,recoveredGroups:recovered.groups?.length??0,referenceBeforeCharacters:referenceBefore?.length??0,referenceAfterCharacters:referenceAfter?.length??0,snapshots,matches})
  await writeFile(path.join(directory,'process-crash-browser.json'),JSON.stringify({browser:browser.version(),profile,phases,persistentProfileReopened:true,legacyDraftMigrated:true},null,2)+'\n')
  assert.ok(matches,'Process crash recovery differs from the published document; see process-crash-browser.json')
 }
 if(await menu().evaluate(e=>e.parentElement.open))await menu().click()
 await page.screenshot({path:path.join(directory,'recovered-after-process-crash.png')})
 const report={browser:browser.version(),profile,phases,persistentProfileReopened:true,legacyDraftMigrated:true}
 await writeFile(path.join(directory,'process-crash-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page&&!page.isClosed()){await page.screenshot({path:path.join(directory,'failure.png'),timeout:3000}).catch(()=>{})}throw error}finally{if(child&&child.exitCode===null&&!child.signalCode){child.kill('SIGKILL');await exit}await new Promise(resolve=>server.close(resolve))}
