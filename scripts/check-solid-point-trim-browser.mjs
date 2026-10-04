import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-sketch-snaps')
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
  const NativeWorker=window.Worker;window.__snapRequests=0;window.__holdSnaps=true
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='nurbsEdit'&&['trim-point','trim-screen-point'].includes(message.job.options.kind)){
     window.__snapRequests++
     if(window.__holdSnaps){this.__held=true;window.__snapHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__snapTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 const fixture={version:1,bodies:[],sketches:[],curves:[{id:'arc',name:'Arc',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[10,0,0],[10,10,0],[0,10,0]],weights:[1,Math.SQRT1_2,1]}}]}
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'point-trim.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
 await solid.getByRole('button',{name:'Arc',exact:true}).click()
 await solid.getByRole('status',{name:'curve-display',exact:true}).waitFor({state:'hidden'})
 const before=await exportDoc('before.json')
 async function begin(){
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Обрезать NURBS по точке');await search.press('Enter')
 }
 async function setPoint(x,y){await solid.getByRole('textbox',{name:'Точка разреза X',exact:true}).fill(String(x));await solid.getByRole('textbox',{name:'Точка разреза Y',exact:true}).fill(String(y))}
 async function preview(){await page.waitForFunction(()=>document.querySelector('[data-preview="point-trim"]')?.getAttribute('points').trim().split(' ').length===49)}
 await begin();await page.waitForFunction(()=>window.__snapHeld)
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isEnabled(),false)
 assert.deepEqual(await exportDoc('pending.json'),before)
 await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>window.__snapTerminated),true)
 assert.deepEqual(await exportDoc('cancelled.json'),before)
 await page.evaluate(()=>window.__holdSnaps=false)
 await begin();await setPoint('8 mm','6 mm');await solid.getByRole('combobox',{name:'Сохранить конец',exact:true}).selectOption('end');await preview()
 assert.deepEqual(await exportDoc('preview.json'),before)
 await page.screenshot({path:path.join(directory,'point-trim-preview.png')})
 await setPoint('10 mm','0 mm')
 await solid.getByText('Это конец кривой. Выберите внутреннюю точку.',{exact:false}).first().waitFor()
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isEnabled(),false)
 assert.deepEqual(await exportDoc('endpoint-refused.json'),before)
 await setPoint('8 mm','6 mm');await preview()
 await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
 const changed=await exportDoc('changed.json'),curve=changed.curves[0].curve
 assert.equal(changed.curves[0].id,'arc');assert.equal(changed.curves[0].name,'Arc')
 const cut=curve.controlPoints[0];assert.ok(Math.abs(cut[0]-8)<1e-7&&Math.abs(cut[1]-6)<1e-7&&cut[2]===0)
 assert.deepEqual(curve.controlPoints.at(-1),[0,10,0]);assert.ok(curve.weights.some(w=>Math.abs(w-1)>1e-6))
 assert.deepEqual(changed.bodies,before.bodies);assert.deepEqual(changed.sketches,before.sketches)
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('undone.json'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('redone.json'),changed)
 await page.reload();await ready();assert.deepEqual(await exportDoc('reloaded.json'),changed)
 await page.evaluate(()=>window.__holdSnaps=false)
 await solid.getByRole('button',{name:'Arc',exact:true}).click()
 await solid.getByRole('status',{name:'curve-display',exact:true}).waitFor({state:'hidden'})
 await begin();await setPoint('6 mm','8 mm');await solid.getByRole('combobox',{name:'Сохранить конец',exact:true}).selectOption('start');await preview()
 await solid.locator('svg[aria-label="Холст тел 3D"]').focus();await page.keyboard.press('Enter')
 const startKept=await exportDoc('start-kept.json'),last=startKept.curves[0].curve.controlPoints.at(-1)
 assert.ok(Math.abs(last[0]-6)<1e-7&&Math.abs(last[1]-8)<1e-7)
 assert.deepEqual(startKept.curves[0].curve.controlPoints[0],changed.curves[0].curve.controlPoints[0])
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('start-undone.json'),changed)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('start-redone.json'),startKept)

 // A tilted rational arc tests screen selection independently of numeric XYZ input.
 const tilted=structuredClone(fixture)
 tilted.curves[0].curve.controlPoints=[[10,0,10],[10,10,15],[0,10,15]]
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'tilted.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(tilted))});await closeMenu();await ready()
 await solid.getByRole('button',{name:'Arc',exact:true}).click()
 await solid.getByRole('status',{name:'curve-display',exact:true}).waitFor({state:'hidden'})
 const tiltedBefore=await exportDoc('tilted-before.json')
 async function clickCurve(){
  const screen=await solid.locator('svg[aria-label="Холст тел 3D"] polyline[stroke="#ffc977"][stroke-width="3"]').evaluate(e=>{
   const points=e.getAttribute('points').trim().split(/\s+/).map(p=>p.split(',').map(Number)),p=points[Math.floor(points.length/2)],m=e.getScreenCTM()
   return {x:m.a*p[0]+m.c*p[1]+m.e,y:m.b*p[0]+m.d*p[1]+m.f}
  })
  await page.mouse.click(screen.x,screen.y)
 }
 await begin();await preview()
 await page.evaluate(()=>{window.__holdSnaps=true;window.__snapHeld=false;window.__snapTerminated=false})
 await clickCurve();await page.waitForFunction(()=>window.__snapHeld)
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isEnabled(),false)
 await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>window.__snapTerminated),true)
 assert.deepEqual(await exportDoc('screen-cancelled.json'),tiltedBefore)
 await page.evaluate(()=>window.__holdSnaps=false)
 const canvasBox=await solid.locator('svg[aria-label="Холст тел 3D"]').boundingBox()
 await page.mouse.move(canvasBox.x+35,canvasBox.y+35);await page.mouse.down({button:'right'});await page.mouse.move(canvasBox.x+75,canvasBox.y+55,{steps:5});await page.mouse.up({button:'right'})
 await begin();await clickCurve();await preview()
 await solid.locator('[data-testid="point-trim-picked"]').waitFor()
 assert.equal(await solid.getByRole('textbox',{name:'Точка разреза X',exact:true}).count(),0)
 assert.deepEqual(await exportDoc('screen-preview.json'),tiltedBefore)
 await solid.getByRole('button',{name:'Ввести координаты',exact:true}).click();await preview()
 const pickedZ=Number(await solid.getByRole('textbox',{name:'Точка разреза Z',exact:true}).inputValue())
 assert.ok(pickedZ>10&&pickedZ<15)
 await clickCurve();await preview();await solid.locator('[data-testid="point-trim-picked"]').waitFor()
 await page.screenshot({path:path.join(directory,'screen-trim-preview.png')})
 await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
 const screenResult=await exportDoc('screen-result.json'),screenCut=screenResult.curves[0].curve.controlPoints.at(-1)
 assert.ok(Math.abs(Math.hypot(screenCut[0],screenCut[1])-10)<1e-8)
 assert.ok(Math.abs(screenCut[2]-.5*screenCut[1]-10)<1e-8)
 assert.ok(Math.abs(screenCut[0]-Math.SQRT1_2*10)<.05&&Math.abs(screenCut[1]-Math.SQRT1_2*10)<.05)
 assert.equal(screenResult.curves[0].id,'arc')
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc('screen-undone.json'),tiltedBefore)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc('screen-redone.json'),screenResult)
 await page.reload();await ready();assert.deepEqual(await exportDoc('screen-reloaded.json'),screenResult)

 assert.deepEqual(errors,[])
 const report={screenPick:true,screenCut,screenCancel:true,rotatedCamera:true,numericModeSwitch:true,depthPreserved:true,browser:browser.version(),cancelledWorkerTerminated:true,preview:true,bothRetainedEnds:true,keyboardApply:true,endpointRefused:true,cut,identityPreserved:true,rationalWeights:true,undoRedo:true,reload:true}
 await writeFile(path.join(directory,'point-trim-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
