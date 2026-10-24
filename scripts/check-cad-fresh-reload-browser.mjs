import assert from 'node:assert/strict'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

assert(process.argv[2] && process.argv[3], 'Provide URL and new output directory')
const url=process.argv[2], output=path.resolve(process.argv[3])
const filletThroughUi=process.argv.includes('--fillet')
const faults=process.argv.includes('--faults')
assert(!faults||filletThroughUi,'Fault qualification requires --fillet')
const source=path.resolve('docs/qualification/cad-fresh-parts-shared-corner-normalized-2026-10-11')
await mkdir(output)
const manifest=JSON.parse(await readFile(path.join(source,'manifest.json'),'utf8'))
const hash=bytes=>createHash('sha256').update(bytes).digest('hex')
const {playwright}=await loadQualificationPlaywrightPackage()
const browser=await playwright.chromium.launch({headless:true,args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
let page
const rows=[],errors=[],stepParts=[]
try {
 const context=await browser.newContext({acceptDownloads:true,viewport:{width:1440,height:1100}})
 page=await context.newPage()
 page.on('pageerror',e=>errors.push(String(e)))
 if(faults)await page.addInitScript(()=>{
  const Native=window.Worker
  window.Worker=class extends Native{
   constructor(...args){super(...args);this.addEventListener('message',event=>{
    if(window.__holdFillet&&event.data?.kind==='bodyEdit'&&event.data.ok){
     event.stopImmediatePropagation();const deliver=this.onmessage
     window.__lateFillet=()=>deliver?.call(this,event);window.__holdFillet=false
    }
   },true)}
   postMessage(message,...args){
    if(message?.job?.kind==='bodyEdit'&&window.__failFillet){
     window.__failFillet=false
     queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:'bodyEdit',ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Injected qualification transport failure'}}}))
     return
    }
    return super.postMessage(message,...args)
   }
  }
 })
 await page.goto(url)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 await solid.waitFor()
 const wasm=await context.request.get(new URL('/wasm/geometry-kernel.wasm',url).href)
 assert(wasm.ok());assert.equal(hash(await wasm.body()),manifest.wasmSha256)
 const menu=solid.locator('summary[title="Файл"]')
 async function ready(){for(const name of ['history-restore','primitive-build','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 async function opened(value){if(await menu.evaluate(e=>e.parentElement.open)!==value)await menu.click()}
 let lastDownload=0
 async function exported(file){
  await ready();const delay=1100-(Date.now()-lastDownload);if(delay>0)await new Promise(r=>setTimeout(r,delay))
  await opened(true);const event=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click()
  await(await event).saveAs(path.join(output,file));lastDownload=Date.now();await opened(false)
  return JSON.parse(await readFile(path.join(output,file),'utf8'))
 }
 async function exportStep(name,stage,document){
  await ready();await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
  await solid.getByRole('button',{name:document.bodies[0].name,exact:true}).click()
  const delay=1100-(Date.now()-lastDownload);if(delay>0)await new Promise(r=>setTimeout(r,delay))
  await opened(true);const event=page.waitForEvent('download')
  await solid.getByRole('button',{name:'STEP выбранного тела · текущая геометрия',exact:true}).click()
  const file=name+'-'+stage+'.step';await(await event).saveAs(path.join(output,file));lastDownload=Date.now();await opened(false)
  const expected=structuredClone(manifest.parts.find(p=>p.name===name+'-edited-rounded').expected)
  const placement=filletThroughUi?[1,0,0]:[4,-2,1]
  for(const corner of expected.boundsMm)for(let axis=0;axis<3;axis++)corner[axis]+=placement[axis]-[3,-2,1][axis]
  stepParts.push({name:name+'-'+stage,file,sha256:hash(await readFile(path.join(output,file))),expected,
   gaugeSpecification:{part:name,rounded:true,translationMm:placement}})
 }
 for(const name of ['bracket','flange','enclosure']){
  const fixture=filletThroughUi?path.resolve('docs/qualification/cad-fresh-ui-fillet-source-2026-10-11',name+'-source.json'):path.join(source,name+'-restored.json')
  await ready();await opened(true);await solid.locator('input[accept=".json,application/json"]').setInputFiles(fixture);await opened(false);await ready()
  await solid.getByRole('tab',{name:'Сцена',exact:true}).click()
  let before=await exported(name+'-before.json')
  assert.equal(before.bodies.length,1)
  if(filletThroughUi){
   await solid.getByRole('button',{name:before.bodies[0].name,exact:true}).click()
   await solid.getByRole('button',{name:'Рёбра',exact:true}).click();await ready()
   const model=before.bodies[0].brep
   const edges=model.edges.flatMap((edge,index)=>{
    const points=edge.vertices.map(i=>model.vertices[i].point)
    return (name==='flange'?edge.curve.degree===2&&points.every(p=>Math.abs(p[2]-6)<1e-8&&Math.abs(Math.hypot(p[0],p[1])-20)<1e-8):points.every(p=>p[0]===0&&p[1]===0))?[index]:[]
   })
   assert(edges.length>0)
   for(const [i,index] of edges.entries())await solid.locator(`[data-topology-edge="${model.topologyIds.edges[index]}"]`).press(i?'Shift+Enter':'Enter')
   async function beginFillet(){
    await page.getByRole('button',{name:'Команды',exact:true}).click()
    const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
    await search.fill('Скруглить 3D');await search.press('Enter')
    await solid.getByLabel('Радиус / размер, мм',{exact:true}).fill('1 mm')
    await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click({trial:true})
   }
   await beginFillet();await page.keyboard.press('Escape')
   assert.deepEqual(await exported(name+'-fillet-cancelled.json'),before)
   if(faults){
    await beginFillet();await page.evaluate(()=>{window.__failFillet=true})
    await solid.getByLabel('Радиус / размер, мм',{exact:true}).fill('1.05 mm')
    const retry=solid.getByRole('button',{name:'Повторить вычисление',exact:true})
    await retry.waitFor();assert(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled())
    assert.deepEqual(await exported(name+'-worker-error.json'),before)
    await retry.click();await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click({trial:true})
    await page.keyboard.press('Escape');assert.deepEqual(await exported(name+'-retry-cancelled.json'),before)
    await beginFillet();await page.evaluate(()=>{window.__holdFillet=true;window.__lateFillet=undefined})
    await solid.getByLabel('Радиус / размер, мм',{exact:true}).fill('1.1 mm')
    await page.waitForFunction(()=>typeof window.__lateFillet==='function')
    await page.keyboard.press('Escape');await page.evaluate(()=>window.__lateFillet())
    assert.deepEqual(await exported(name+'-late-result.json'),before)
   }
   await beginFillet();await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
   const rounded=await exported(name+'-fillet-applied.json');assert.notDeepEqual(rounded,before)
   await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exported(name+'-fillet-undo.json'),before)
   await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exported(name+'-fillet-redo.json'),rounded)
   before=rounded
   await solid.getByRole('button',{name:'Тела',exact:true}).click()
  }
  await solid.getByRole('button',{name:before.bodies[0].name,exact:true}).click()
  await page.getByRole('button',{name:'Команды',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  await search.fill('Transform selection');await search.press('Enter')
  await solid.getByLabel('X',{exact:true}).fill('1 mm')
  await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
  const edited=await exported(name+'-edited.json')
  assert.notDeepEqual(edited,before)
  assert.deepEqual(edited.bodies[0].brep.topologyIds,before.bodies[0].brep.topologyIds)
  await solid.getByRole('button',{name:'↶',exact:true}).click()
  assert.deepEqual(await exported(name+'-undo.json'),before)
  await solid.getByRole('button',{name:'↷',exact:true}).click()
  assert.deepEqual(await exported(name+'-redo.json'),edited)
  await exportStep(name,'ui-edited',edited)
  await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor()
  await page.reload();await solid.waitFor();await ready()
  assert.deepEqual(await exported(name+'-reload.json'),edited)
  await exportStep(name,'ui-reloaded',edited)
  await page.screenshot({path:path.join(output,name+'-reload.png')})
  rows.push({part:name,uiTranslation:true,uiFillet:filletThroughUi,filletCancel:filletThroughUi,workerErrorRetryCancel:faults,lateWorkerResponseIgnored:faults,undoExact:true,redoExact:true,reloadExact:true,topologyIdsPreserved:true})
 }
 assert.deepEqual(errors,[])
 await writeFile(path.join(output,'manifest.json'),JSON.stringify({schema:'fresh-cad-ui-step/1',wasmSha256:manifest.wasmSha256,toleranceMm:manifest.toleranceMm,relativeVolumeTolerance:manifest.relativeVolumeTolerance,parts:stepParts,independentStepQualified:false},null,2)+'\n',{flag:'wx'})
 await writeFile(path.join(output,'report.json'),JSON.stringify({passed:true,url,wasmSha256:manifest.wasmSha256,parts:rows,errors,scope:filletThroughUi?'Native source construction; actual UI edge selection by Enter/Shift+Enter, fillet preview/cancel/apply, Undo/Redo, translation, reload and STEP menu. Full keyboard navigation and UI source construction are not covered.':'Isolated browser storage; fresh native edited/rounded fixtures imported through UI, one additional UI translation, exact Undo/Redo and browser reload. Native construction and fillet are not claimed as UI actions.'},null,2)+'\n',{flag:'wx'})
 console.log(JSON.stringify({passed:true,parts:rows.length}))
}catch(e){
 await writeFile(path.join(output,'failure.json'),JSON.stringify({passed:false,error:String(e),completed:rows,errors},null,2)+'\n',{flag:'wx'})
 if(page){await page.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});await writeFile(path.join(output,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}
 throw e
}finally{await browser.close()}
