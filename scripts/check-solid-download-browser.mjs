import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-blender-qualification')
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
let browser
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
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
 const snapshots=[],stepParts=[]
 for(const width of [2,7]){
  await openMenu()
  await solid.locator('input[accept=".json,application/json"]').setInputFiles(path.join(directory,`width-${width}.json`))
  await solid.getByRole('button',{name:'Qualified part',exact:true}).waitFor()
  // Import is asynchronous; inspect the serialized UI export before testing bridge output.
  let saved
  for(let attempt=0;attempt<10;attempt++){
   saved=await download('Скачать проект JSON',`project-${width}.json`)
   const body=saved.bodies.find(b=>b.id==='source')
   if(body && Math.max(...body.mesh.positions.filter((_,i)=>i%3===0))===width)break
   if(attempt===9)throw Error('Imported geometry did not become current')
  }
  const snapshot=await download('Экспорт для Blender',`browser-${width}.osv-blender.json`)
  assert.equal(snapshot.schema,'openscad-viewer/blender-1')
  assert.deepEqual(snapshot.bodies.map(b=>b.id),['source','instance'])
  assert.equal(Math.max(...snapshot.bodies[0].mesh.positions.filter((_,i)=>i%3===0)),width)
  snapshots.push(snapshot)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'Qualified part',exact:true}))
  const file=`browser-${width}.step`
  const step=await download('STEP выбранного тела · текущая геометрия',file,false)
  assert.ok(step.startsWith('ISO-10303-21;'))
  stepParts.push({name:`Browser box ${width}×3×4`,file,sha256:createHash('sha256').update(step).digest('hex'),expected:{boundsMm:[[0,0,0],[width,3,4]],volumeMm3:width*3*4}})
 }
 assert.equal(snapshots[0].projectId,snapshots[1].projectId)
 const saved=await download('Скачать проект JSON','project-final.json')
 assert.equal(saved.blenderProjectId,snapshots[0].projectId)
 await page.reload()
 const recovered=await download('Экспорт для Blender','browser-recovered.osv-blender.json')
 assert.deepEqual(recovered,snapshots[1])
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'Qualified part',exact:true}))
 await activate(solid.getByRole('tab',{name:'Свойства',exact:true}))
 const scale=solid.getByRole('spinbutton',{name:'Масштаб',exact:true})
 if(keyboard){await tabTo(scale);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.press('2')}else await scale.fill('2')
 await activate(solid.getByRole('button',{name:'Применить',exact:true}))
 const editedStep=await download('STEP выбранного тела · текущая геометрия','browser-edited.step',false)
 stepParts.push({name:'Browser UI scale 2',file:'browser-edited.step',sha256:createHash('sha256').update(editedStep).digest('hex'),expected:{boundsMm:[[-3.5,-1.5,-2],[10.5,4.5,6]],volumeMm3:672}})
 const svg=await download('SVG выбранного тела · проекция XY','browser-edited.svg',false)
 const svgDimensions=await page.evaluate(text=>{
  const doc=new DOMParser().parseFromString(text,'image/svg+xml')
  if(doc.querySelector('parsererror'))throw Error('Invalid downloaded SVG')
  const node=document.importNode(doc.documentElement,true);document.body.append(node)
  const bounds=node.querySelector('path').getBBox()
  const result={width:node.getAttribute('width'),height:node.getAttribute('height'),paths:node.querySelectorAll('path').length,bounds:[bounds.x,bounds.y,bounds.width,bounds.height]}
  node.remove();return result
 },svg)
 assert.deepEqual(svgDimensions,{width:'14mm',height:'6mm',paths:1,bounds:[0,0,14,6]})
 await openMenu()
 await solid.locator('input[accept=".svg,image/svg+xml"]').setInputFiles(path.join(directory,'browser-edited.svg'))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
 await solid.getByRole('button',{name:'browser-edited.svg · 1',exact:true}).waitFor()
 const importedSvg=await download('Скачать проект JSON','svg-imported.json')
 assert.equal(importedSvg.sketches.length,1)
 assert.equal(importedSvg.sketches[0].closed,true)
 const points=importedSvg.sketches[0].points
 assert.equal(Math.max(...points.map(p=>p[0]))-Math.min(...points.map(p=>p[0])),14)
 assert.equal(Math.max(...points.map(p=>p[1]))-Math.min(...points.map(p=>p[1])),6)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undoneSvg=await download('Скачать проект JSON','svg-undone.json')
 assert.equal(undoneSvg.sketches.length,0)
 assert.equal(undoneSvg.bodies.length,2)
 const holeSvg='<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M0 0H20V20H0Z M5 5H15V15H5Z"/></svg>'
 await openMenu()
 await solid.locator('input[accept=".svg,image/svg+xml"]').setInputFiles({name:'hole.svg',mimeType:'image/svg+xml',buffer:Buffer.from(holeSvg)})
 await solid.getByRole('button',{name:'hole.svg · 2',exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'Инструменты',exact:true}))
 await activate(solid.getByRole('button',{name:'Выдавить · E',exact:true}))
 const height=solid.getByLabel('Высота, мм',{exact:true})
 if(keyboard){await tabTo(height);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.press('2')}else await height.fill('2')
 await activate(solid.getByRole('button',{name:'Готово · Enter',exact:true}))
 const extruded=await download('Скачать проект JSON','svg-hole-extruded.json')
 const added=extruded.bodies.filter(body=>!undoneSvg.bodies.some(old=>old.id===body.id))
 assert.equal(added.length,1)
 const {positions:p,indices:ix}=added[0].mesh
 let volume6=0
 for(let i=0;i<ix.length;i+=3){
  const a=ix[i]*3,b=ix[i+1]*3,c=ix[i+2]*3
  volume6+=p[a]*(p[b+1]*p[c+2]-p[b+2]*p[c+1])+p[a+1]*(p[b+2]*p[c]-p[b]*p[c+2])+p[a+2]*(p[b]*p[c+1]-p[b+1]*p[c])
 }
 const holeVolume=Math.abs(volume6/6)
 assert.ok(Math.abs(holeVolume-600)<1e-5,`Hole extrusion volume: ${holeVolume}`)
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:stepParts},null,2)+'\n')
 const assemblyDirectory=process.argv.find(arg=>arg.startsWith('--assembly='))?.slice(11)
 if(assemblyDirectory){
  await openMenu()
  await solid.locator('input[accept=".json,application/json"]').setInputFiles(path.join(assemblyDirectory,'scene.json'))
  await solid.getByRole('button',{name:"Кронштейн 'A' #1",exact:true}).waitFor()
  const assembly=await download('STEP сборки · тела и группы','browser-assembly.step',false)
  await writeFile(path.join(assemblyDirectory,'scene.step'),assembly)
 }
 const report={assemblyDownloaded:!!assemblyDirectory,holeExtrusionVolumeMm3:holeVolume,svgImported:true,svgUndo:true,svgDimensions,tabPresses,sequentialTabNavigation:keyboard,escapeRestoresFileFocus:keyboard,interaction:keyboard?'keyboard':'mouse',theme,uiScaleEdit:2,stepDownloads:stepParts.length,browser:browser.version(),downloads,sourceWidthsMm:[2,7],stableProjectId:true,reloadPreserved:true}
 await writeFile(path.join(directory,'browser-download-verification.json'),JSON.stringify(report,null,2)+'\n')
 console.log('SOLID_BROWSER_DOWNLOAD_VERIFIED',report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
