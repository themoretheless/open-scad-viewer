import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-rational-patch')
const prepareWeights=process.argv.includes('--prepare-weights'),keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
const nonbinaryPreparation=process.argv.includes('--nonbinary-preparation')
assert.ok(!nonbinaryPreparation||prepareWeights,'Nonbinary qualification requires preparation')
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
let browser
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 const page=await browser.newPage({acceptDownloads:true})
 const origin=`http://127.0.0.1:${server.address().port}`
 await page.goto(origin)
 await page.getByRole('combobox',{name:'Тема',exact:true}).selectOption(theme)
 let tabPresses=0
 async function tabTo(locator){
  for(let i=0;i<250;i++){
   if(await locator.evaluate(el=>el===document.activeElement))return
   await page.keyboard.press('Tab');tabPresses++
  }
  throw Error('Target is unreachable through sequential Tab navigation: '+await locator.getAttribute('aria-label'))
 }
 async function activate(locator){
  if(keyboard){await tabTo(locator);await page.keyboard.press('Enter')}
  else await locator.click()
 }
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 async function selectBoundaries(){
  for(let i=0;i<4;i++){
   const row=solid.getByRole('button',{name:`Boundary ${i}`,exact:true})
   if(keyboard){await tabTo(row);await page.keyboard.press(i?'Shift+Enter':'Enter')}
   else await row.click({modifiers:i?['Shift']:[]})
  }
 }
 async function inputBudget(field,value){
  if(keyboard){await tabTo(field);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}
  else await field.fill(value)
  await field.press('Tab')
 }
 async function cancelPatch(){
  if(keyboard)await page.keyboard.press('Escape')
  else await solid.getByLabel('3D — тела',{exact:true}).getByRole('button',{name:'Esc',exact:true}).click()
 }
 let downloads=0
 const menu=solid.locator('summary[title="Файл"]')
 async function openMenu(){if(await menu.evaluate(e=>!e.parentElement.open))await activate(menu)}
 async function download(label,file,json=true){
  await openMenu()
  const pending=page.waitForEvent('download',{timeout:20000})
  await activate(solid.getByRole('button',{name:label,exact:true}))
  const item=await pending
  assert.equal(await item.failure(),null)
  await item.saveAs(path.join(directory,file));downloads++
  if(keyboard){
   await page.keyboard.press('Escape')
   assert.equal(await menu.evaluate(e=>e.parentElement.open),false)
   assert.equal(await menu.evaluate(e=>e===document.activeElement),true)
  }
  const text=await readFile(path.join(directory,file),'utf8')
  return json?JSON.parse(text):text
 }
 const controlPoints=[[[1,0,0],[1,1,0],[0,1,0]],[[1,0,2],[1,1,2],[0,1,2]],[[1,0,0],[1,0,2]],[[0,1,0],[0,1,2]]]
 let curves=controlPoints.map((points,i)=>({id:`boundary-${i}`,name:`Boundary ${i}`,curve:{degree:points.length-1,controlPoints:points,knots:[...Array(points.length).fill(0),...Array(points.length).fill(1)],weights:i<2?[1,Math.SQRT1_2,1]:[1,1]}}))
 const fixture=process.argv.find(a=>a.startsWith('--fixture='))?.slice(10)
 if(fixture)curves=JSON.parse(await readFile(fixture,'utf8')).curves
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'rational-patch.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves}))})
 await page.waitForFunction(()=>document.querySelector('input[accept=".json,application/json"]')?.value==='')
 await solid.getByRole('button',{name:'Boundary 3',exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await selectBoundaries()
 async function patch(){
  await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}))
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  if(keyboard)await search.pressSequentially('Coons patch');else await search.fill('Coons patch');await search.press('Enter')
 }
 await patch()
 const topRole=solid.getByRole('combobox',{name:'Верх · vMax',exact:true})
 async function setTopRole(id){
  if(!keyboard){await topRole.selectOption(id);return}
  await tabTo(topRole)
  const index=await topRole.locator('option').evaluateAll((options,id)=>options.findIndex(o=>o.value===id),id)
  assert.ok(index>=0)
  const current=await topRole.evaluate(el=>el.selectedIndex)
  for(let i=0;i<Math.abs(index-current);i++)await page.keyboard.press(index<current?'ArrowUp':'ArrowDown')
  await page.keyboard.press('Tab')
  assert.equal(await topRole.inputValue(),id)
 }
 await setTopRole(curves[0].id)
 await solid.getByText('Для каждой роли выберите отдельную кривую.',{exact:false}).waitFor()
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 await setTopRole(curves[1].id)
 const rightDirection=solid.getByRole('checkbox',{name:'Развернуть: Справа · uMax',exact:true})
 if(keyboard){await tabTo(rightDirection);await page.keyboard.press('Space')}else await rightDirection.check()
 await solid.getByText('Угол 2:',{exact:false}).waitFor()
 assert.equal(await solid.locator('[data-diagnostic="patch-gap"] circle').count(),2)
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 if(keyboard){await tabTo(rightDirection);await page.keyboard.press('Space')}else await rightDirection.uncheck()
 await solid.getByRole('button',{name:'Готово · Enter',exact:true}).waitFor()
 await solid.locator('[data-preview-body]').first().waitFor({state:'visible'})
 await cancelPatch()
 const canceled=await download('Скачать проект JSON','patch-canceled.json')
 assert.equal(canceled.surfaces.length,0)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await patch()
 await activate(solid.getByRole('button',{name:'Готово · Enter',exact:true}))
 const committed=await download('Скачать проект JSON','patch-committed.json')
 assert.equal(committed.surfaces.length,1)
 assert.deepEqual(committed.curves,curves)
 assert.ok(committed.surfaces[0].surface.weights.some(row=>row.some(w=>w!==1)))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','patch-undone.json')
 assert.equal(undone.surfaces.length,0)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','patch-redone.json')
 assert.deepEqual(redone.surfaces,committed.surfaces)
 await page.reload();await solid.getByRole('button',{name:'Boundary 3',exact:true}).waitFor()
 const reloaded=await download('Скачать проект JSON','patch-reloaded.json')
 assert.deepEqual(reloaded.surfaces,committed.surfaces);assert.deepEqual(reloaded.curves,curves)
 const invalid=structuredClone(curves);invalid[0].curve.weights[invalid[0].curve.weights.length-1]=.5
 if(nonbinaryPreparation)invalid.forEach((c,edge)=>{c.curve.weights=c.curve.weights.map((_,i)=>.1+(7+edge*13+i*17)/31)})
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'incompatible.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves:invalid}))})
 await page.waitForFunction(()=>document.querySelector('input[accept=".json,application/json"]')?.value==='')
 const importedInvalid=await download('Скачать проект JSON','patch-invalid-import.json')
 assert.deepEqual(importedInvalid.curves,invalid)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await selectBoundaries()
 await patch()
 await solid.getByText('Несовместимые угловые веса.',{exact:false}).waitFor()
 const refused=await download('Скачать проект JSON','patch-refused.json')
 assert.equal(refused.surfaces.length,0)
 assert.deepEqual(refused.curves,invalid)
 if(prepareWeights){
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  const checkbox=solid.getByRole('checkbox',{name:'Prepare patch boundary weights',exact:true})
  if(keyboard){await tabTo(checkbox);await page.keyboard.press('Space')}else await checkbox.check()
  await solid.locator('[data-diagnostic="patch-preparation"]').waitFor()
  const budget=solid.getByRole('textbox',{name:'Допуск подготовки, мм',exact:true})
  await inputBudget(budget,'0')
  await solid.getByText('Подготовка границ превышает допуск.',{exact:false}).waitFor()
  await solid.locator('[data-diagnostic="patch-budget"]').waitFor()
  assert.equal(await solid.locator('[data-diagnostic="patch-budget"]').count(),1)
  assert.equal(await solid.locator('[data-diagnostic="patch-budget"]').getAttribute('stroke'),'#ff647c')
  const refusal=solid.locator('.operation-card [role="alert"]').filter({hasText:'Подготовка границ превышает допуск.'})
  const errorBox=await refusal.boundingBox(),panelBox=await solid.locator('.operation-card').boundingBox()
  assert.ok(errorBox&&panelBox&&errorBox.y>=panelBox.y&&errorBox.y+errorBox.height<=panelBox.y+panelBox.height,'Preparation error is outside the visible command panel')
  await page.screenshot({path:path.join(directory,'patch-preparation-refused.png')})
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
  await inputBudget(budget,'0.000001')
  await solid.locator('[data-diagnostic="patch-preparation"]').waitFor()
  assert.equal(await solid.locator('[data-diagnostic="patch-budget"]').count(),0)
  await solid.locator('[data-preview-body]').first().waitFor({state:'visible'})
  await solid.locator('[data-diagnostic="patch-preparation"]').evaluate(el=>el.scrollIntoView({block:'center'}))
  await page.screenshot({path:path.join(directory,'patch-prepared-preview.png')})
  await cancelPatch()
  const preparedCanceled=await download('Скачать проект JSON','patch-prepared-canceled.json')
  assert.deepEqual(preparedCanceled.curves,invalid);assert.equal(preparedCanceled.surfaces.length,0)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await patch();await solid.locator('[data-diagnostic="patch-preparation"]').waitFor()
  await activate(solid.getByRole('button',{name:'Готово · Enter',exact:true}))
  const prepared=await download('Скачать проект JSON','patch-prepared.json')
  assert.deepEqual(prepared.curves,invalid);assert.equal(prepared.surfaces.length,1)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  const preparedUndone=await download('Скачать проект JSON','patch-prepared-undone.json')
  assert.deepEqual(preparedUndone.curves,invalid);assert.equal(preparedUndone.surfaces.length,0)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↷',exact:true}))
  const preparedRedone=await download('Скачать проект JSON','patch-prepared-redone.json')
  assert.deepEqual(preparedRedone.surfaces,prepared.surfaces)
  await page.reload();await solid.getByRole('button',{name:'Boundary 3',exact:true}).waitFor()
  const preparedReloaded=await download('Скачать проект JSON','patch-prepared-reloaded.json')
  assert.deepEqual(preparedReloaded.curves,invalid);assert.deepEqual(preparedReloaded.surfaces,prepared.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await selectBoundaries()
  await patch()
 }
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await cancelPatch()
 const gap=structuredClone(curves);gap[2].curve.controlPoints[0][0]=1.2
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'gap.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves:gap}))})
 await page.waitForFunction(()=>document.querySelector('input[accept=".json,application/json"]')?.value==='')
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await selectBoundaries()
 await patch()
 await solid.locator('[data-diagnostic="patch-gap"]').waitFor()
 assert.equal(await solid.locator('[data-diagnostic="patch-gap"] circle').count(),2)
 async function verifyGpuAlignment(){
  if(process.env.SOLID_GPU_HEADED!=='1')return
  await solid.locator('.fps-badge').filter({hasText:'draw'}).waitFor()
  await page.waitForFunction(()=>{
   const canvas=document.querySelector('.gpu-layer'),svg=canvas?.parentElement?.querySelector('svg')
   if(!canvas||!svg)return false
   const a=canvas.getBoundingClientRect(),b=svg.getBoundingClientRect(),r=devicePixelRatio
   return Math.abs(a.x-b.x)<1&&Math.abs(a.y-b.y)<1&&Math.abs(a.width-b.width)<1&&Math.abs(a.height-b.height)<1&&Math.abs(canvas.width-a.width*r)<=2&&Math.abs(canvas.height-a.height*r)<=2
  })
 }
 await verifyGpuAlignment()
 const viewport=await solid.locator('.canvas-viewport').last().boundingBox()
 const card=await solid.locator('.operation-card').boundingBox()
 assert.ok(viewport&&card&&viewport.x+viewport.width<=card.x+1,'Command panel overlaps viewport')
 await page.screenshot({path:path.join(directory,'patch-gap.png')})
 await page.setViewportSize({width:700,height:900})
 await verifyGpuAlignment()
 const narrowViewport=await solid.locator('.canvas-viewport').last().boundingBox()
 const narrowCard=await solid.locator('.operation-card').boundingBox()
 assert.ok(narrowViewport&&narrowCard&&narrowViewport.y+narrowViewport.height<=narrowCard.y+1,'Narrow command panel overlaps viewport')
 assert.ok(narrowViewport.height>=120&&narrowCard.height>100)
 const fps=await solid.locator('.fps-badge').boundingBox()
 assert.ok(fps&&fps.y+fps.height<=narrowCard.y,'FPS badge overlaps command panel')
 await page.screenshot({path:path.join(directory,'patch-gap-narrow.png')})

 const report={prepareWeights,nonbinaryPreparation,keyboard,tabPresses,roleRecovery:true,gpuAlignment:process.env.SOLID_GPU_HEADED==='1',narrowLayout:true,gapEndpointMarkers:2,localizedRefusal:true,browser:browser.version(),rationalPatch:true,cancel:true,undo:true,sourceCurvesPreserved:true,downloads}
 await writeFile(path.join(directory,'rational-patch-browser.json'),JSON.stringify(report,null,2)+'\n')
 console.log(report)
}catch(error){
 if(browser){const pages=browser.contexts().flatMap(c=>c.pages());if(pages[0]){await pages[0].screenshot({path:path.join(directory,'failure.png')});await writeFile(path.join(directory,'failure.txt'),await pages[0].locator('body').innerText())}}
 throw error
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
