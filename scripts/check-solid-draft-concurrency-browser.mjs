import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {tmpdir} from 'node:os'
import {readFile,mkdir,writeFile,mkdtemp,rm} from 'node:fs/promises'
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
let browser,page,persistentContext,profile
const errors=[]
const browserOptions={headless:true,channel:'chromium',args:['--enable-unsafe-webgpu']}
const contextOptions={acceptDownloads:true,viewport:{width:1440,height:1100}}
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 if(process.argv.includes('--persistent')){profile=await mkdtemp(path.join(tmpdir(),'cad-draft-quota-'));persistentContext=await playwright.chromium.launchPersistentContext(profile,{...browserOptions,...contextOptions});browser=persistentContext.browser()}
 else browser=await playwright.chromium.launch(browserOptions)
 const context=persistentContext??await browser.newContext(contextOptions)
 const origin=`http://127.0.0.1:${server.address().port}`
 page=await context.newPage();page.on('pageerror',e=>errors.push(String(e)))
 await page.goto(origin)
 const region=p=>p.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 const menu=p=>region(p).locator('summary[title="Файл"]')
 async function closeMenu(p){if(await menu(p).evaluate(e=>e.parentElement.open))await menu(p).click()}
 async function command(p,name){await closeMenu(p);await region(p).getByRole('button',{name:'Команда… Ctrl K',exact:true}).click();const search=p.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 async function saved(p){await region(p).getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await region(p).getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await region(p).getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await p.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'))}
 async function exportDoc(p,file){if(await menu(p).evaluate(e=>!e.parentElement.open))await menu(p).click();const download=p.waitForEvent('download');await region(p).getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const item=await download;await item.saveAs(path.join(directory,file));await closeMenu(p);return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await menu(page).click();await region(page).locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[]}))});await closeMenu(page);await saved(page)
 const key=await page.evaluate(()=>['scad-main-modeler-v1','scad-solid-modeler-v1'].find(k=>localStorage.getItem(k)))
 assert.ok(key)
 let other=await context.newPage();other.on('pageerror',e=>errors.push(String(e)));await other.goto(origin);await saved(other)
 // This tab keeps the initial revision while the other two resolve a conflict.
 const stale=await context.newPage();stale.on('pageerror',e=>errors.push(String(e)));await stale.goto(origin);await saved(stale)
 async function hold(){await page.evaluate(key=>{window.__held=false;window.__lock=navigator.locks.request('solid-draft:'+key,()=>new Promise(resolve=>{window.__release=resolve;window.__held=true}))},key);await page.waitForFunction(()=>window.__held)}
 async function release(){await page.evaluate(()=>window.__release());await page.evaluate(()=>window.__lock)}
 await hold()
 await Promise.all([command(page,'Box'),command(other,'Sphere')])
 await Promise.all([region(page).getByRole('status',{name:'Сохраняю…',exact:true}).waitFor(),region(other).getByRole('status',{name:'Сохраняю…',exact:true}).waitFor()])
 await release()
 await Promise.all([page.waitForFunction(()=>!document.querySelector('[aria-label="Сохраняю…"]')),other.waitForFunction(()=>!document.querySelector('[aria-label="Сохраняю…"]'))])
 const conflict=async p=>(await p.locator('body').innerText()).includes('Документ изменён в другой вкладке')
 const firstConflict=await conflict(page),secondConflict=await conflict(other);assert.notEqual(firstConflict,secondConflict)
 const loser=firstConflict?page:other,winner=firstConflict?other:page
 const normalize=d=>({...d,curves:d.curves??[],surfaces:d.surfaces??[]})
 const canonical=normalize(JSON.parse(await page.evaluate(key=>localStorage.getItem(key),key)))
 assert.deepEqual(await exportDoc(winner,'winner.json'),canonical)
 const local=await exportDoc(loser,'loser-local.json');assert.notDeepEqual(local,canonical);assert.equal(local.bodies.length,1)
 await loser.evaluate(()=>{
  const post=Worker.prototype.postMessage,terminate=Worker.prototype.terminate
  window.__restoreSharedMethods=()=>{Worker.prototype.postMessage=post;Worker.prototype.terminate=terminate}
  Worker.prototype.postMessage=function(message,...args){if(message?.job?.kind==='restoreDocument'){window.__sharedHeld=true;this.__sharedHeld=true;return}return post.call(this,message,...args)}
  Worker.prototype.terminate=function(){if(this.__sharedHeld)window.__sharedTerminated=true;return terminate.call(this)}
 })
 await menu(loser).click();await region(loser).getByRole('button',{name:'Загрузить сохранённую версию · локальная останется в Undo',exact:true}).click()
 await loser.waitForFunction(()=>window.__sharedHeld)
 await loser.keyboard.press('Escape');await loser.waitForFunction(()=>window.__sharedTerminated)
 await loser.evaluate(()=>window.__restoreSharedMethods())
 assert.deepEqual(await exportDoc(loser,'cancelled-adoption.json'),local)
 await menu(loser).click();await region(loser).getByRole('button',{name:'Загрузить сохранённую версию · локальная останется в Undo',exact:true}).click();await closeMenu(loser);await saved(loser)
 assert.deepEqual(await exportDoc(loser,'adopted.json'),canonical)
 await region(loser).getByRole('button',{name:'↶',exact:true}).click();await saved(loser);assert.deepEqual(await exportDoc(loser,'local-undo.json'),local)
 await loser.screenshot({path:path.join(directory,'recovered-local.png')})
 // A third stale writer cannot overwrite the head with subsequent edits.
 const latest=await page.evaluate(key=>localStorage.getItem(key),key)
 await command(stale,'Cylinder')
 await stale.waitForFunction(()=>document.body.innerText.includes('Документ изменён в другой вкладке'))
 const staleLocal=await exportDoc(stale,'third-stale-local.json')
 assert.notDeepEqual(staleLocal,normalize(JSON.parse(latest)))
 await menu(stale).click()
 assert.equal(await region(stale).getByRole('button',{name:'Повторить сохранение черновика',exact:true}).count(),0)
 await closeMenu(stale);await command(stale,'Sphere')
 await stale.waitForFunction(()=>document.body.innerText.includes('Документ изменён в другой вкладке'))
 assert.equal(await page.evaluate(key=>localStorage.getItem(key),key),latest)
 const staleEdited=await exportDoc(stale,'third-retry-local.json')
 assert.equal(staleEdited.bodies.length,staleLocal.bodies.length+1)
 await stale.reload();await saved(stale)
 assert.deepEqual(await exportDoc(stale,'third-reloaded.json'),normalize(JSON.parse(latest)))
 await stale.close()
 // Closing a waiting writer cannot publish its abandoned state after the lock is released.
 await other.close();other=await context.newPage();await other.goto(origin);await saved(other)
 const durable=await page.evaluate(key=>localStorage.getItem(key),key)
 await hold();await command(other,'Box');await region(other).getByRole('status',{name:'Сохраняю…',exact:true}).waitFor();await other.close();await release()
 assert.equal(await page.evaluate(key=>localStorage.getItem(key),key),durable)
 other=await context.newPage();await other.goto(origin);await saved(other)
 assert.deepEqual(await exportDoc(other,'after-close.json'),normalize(JSON.parse(durable)))
 // Abort an IndexedDB snapshot transaction before publication; the old reference survives.
 await other.evaluate(()=>{const add=IDBObjectStore.prototype.add;IDBObjectStore.prototype.add=function(...args){const request=add.apply(this,args);if(this.name==='snapshots'){this.transaction.abort()}return request}})
 const large={version:1,sketches:[],bodies:canonical.bodies,groups:Array.from({length:60},(_,i)=>({name:`Group ${i}`,source:' '.repeat(70000)}))}
 await menu(other).click();await region(other).locator('input[accept=".json,application/json"]').setInputFiles({name:'large.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(large))});await closeMenu(other)
 await region(other).getByRole('status',{name:'Не сохранено',exact:true}).waitFor()
 assert.equal(await other.evaluate(key=>localStorage.getItem(key),key),durable)
 assert.equal((await exportDoc(other,'failed-write-local.json')).groups.length,60)
 await other.reload();await saved(other);assert.deepEqual(await exportDoc(other,'after-abort-reload.json'),normalize(JSON.parse(durable)))
 // Crash the renderer exactly before/after publishing a committed snapshot reference.
 const crashPhases=[]
 for(const phase of ['before','after']){
  await other.close();other=await context.newPage();await other.goto(origin);await saved(other)
  const prior=await page.evaluate(key=>localStorage.getItem(key),key)
  const cdp=await context.newCDPSession(other);await cdp.send('Debugger.enable')
  await other.evaluate(({key,phase})=>{const original=IDBObjectStore.prototype.put;IDBObjectStore.prototype.put=function(value,...args){if(this.name==='heads'&&value.key===key){if(phase==='before')debugger;if(phase==='after')this.transaction.addEventListener('complete',()=>{debugger});return original.call(this,value,...args)}return original.call(this,value,...args)}},{key,phase})
  const paused=new Promise(resolve=>cdp.once('Debugger.paused',resolve))
  await menu(other).click()
  await region(other).locator('input[accept=".json,application/json"]').setInputFiles({name:'crash-large.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(large))})
  await paused
  const crashed=other.waitForEvent('crash')
  void cdp.send('Page.crash').catch(()=>{})
  await crashed
  await other.close();other=await context.newPage();await other.goto(origin)
  await other.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'))
  await saved(other)
  const reference=await page.evaluate(key=>localStorage.getItem(key),key)
  const restored=await exportDoc(other,`crash-${phase}.json`)
  if(phase==='before'){assert.equal(reference,prior);assert.deepEqual(restored,normalize(JSON.parse(durable)))}
  else {assert.equal(restored.groups.length,60);assert.deepEqual(restored.bodies,large.bodies)}
  crashPhases.push(phase)
 }
 const canonicalAfterCrash=await exportDoc(other,'canonical-after-crash.json')
 const missingMarker=await other.evaluate(async key=>{
  const marker=localStorage.getItem(key),id=JSON.parse(marker).solidDraftId??'missing-cache'
  await new Promise((resolve,reject)=>{const request=indexedDB.open('scad-solid-draft-snapshots-v1',1);request.onerror=()=>reject(request.error);request.onsuccess=()=>{const db=request.result,tx=db.transaction('snapshots','readwrite');tx.objectStore('snapshots').delete(id);tx.oncomplete=()=>{db.close();resolve()};tx.onabort=()=>{db.close();reject(tx.error)}}})
  localStorage.setItem(key,JSON.stringify({version:1,solidDraftId:'missing-cache'}))
  return marker
 },key)
 await other.reload();await saved(other);assert.deepEqual(await exportDoc(other,'missing-cache-recovered.json'),canonicalAfterCrash)
 await other.evaluate(key=>localStorage.setItem(key,'{broken document'),key);await other.reload();await saved(other)
 assert.deepEqual(await exportDoc(other,'corrupt-cache-recovered.json'),canonicalAfterCrash)
 await other.screenshot({path:path.join(directory,'canonical-recovery.png')})
 // Hold a real startup worker dispatch to exercise keyboard cancellation before
 // geometry arrives, then reload and recover the same canonical head.
 const recovery=await context.newPage();recovery.on('pageerror',e=>errors.push(String(e)))
 await recovery.addInitScript(()=>{
  const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='restoreDocument'&&!sessionStorage.getItem('qualification-recovery-held')){
     sessionStorage.setItem('qualification-recovery-held','1');window.__recoveryHeld=true;this.__held=true;return
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__recoveryTerminated=true;return super.terminate()}
  }
 })
 await recovery.goto(origin);await recovery.waitForFunction(()=>window.__recoveryHeld)
 const savedBeforeCancel=await recovery.evaluate(key=>localStorage.getItem(key),key)
 await region(recovery).getByRole('button',{name:'Отмена · Esc',exact:true}).focus();await recovery.keyboard.press('Escape')
 await recovery.waitForFunction(()=>window.__recoveryTerminated)
 assert.equal(await recovery.evaluate(key=>localStorage.getItem(key),key),savedBeforeCancel)
 assert.equal((await exportDoc(recovery,'cancelled-recovery.json')).bodies.length,0)
 await recovery.reload();await saved(recovery)
 assert.deepEqual(await exportDoc(recovery,'retried-recovery.json'),canonicalAfterCrash)
 await recovery.close()
 await other.reload();await saved(other);page=other
 // Corruption while a tab remains open must not be replaced by its next save.
 const originalHead=await other.evaluate(async key=>new Promise((resolve,reject)=>{
  const r=indexedDB.open('scad-solid-draft-heads-v1',1);r.onerror=()=>reject(r.error);r.onsuccess=()=>{
   const db=r.result,tx=db.transaction('heads','readwrite'),store=tx.objectStore('heads'),get=store.get(key);let head
   get.onsuccess=()=>{head=get.result;store.put({...head,text:null})}
   tx.oncomplete=()=>{db.close();resolve(head)};tx.onabort=()=>{db.close();reject(tx.error)}
  }
 }),key)
 await command(other,'Box');await region(other).getByRole('status',{name:'Не сохранено',exact:true}).waitFor()
 await region(other).locator('.error-bar').filter({hasText:'Сохранённая версия повреждена. Скачайте локальный JSON через меню «Файл».'}).waitFor()
 const unsavedLocal=await exportDoc(other,'corrupt-head-local.json')
 assert.notDeepEqual(unsavedLocal,canonicalAfterCrash)
 await other.evaluate(async head=>new Promise((resolve,reject)=>{
  const r=indexedDB.open('scad-solid-draft-heads-v1',1);r.onsuccess=()=>{
   const db=r.result,tx=db.transaction('heads','readwrite'),store=tx.objectStore('heads'),get=store.get(head.key)
   get.onsuccess=()=>{if(get.result.text!==null||get.result.revision!==head.revision){tx.abort();return}store.put(head)}
   tx.oncomplete=()=>{db.close();resolve()};tx.onabort=()=>{db.close();reject(Error('Corrupt head was overwritten'))}
  };r.onerror=()=>reject(r.error)
 }),originalHead)
 await menu(other).click();await region(other).getByRole('button',{name:'Повторить сохранение черновика',exact:true}).click();await closeMenu(other);await saved(other)
 await other.reload();await saved(other);assert.deepEqual(await exportDoc(other,'corrupt-head-retried.json'),unsavedLocal)
 // Use Chromium's real quota manager, not a mocked IndexedDB request.
 const quotaSession=await context.newCDPSession(other)
 const beforeQuota=await other.evaluate(key=>localStorage.getItem(key),key)
 const readHead=()=>other.evaluate(key=>new Promise((resolve,reject)=>{
  const r=indexedDB.open('scad-solid-draft-heads-v1',1);r.onerror=()=>reject(r.error);r.onsuccess=()=>{
   const db=r.result,tx=db.transaction('heads','readonly'),get=tx.objectStore('heads').get(key)
   tx.oncomplete=()=>{db.close();resolve(get.result)};tx.onabort=()=>{db.close();reject(tx.error)}
  }
 }),key)
 const headBeforeQuota=await readHead()
 await quotaSession.send('Storage.overrideQuotaForOrigin',{origin,quotaSize:1})
 // Chromium caches bucket space for 30 seconds (BucketContext::kBucketSpaceCacheTimeLimit).
 // Expire that allowance before attempting the application write against the new quota.
 await new Promise(resolve=>setTimeout(resolve,31000))
 const quotaEvidence=await quotaSession.send('Storage.getUsageAndQuota',{origin});console.log({quotaEvidence})
 await command(other,'Sphere');await region(other).getByRole('status',{name:'Не сохранено',exact:true}).waitFor()
 await region(other).locator('.error-bar').filter({hasText:'Хранилище заполнено. Скачайте JSON, освободите место и повторите сохранение.'}).waitFor()
 assert.equal(await other.evaluate(key=>localStorage.getItem(key),key),beforeQuota)
 assert.deepEqual(await readHead(),headBeforeQuota)
 await other.locator('.statusbar').getByText('Черновик сохранён в браузере',{exact:true}).waitFor({state:'hidden'})
 await other.screenshot({path:path.join(directory,'quota-refused.png')})
 const quotaLocal=await exportDoc(other,'quota-local.json')
 assert.notDeepEqual(quotaLocal,unsavedLocal)
 await quotaSession.send('Storage.overrideQuotaForOrigin',{origin})
 await menu(other).click();await region(other).getByRole('button',{name:'Повторить сохранение черновика',exact:true}).click();await closeMenu(other);await saved(other)
 await other.reload();await saved(other);assert.deepEqual(await exportDoc(other,'quota-retried.json'),quotaLocal)
 assert.notEqual((await readHead()).revision,headBeforeQuota.revision)
 // Large documents first add an immutable snapshot; failure must not publish a new head
 // or leave an orphan row, and retry must publish a resolvable snapshot reference.
 const snapshotKeys=()=>other.evaluate(()=>new Promise((resolve,reject)=>{
  const r=indexedDB.open('scad-solid-draft-snapshots-v1',1);r.onerror=()=>reject(r.error);r.onsuccess=()=>{
   const db=r.result,tx=db.transaction('snapshots','readonly'),get=tx.objectStore('snapshots').getAllKeys()
   tx.oncomplete=()=>{db.close();resolve(get.result)};tx.onabort=()=>{db.close();reject(tx.error)}
  }
 }))
 const beforeLargeHead=await readHead(),beforeLargeReference=await other.evaluate(key=>localStorage.getItem(key),key),beforeLargeKeys=await snapshotKeys()
 await quotaSession.send('Storage.overrideQuotaForOrigin',{origin,quotaSize:1})
 await new Promise(resolve=>setTimeout(resolve,31000))
 await menu(other).click();await region(other).locator('input[accept=".json,application/json"]').setInputFiles({name:'quota-large.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(large))});await closeMenu(other)
 await region(other).getByRole('status',{name:'Не сохранено',exact:true}).waitFor()
 await region(other).locator('.error-bar').filter({hasText:'Хранилище заполнено. Скачайте JSON, освободите место и повторите сохранение.'}).waitFor()
 assert.deepEqual(await readHead(),beforeLargeHead)
 assert.deepEqual(await snapshotKeys(),beforeLargeKeys)
 assert.equal(await other.evaluate(key=>localStorage.getItem(key),key),beforeLargeReference)
 const quotaLargeLocal=await exportDoc(other,'quota-large-local.json')
 assert.ok(JSON.stringify(quotaLargeLocal).length>4_000_000)
 await quotaSession.send('Storage.overrideQuotaForOrigin',{origin});await quotaSession.detach()
 await menu(other).click();await region(other).getByRole('button',{name:'Повторить сохранение черновика',exact:true}).click();await closeMenu(other);await saved(other)
 const largeReference=JSON.parse(await other.evaluate(key=>localStorage.getItem(key),key))
 assert.equal(typeof largeReference.solidDraftId,'string')
 assert.ok((await snapshotKeys()).includes(largeReference.solidDraftId))
 assert.notEqual((await readHead()).revision,beforeLargeHead.revision)
 await other.reload();await saved(other);assert.deepEqual(await exportDoc(other,'quota-large-retried.json'),quotaLargeLocal)
 if(profile){
  await persistentContext.close()
  persistentContext=await playwright.chromium.launchPersistentContext(profile,{...browserOptions,...contextOptions})
  browser=persistentContext.browser();page=await persistentContext.newPage();page.on('pageerror',e=>errors.push(String(e)))
  await page.goto(origin);await saved(page)
  assert.deepEqual(await exportDoc(page,'browser-restarted.json'),quotaLargeLocal)
  await page.screenshot({path:path.join(directory,'browser-restarted.png')})
 }
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),browserRestart:!!profile,key,simultaneousWriters:true,thirdStaleWriterEditsAndReload:true,conflictPreservesBoth:true,adoptUndo:true,sharedLoadWorkerCancellation:true,closedWaitingWriter:true,abortedSnapshotRetainsPrevious:true,rendererCrashAtPublication:crashPhases,missingAndCorruptCacheRecovered:true,startupWorkerCancellationAndReload:true,corruptHeadPreservedAndRetry:true,realQuotaRefusalAndRetry:true,largeSnapshotQuotaRefusalAndRetry:true,quotaEvidence}
 await writeFile(path.join(directory,'draft-concurrency-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page&&!page.isClosed()){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await persistentContext?.close();await browser?.close();await new Promise(resolve=>server.close(resolve));if(profile)await rm(profile,{recursive:true,force:true})}
