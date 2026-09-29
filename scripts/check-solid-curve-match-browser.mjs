import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-seam-preparation')
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
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true})
 const pageErrors=[];page.on('pageerror',e=>pageErrors.push(String(e)))
 if(process.argv.includes('--hold-curve-display'))await page.addInitScript(()=>{
  const OriginalWorker=window.Worker
  window.__curveHeld=false;window.__curveTerminated=false
  window.Worker=class extends OriginalWorker{
   postMessage(message,...rest){if(message?.job?.kind==='curveDisplay'&&!window.__curveHeld){window.__curveHeld=true;this.heldCurve=true;return}return super.postMessage(message,...rest)}
   terminate(){if(this.heldCurve)window.__curveTerminated=true;return super.terminate()}
  }
 })
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
 const definition={degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:[[0,1,2],[1,2,3],[3,1,4],[4,2,5]],weights:[1,.7,1.3,.9]}
 const a={id:'match-a',name:'Match A',curve:definition}
 const b=structuredClone(a);b.id='match-b';b.name='Match B';b.curve.controlPoints=b.curve.controlPoints.map(p=>p.map(x=>x+10));b.curve.weights=[.8,1.2,1,.6]
 const original={version:1,sketches:[],bodies:[],curves:[a,b]}
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'prepare.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 await solid.getByRole('button',{name:a.name,exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 if(process.argv.includes('--hold-curve-display')){
  const status=solid.getByRole('status',{name:'curve-display',exact:true})
  await status.waitFor();await page.waitForFunction(()=>window.__curveHeld)
  await status.getByRole('button',{name:'Esc',exact:true}).click()
  assert.equal(await page.evaluate(()=>window.__curveTerminated),true)
  await solid.getByRole('button',{name:'Обновить кривые',exact:true}).click()
 }
 await solid.getByRole('status',{name:'curve-display',exact:true}).waitFor({state:'hidden'})
 await solid.getByRole('button',{name:a.name,exact:true}).click();await solid.getByRole('button',{name:b.name,exact:true}).click({modifiers:['Shift']})
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await solid.getByRole('region',{name:'3D — тела',exact:true}).getByRole('button',{name:'Вписать',exact:true}).click()
 await command('Match curves G1')
 await solid.getByLabel('A · опорная',{exact:true}).selectOption(a.id);await solid.getByLabel('B · изменяемая',{exact:true}).selectOption(b.id)
 await solid.getByLabel('Конец A',{exact:true}).selectOption('start');await solid.getByLabel('Конец B',{exact:true}).selectOption('end')
 const tolerance=solid.getByLabel('Угловой допуск, °',{exact:true})
 await tolerance.fill('invalid');assert.equal(await apply.isDisabled(),true)
 await tolerance.fill('0 deg');await solid.getByText('Угловая ошибка превышает допуск. Увеличьте допуск или измените входы.',{exact:true}).first().waitFor();assert.equal(await apply.isDisabled(),true)
 await tolerance.fill('0.000001 deg');await apply.click({trial:true})
 const proof=await solid.getByTestId('curve-match-report').textContent();assert.match(proof,/Допуск подтверждён/)
 assert.equal(await solid.locator('[data-preview="curve-match"]').count(),1)
 await solid.getByRole('status',{name:'curve-display',exact:true}).waitFor({state:'hidden'})
 assert.equal((await solid.locator('[data-preview="curve-match"]').getAttribute('points')).trim().split(' ').length,49)
 await page.screenshot({path:path.join(directory,'curve-match-preview.png')})
 await activate(solid.getByRole('button',{name:'Esc',exact:true}))
 const canceled=await download('Скачать проект JSON','curve-match-canceled.json');assert.deepEqual(canceled.curves,original.curves)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Match curves G1');await activate(apply)
 const committed=await download('Скачать проект JSON','curve-match-committed.json')
 assert.deepEqual(committed.curves[0],a);assert.equal(committed.curves[1].id,b.id)
 assert.deepEqual(committed.curves[1].curve.controlPoints[3],a.curve.controlPoints[0])
 assert.deepEqual(committed.curves[1].curve.weights,b.curve.weights);assert.deepEqual(committed.curves[1].curve.knots,b.curve.knots)
 const angle=Number(proof.match(/≤ ([0-9.eE+-]+)°/)[1])*1.001
 await writeFile(path.join(directory,'browser-oracle-fixtures.json'),JSON.stringify([{reference:a.curve,editedBefore:b.curve,referenceEnd:'start',editedEnd:'end',result:{curve:committed.curves[1].curve,report:{angleDegreesUpper:angle,sineAngleUpper:angle*Math.PI/180,maxAngleDegrees:1e-6}}}]))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','curve-match-undone.json');assert.deepEqual(undone.curves,original.curves)
 assert.deepEqual(pageErrors,[])
 const report={browser:browser.version(),curveWorkerCancelled:await page.evaluate(()=>window.__curveTerminated===true),proof,refusal:true,cancel:true,undo:true,identityPreserved:true,downloads,keyboard,tabPresses}
 await writeFile(path.join(directory,'curve-match-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
