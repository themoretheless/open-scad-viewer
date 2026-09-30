import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-step-mixed')
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
const errors=[],parts=[]
try{
 const {playwright}=await loadQualificationPlaywrightPackage();browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.on('pageerror',e=>errors.push(String(e)))
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 async function openMenu(){await ready();if(!await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 let lastDownload=0
 async function download(label,file){
  const wait=1100-(Date.now()-lastDownload);if(wait>0)await new Promise(resolve=>setTimeout(resolve,wait));lastDownload=Date.now()
  await openMenu();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:label,exact:true}).click();await(await pending).saveAs(path.join(directory,file));await closeMenu();return readFile(path.join(directory,file))
 }
 async function doc(name){return JSON.parse(await download('Скачать проект JSON',name+'.json'))}
 const fixtures=path.resolve(process.env.CAD_MIXED_FIXTURES??'/tmp/cad-mixed-final')
 const sourceManifest=JSON.parse(await readFile(path.join(fixtures,'manifest.json'),'utf8'))
 for(const name of ['mixed','placed']){
  await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[]}))});await closeMenu();await ready()
  await openMenu();await solid.locator('input[accept=".step,.stp"]').setInputFiles(path.join(fixtures,name+'-input.step'))
  await page.waitForFunction(()=>{const input=document.querySelector('input[accept=".step,.stp"]');return input&&input.value===''});await closeMenu();await ready()
  const before=await doc(name+'-before');assert.equal(before.bodies.length,1);assert.equal(before.bodies[0].brep.bodies.length,2)
  const body=before.bodies[0]
  await solid.getByRole('tab',{name:'Сцена',exact:true}).click();await solid.getByRole('button',{name:body.name,exact:true}).click()
  await solid.getByRole('button',{name:'Грани',exact:true}).click();await ready()
  const selector=solid.getByRole('combobox',{name:'Выбрать грань',exact:true})
  const face=await selector.locator('option').evaluateAll(options=>{const o=options.find(o=>o.value!=='-1'&&Math.abs(Number(o.textContent.split('·')[1]?.replace('mm','').split(',')[2])-101.6)<1e-7);return o?.value})
  assert.ok(face);await selector.selectOption(face)
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill('Push / Pull');await search.press('Enter')
  const amount=solid.getByRole('textbox',{name:'Расстояние, мм',exact:true});await amount.fill('1 mm')
  await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click();await ready()
  const after=await doc(name+'-after');assert.equal(after.bodies[0].id,body.id)
  assert.deepEqual(after.bodies[0].brep.topologyIds.bodies,body.brep.topologyIds.bodies)
  const vertices=after.bodies[0].brep.vertices
  assert.ok(Math.abs(Math.max(...vertices.map(v=>v.point[2]))-102.6)<1e-7)
  for(let i=0;i<body.brep.vertices.length;i++)if(body.brep.vertices[i].point[0]<3){const id=body.brep.topologyIds.vertices[i],j=after.bodies[0].brep.topologyIds.vertices.indexOf(id);assert.ok(j>=0);assert.deepEqual(vertices[j],body.brep.vertices[i])}
  await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await doc(name+'-undo'),before)
  await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await doc(name+'-redo'),after)
  await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await solid.getByRole('button',{name:body.name,exact:true}).waitFor();assert.deepEqual(await doc(name+'-reload'),after)
  await solid.getByRole('button',{name:body.name,exact:true}).click()
  const file=name+'-browser.step',bytes=await download('STEP выбранного тела · текущая геометрия',file)
  parts.push({name,file,sha256:createHash('sha256').update(bytes).digest('hex'),expected:sourceManifest.parts.find(p=>p.name===name+'-edited').expected})
  await page.screenshot({path:path.join(directory,name+'.png')})
 }
 assert.deepEqual(errors,[])
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({...sourceManifest,parts},null,2))
 await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,cases:['mixed','placed'],pushMm:1,undoRedo:true,reload:true,errors},null,2))
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
