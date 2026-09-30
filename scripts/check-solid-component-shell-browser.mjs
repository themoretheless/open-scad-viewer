import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-component-shell')
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
const errors=[],gpu=process.argv.includes('--gpu')
try{
 const {playwright}=await loadQualificationPlaywrightPackage();browser=await playwright.chromium.launch({headless:!gpu,args:gpu?['--enable-unsafe-webgpu']:[]})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.on('pageerror',e=>errors.push(String(e)))
 if(gpu)await page.addInitScript(()=>{
  window.qualificationGpuErrors=[]
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const device=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const result=await device(...args);result.addEventListener('uncapturederror',e=>window.qualificationGpuErrors.push(e.error.message));return result}}return adapter}
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'});if(gpu){await solid.locator('canvas.gpu-layer').waitFor({state:'visible'});assert.deepEqual(await page.evaluate(()=>window.qualificationGpuErrors),[])}}
 async function openMenu(){await ready();if(!await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 let lastDownload=0
 async function download(label,file){
  const wait=1100-(Date.now()-lastDownload);if(wait>0)await new Promise(resolve=>setTimeout(resolve,wait));lastDownload=Date.now()
  await openMenu();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:label,exact:true}).click();await(await pending).saveAs(path.join(directory,file));await closeMenu();return readFile(path.join(directory,file))
 }
 async function doc(name){return JSON.parse(await download('Скачать проект JSON',name+'.json'))}

 const fixture=path.resolve(process.env.CAD_COMPONENT_FIXTURE??'/tmp/cad-component-fixture/fixture.json')
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles(fixture);await closeMenu();await ready()
 async function command(name){
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter');await ready()
 }
 if(gpu)await solid.locator('canvas.gpu-layer').waitFor({state:'visible'})
 const before=await doc('before'),body=before.bodies[0]
 assert.equal(body.brep.bodies.length,2);assert.equal(body.brep.shells.length,3)
 const canvas=solid.locator('svg[aria-label="Холст тел 3D"]'),view=await canvas.getAttribute('viewBox'),box=await canvas.boundingBox();assert.ok(box)
 assert.equal(await canvas.locator('polygon[data-body]').count(),gpu?0:24,'closed components cull backfaces while the open shell remains two-sided')
 await command('Показывать B-rep гранёным');assert.equal(await canvas.locator('polygon[data-body]').count(),gpu?0:24)
 await command('Показывать B-rep гладким');assert.equal(await canvas.locator('polygon[data-body]').count(),gpu?0:24)
 await page.screenshot({path:path.join(directory,'mixed-shell-culling.png')})

 await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down({button:'middle'});await page.mouse.move(box.x+box.width/2+30,box.y+box.height/2+20);await page.waitForFunction(expected=>document.querySelectorAll('svg[aria-label="Холст тел 3D"] polygon[data-body]').length===expected,gpu?0:24);await page.mouse.up({button:'middle'})
 assert.equal(await canvas.locator('polygon[data-body]').count(),gpu?0:24);assert.notEqual(await canvas.getAttribute('viewBox'),view);assert.deepEqual(await doc('panned'),before)
 await page.mouse.down({button:'middle'});await page.mouse.move(box.x+box.width/2+45,box.y+box.height/2+30);await page.keyboard.press('Escape');await page.mouse.up({button:'middle'});assert.deepEqual(await doc('pan-cancelled'),before)

 await solid.getByRole('tab',{name:'Сцена',exact:true}).click();await solid.getByRole('button',{name:body.name,exact:true}).click()
 await solid.getByRole('button',{name:'Грани',exact:true}).click();await ready()
 const selector=solid.getByRole('combobox',{name:'Выбрать грань',exact:true})
 const face=await selector.locator('option').evaluateAll(options=>options.find(o=>o.value!=='-1'&&Math.abs(Number(o.textContent.split('·')[1]?.replace('mm','').split(',')[2])-4)<1e-7)?.value)
 assert.ok(face);await selector.selectOption(face)
 await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
 const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Push / Pull');await search.press('Enter')
 await solid.getByRole('textbox',{name:'Расстояние, мм',exact:true}).fill('1 mm');await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click();await ready()
 const after=await doc('after'),result=after.bodies[0].brep
 assert.equal(after.bodies[0].id,body.id);assert.equal(result.bodies.length,2);assert.equal(result.shells.length,3)
 assert.equal(result.shells.filter(s=>!s.closed).length,1)
 assert.deepEqual(result.topologyIds.bodies,body.brep.topologyIds.bodies)
 assert.deepEqual(result.topologyIds.shells,body.brep.topologyIds.shells)
 for(const [i,vertex] of body.brep.vertices.entries())if(vertex.point[0]<10||vertex.point[0]>=20){const j=result.topologyIds.vertices.indexOf(body.brep.topologyIds.vertices[i]);assert.ok(j>=0);assert.deepEqual(result.vertices[j],vertex)}
 assert.equal(Math.max(...result.vertices.filter(v=>v.point[0]>=10&&v.point[0]<20).map(v=>v.point[2])),5)
 await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await doc('undo'),before)
 await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await doc('redo'),after)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await solid.getByRole('button',{name:body.name,exact:true}).waitFor();assert.deepEqual(await doc('reload'),after)

 if(process.argv.includes('--opacity')){
  await solid.getByRole('button',{name:body.name,exact:true}).click()
  await solid.getByRole('tab',{name:'Свойства',exact:true}).click()
  const opacity=solid.getByRole('spinbutton',{name:'Непрозрачность',exact:true})
  const opaquePixels=gpu?createHash('sha256').update(await solid.locator('canvas.gpu-layer').screenshot()).digest('hex'):null
  await opacity.fill('0.35');await opacity.press('Tab');await ready()
  const transparent=await doc('transparent')
  assert.equal(transparent.bodies[0].material.opacity,.35)
  assert.deepEqual(transparent.bodies[0].mesh,after.bodies[0].mesh);assert.deepEqual(transparent.bodies[0].brep,after.bodies[0].brep)
  assert.equal(await canvas.locator('polygon[data-body]').count(),gpu?0:97,'CPU BSP splits this fixture into 97 display fragments')
  assert.ok(await canvas.locator('polygon[data-body]').evaluateAll(nodes=>nodes.every(n=>getComputedStyle(n).opacity==='0.35'&&getComputedStyle(n).stroke==='none')))
  if(gpu)assert.notEqual(createHash('sha256').update(await solid.locator('canvas.gpu-layer').screenshot()).digest('hex'),opaquePixels,'opacity must change GPU pixels')
  await page.screenshot({path:path.join(directory,'opacity.png')})
  await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await doc('opacity-undo'),after)
  await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await doc('opacity-redo'),transparent)
  await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await solid.getByRole('button',{name:body.name,exact:true}).waitFor();assert.deepEqual(await doc('opacity-reload'),transparent)
  if(process.argv.includes('--pick-fragment')){
   assert.equal(gpu,false,'fragment picking scenario uses CPU SVG')
   await solid.getByRole('button',{name:body.name,exact:true}).click()
   await solid.getByRole('button',{name:'Грани',exact:true}).click();await ready()
   let point=null
   for(let attempt=0;attempt<12&&!point;attempt++){
   point=await canvas.locator('polygon[data-body]').evaluateAll((nodes,mesh)=>{
    const counts=new Map();for(const n of nodes){const key=n.getAttribute('data-triangle');counts.set(key,(counts.get(key)??0)+1)}
    for(const n of nodes){
     const triangle=Number(n.getAttribute('data-triangle'))
     if(counts.get(String(triangle))<2)continue
     const indices=mesh.indices.slice(triangle*3,triangle*3+3)
     if(!indices.every(i=>mesh.positions[i*3]<20))continue
     const vertices=Array.from(n.points),local=new DOMPoint(vertices.reduce((v,p)=>v+p.x,0)/3,vertices.reduce((v,p)=>v+p.y,0)/3),screen=local.matrixTransform(n.getScreenCTM())
     if(document.elementFromPoint(screen.x,screen.y)===n)return {x:screen.x,y:screen.y,triangle,fragments:counts.get(String(triangle))}
    }
    return null
   },transparent.bodies[0].mesh)
   if(!point){const bounds=await canvas.boundingBox();const x=bounds.x+bounds.width*.8,y=bounds.y+bounds.height*.2;await page.mouse.move(x,y);await page.mouse.down({button:'right'});await page.mouse.move(x+60,y+15);await page.mouse.up({button:'right'});await ready()}
   }
   assert.ok(point,'a split fragment on a closed component must be visibly clickable')
   await page.mouse.click(point.x,point.y);await ready()
   const selected=Number(await solid.getByRole('combobox',{name:'Выбрать грань',exact:true}).inputValue());assert.ok(selected>=0)
   await command('Push / Pull')
   await solid.getByRole('textbox',{name:'Расстояние, мм',exact:true}).fill('1 mm');await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click();await ready()
   const edited=await doc('fragment-push')
   assert.equal(edited.bodies[0].id,transparent.bodies[0].id);assert.deepEqual(edited.bodies[0].material,transparent.bodies[0].material)
   assert.notDeepEqual(edited.bodies[0].brep,transparent.bodies[0].brep)
   await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await doc('fragment-push-undo'),transparent)
   await writeFile(path.join(directory,'fragment-pick.json'),JSON.stringify({point,selectedFace:selected,pushMm:1,undoExact:true},null,2))
  }
 }
 if(process.argv.includes('--measure-cpu-orbit')){
  assert.equal(gpu,false)
  const baseline=await doc('orbit-before'),bounds=await canvas.boundingBox()
  await page.evaluate(()=>{window.orbitFrames=[];window.orbitMeasuring=true;let last=performance.now();function frame(now){if(!window.orbitMeasuring)return;window.orbitFrames.push(now-last);last=now;requestAnimationFrame(frame)}requestAnimationFrame(frame)})
  const start=performance.now(),counts=[]
  await page.mouse.move(bounds.x+bounds.width*.7,bounds.y+bounds.height*.3);await page.mouse.down({button:'right'})
  for(let i=1;i<=30;i++){
   await page.mouse.move(bounds.x+bounds.width*.7+i*3,bounds.y+bounds.height*.3+Math.sin(i*.2)*20)
   await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(resolve)))
   counts.push(await canvas.locator('polygon[data-body]').count())
  }
  await page.mouse.up({button:'right'});await ready()
  const elapsed=performance.now()-start,frames=await page.evaluate(()=>{window.orbitMeasuring=false;return window.orbitFrames})
  assert.deepEqual(await doc('orbit-after'),baseline)
  const sorted=[...frames].sort((a,b)=>a-b),at=p=>sorted[Math.min(sorted.length-1,Math.floor(sorted.length*p))]
  await writeFile(path.join(directory,'cpu-orbit.json'),JSON.stringify({steps:30,elapsedMs:elapsed,rafSamples:frames.length,rafMs:{p50:at(.5),p95:at(.95),max:sorted.at(-1)},polygonCounts:{min:Math.min(...counts),max:Math.max(...counts)},documentUnchanged:true,scope:'Single mixed-shell CPU scene; includes browser automation and RAF pacing; not a large-scene FPS acceptance.'},null,2))
 }
 if(gpu){await solid.locator('canvas.gpu-layer').waitFor({state:'visible'});assert.deepEqual(await page.evaluate(()=>window.qualificationGpuErrors),[])}
 await page.screenshot({path:path.join(directory,'result.png')})
 assert.deepEqual(errors,[]);await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,gpu,bodies:2,shells:3,standaloneShells:1,pushMm:1,unchangedNeighbors:true,panPreservesDocument:true,panCancellation:true,opacityChecked:process.argv.includes('--opacity'),undoRedo:true,reload:true,errors},null,2))
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
