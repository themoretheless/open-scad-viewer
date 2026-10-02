import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-edge-errors')
const inputMode=process.env.SOLID_EDGE_INPUT??'keyboard';assert.ok(['mouse','keyboard'].includes(inputMode))
const mode=process.argv[3]??'constant';assert.ok(['constant','variable','corner'].includes(mode))
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
try{
 const {playwright}=await loadQualificationPlaywrightPackage();browser=await playwright.chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}: {})})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.on('pageerror',e=>errors.push(String(e)))
 await page.addInitScript(()=>{
  window.__holdEdge=false;window.__lateEdge=null;window.__edgeTerminated=false;window.__edgeRequests=0
  const NativeWorker=window.Worker
  window.Worker=class extends NativeWorker{
   postMessage(message,...args){
    if(message?.job?.kind==='bodyEdit'&&message.job.options?.operation==='edge-fillet'){
     window.__edgeRequests++
     if(window.__holdEdge){
      this.held=true;const callback=this.onmessage
      this.onmessage=event=>{if(event.data?.ok===true)window.__lateEdge=fail=>callback?.call(this,fail?{data:{...event.data,ok:false,error:{name:'Error',code:'CAD_CRASH',message:'Late discarded edge failure'}}}:event)}
     }
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.held)window.__edgeTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function activate(locator){if(inputMode==='keyboard')await locator.press('Enter');else await locator.click()}
 async function enterText(locator,text){
  if(inputMode==='keyboard'){
   await locator.press('ControlOrMeta+A');await locator.press('Backspace')
   await page.keyboard.insertText(text)
   if(await locator.getAttribute('role')!=='combobox')await locator.press('Tab')
  }else await locator.fill(text)
 }
 async function selectEdge(id,add=false){
  const edge=solid.locator(`[data-topology-edge="${id}"]`)
  if(inputMode==='keyboard'){await edge.press(add?'Shift+Enter':'Enter');return}
  const point=await edge.evaluate(e=>{
   const m=e.getScreenCTM(),length=e.getTotalLength()
   for(const fraction of [.5,.25,.75,.1,.9]){
    const p=e.getPointAtLength(length*fraction),point={x:m.a*p.x+m.c*p.y+m.e,y:m.b*p.x+m.d*p.y+m.f}
    if(document.elementFromPoint(point.x,point.y)===e)return point
   }
   throw Error('Selected edge has no unobstructed sampled mouse target')
  })
  if(add)await page.keyboard.down('Shift')
  try{await page.mouse.click(point.x,point.y)}finally{if(add)await page.keyboard.up('Shift')}
 }
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 async function openMenu(){await ready();if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)}
 let lastDownload=0
 async function download(label,file){
  const wait=1100-(Date.now()-lastDownload);if(wait>0)await new Promise(resolve=>setTimeout(resolve,wait));lastDownload=Date.now()
  await openMenu();const pending=page.waitForEvent('download');await activate(solid.getByRole('button',{name:label,exact:true}));await(await pending).saveAs(path.join(directory,file));await closeMenu();return readFile(path.join(directory,file))
 }
 async function doc(name){return JSON.parse(await download('Скачать проект JSON',name+'.json'))}

 const fixtureRoot=path.resolve(process.env.SOLID_EDGE_FIXTURE_ROOT??'docs/qualification/cad-roadmap-2026-09-28/edge-errors-2026-09-30')
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles(path.join(fixtureRoot,'fixture.json'));await closeMenu();await ready()
 await solid.getByRole('button',{name:JSON.parse(await readFile(path.join(fixtureRoot,'fixture.json'),'utf8')).bodies[0].name,exact:true}).waitFor();await ready()
 const before=await doc('before'),id=(await readFile(path.join(fixtureRoot,'edge-id.txt'),'utf8')).trim()
 await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:before.bodies[0].name,exact:true}));await activate(solid.getByRole('button',{name:'Рёбра',exact:true}));await ready()
 await activate(solid.getByRole('button',{name:'Вписать',exact:true}).last());await ready()
 const brep=before.bodies[0].brep,max=[0,1,2].map(axis=>Math.max(...brep.vertices.map(v=>v.point[axis])))
 const vertex=brep.vertices.findIndex(v=>v.point.every((x,i)=>x===max[i]))
 const fixtureEdges=await readFile(path.join(fixtureRoot,'edge-ids.json'),'utf8').then(JSON.parse).catch(error=>{if(error.code==='ENOENT')return [id];throw error})
 const ids=mode==='corner'&&!process.env.SOLID_EDGE_CORNER_EDGES?brep.edges.flatMap((e,i)=>e.vertices.includes(vertex)?[brep.topologyIds.edges[i]]:[]):fixtureEdges
 if(inputMode==='mouse'){
  const bounds=await solid.locator(`[data-topology-edge="${ids[0]}"]`).evaluate(e=>{
   const r=e.ownerSVGElement.getBoundingClientRect();return {x:r.x+r.width*.7,y:r.y+r.height*.2}
  })
  await page.mouse.move(bounds.x,bounds.y);await page.mouse.down({button:'right'})
  await page.mouse.move(bounds.x+80,bounds.y+35,{steps:12});await page.mouse.up({button:'right'});await ready();await activate(solid.getByRole('button',{name:'Вписать',exact:true}).last());await ready()
 }
 if(ids.length>1&&mode==='constant'){
  await selectEdge(ids[0])
  await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}))
  const partialSearch=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await enterText(partialSearch,'Скруглить 3D');await partialSearch.press('Enter')
  const curved=brep.edges[brep.topologyIds.edges.indexOf(ids[0])].curve.degree>1
  await solid.getByText(curved?'Выберите полное кольцо:':'Выберите всю цепочку рёбер',{exact:false}).waitFor()
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
  assert.deepEqual(await doc('partial-rim'),before)
  await page.keyboard.press('Escape');await ready()
 }
 for(const [i,edge] of ids.entries())await selectEdge(edge,i>0)
 await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}))
 const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await enterText(search,'Скруглить 3D');await search.press('Enter')
 const filletType=solid.getByRole('combobox',{name:'Тип скругления',exact:true})
 if(inputMode==='keyboard'){
  const label=await filletType.locator('option').evaluateAll((nodes,value)=>nodes.find(n=>n.value===value)?.textContent,mode)
  await filletType.press('Tab');await page.keyboard.press('Shift+Tab')
  const cdp=await page.context().newCDPSession(page)
  try{for(const letter of label)await cdp.send('Input.dispatchKeyEvent',{type:'char',text:letter,key:letter})}finally{await cdp.detach()}
 }else await filletType.selectOption(mode)
 assert.equal(await filletType.inputValue(),mode)
 if(process.env.SOLID_EDGE_FIXTURE_ROOT){
  await page.evaluate(()=>window.__holdEdge=true)
  await enterText(solid.getByRole('textbox',{name:'Радиус / размер, мм',exact:true}),'1.25 mm')
  await page.waitForFunction(()=>typeof window.__lateEdge==='function')
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
  if(process.env.SOLID_EDGE_CONTEXT_SWITCH){
   await page.getByRole('button',{name:'Mesh',exact:true}).click()
   await page.evaluate(()=>{window.__lateEdge(false);window.__lateEdge(true)})
   await page.getByRole('button',{name:'Solid',exact:true}).click();await ready()
   assert.deepEqual(await doc('context-restored'),before)
   assert.equal(await solid.locator('[data-preview-body]').count(),0)
  }else await page.keyboard.press('Escape')
  await ready();assert.deepEqual(await doc('cancelled'),before)
  assert.equal(await page.evaluate(()=>window.__edgeTerminated),true)
  await page.evaluate(()=>{window.__holdEdge=false;window.__lateEdge(false);window.__lateEdge(true)})
  assert.deepEqual(await doc('late-cancelled'),before)
  assert.equal(await solid.locator('[data-preview-body]').count(),0)
  await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));await enterText(search,'Скруглить 3D');await search.press('Enter')
  await enterText(solid.getByRole('textbox',{name:'Радиус / размер, мм',exact:true}),'2 mm')
  await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent?.trim()==='Готово · Enter'&&!b.disabled))
  const currentPreview=await solid.locator('[data-preview-body]').evaluateAll(nodes=>nodes.map(n=>n.outerHTML))
  assert.ok(currentPreview.length>0)
  await page.evaluate(()=>{window.__lateEdge(false);window.__lateEdge(true)})
  assert.deepEqual(await doc('late-reopened'),before)
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),false)
  assert.deepEqual(await solid.locator('[data-preview-body]').evaluateAll(nodes=>nodes.map(n=>n.outerHTML)),currentPreview)
 }
 const amount=solid.getByRole('textbox',{name:mode==='variable'?'Радиус A, мм':'Радиус / размер, мм',exact:true});await enterText(amount,'30 mm')
 await solid.getByText('Размер сопряжения не помещается.',{exact:false}).waitFor()
 assert.ok((await solid.innerText()).includes(before.bodies[0].name+' · рёбра '))
 assert.ok((await solid.innerText()).includes('Уменьшите радиус или размер фаски.'))
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 assert.equal(await solid.locator('.command-keys').getByText('Enter',{exact:true}).count(),0)
 assert.equal(await solid.locator('.gizmo-dimension.failed').count(),1)
 assert.deepEqual(await doc('failed'),before)
 assert.equal(await solid.locator('.invalid-input-geometry').count(),0)
 const unselected=await solid.locator('[data-topology-edge][aria-pressed="false"]').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('stroke')))
 assert.ok(unselected.every(color=>color==='#89baff'))
 for(const edge of ids)assert.equal(await solid.locator(`[data-topology-edge="${edge}"]`).getAttribute('stroke'),'#f87171')
 await page.screenshot({path:path.join(directory,'error.png')})
 await enterText(amount,'1 mm');await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click({trial:true})
 const requestsBeforeApply=await page.evaluate(()=>window.__edgeRequests)
 await activate(solid.getByRole('button',{name:'Готово · Enter',exact:true}));await ready()
 assert.equal(await page.evaluate(()=>window.__edgeRequests),requestsBeforeApply)
 const after=await doc('after');assert.notDeepEqual(after.bodies[0].brep,before.bodies[0].brep)
 await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await doc('undo'),before)
 await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await doc('redo'),after)
 await page.reload();await solid.getByRole('button',{name:after.bodies[0].name,exact:true}).waitFor();await ready();assert.deepEqual(await doc('reloaded'),after)
 assert.deepEqual(errors,[]);await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,mode,inputMode,keyboardNumericInput:inputMode==='keyboard',contextSwitch:!!process.env.SOLID_EDGE_CONTEXT_SWITCH,oversizeRefusal:true,localized:true,reduceRadiusRecovery:true,undoRedo:true,reload:true,applyWithoutRecompute:true,lateResponseAfterCancelAndReopen:!!process.env.SOLID_EDGE_FIXTURE_ROOT,errors},null,2))
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
