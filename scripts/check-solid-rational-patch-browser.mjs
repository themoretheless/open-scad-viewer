import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-rational-patch')
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
 const curves=controlPoints.map((points,i)=>({id:`boundary-${i}`,name:`Boundary ${i}`,curve:{degree:points.length-1,controlPoints:points,knots:[...Array(points.length).fill(0),...Array(points.length).fill(1)],weights:i<2?[1,Math.SQRT1_2,1]:[1,1]}}))
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'rational-patch.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves}))})
 await solid.getByRole('button',{name:'Boundary 3',exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 for(let i=0;i<4;i++)await solid.getByRole('button',{name:`Boundary ${i}`,exact:true}).click({modifiers:i?['Shift']:[]})
 async function patch(){
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  await search.fill('Coons patch');await search.press('Enter')
 }
 await patch()
 await solid.getByRole('button',{name:'Esc',exact:true}).click()
 const canceled=await download('Скачать проект JSON','patch-canceled.json')
 assert.equal(canceled.surfaces.length,0)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await patch()
 await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
 const committed=await download('Скачать проект JSON','patch-committed.json')
 assert.equal(committed.surfaces.length,1)
 assert.deepEqual(committed.curves,curves)
 assert.ok(committed.surfaces[0].surface.weights.some(row=>row.some(w=>w!==1)))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:'↶',exact:true}).click()
 const undone=await download('Скачать проект JSON','patch-undone.json')
 assert.equal(undone.surfaces.length,0)
 const invalid=structuredClone(curves);invalid[0].curve.weights[2]=.5
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'incompatible.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves:invalid}))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 for(let i=0;i<4;i++)await solid.getByRole('button',{name:`Boundary ${i}`,exact:true}).click({modifiers:i?['Shift']:[]})
 await patch()
 await solid.getByText('Веса в углах границ несовместимы.',{exact:false}).waitFor()
 const refused=await download('Скачать проект JSON','patch-refused.json')
 assert.equal(refused.surfaces.length,0)
 assert.deepEqual(refused.curves,invalid)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:'Esc',exact:true}).click()
 const gap=structuredClone(curves);gap[2].curve.controlPoints[0][0]=1.2
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'gap.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves:gap}))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 for(let i=0;i<4;i++)await solid.getByRole('button',{name:`Boundary ${i}`,exact:true}).click({modifiers:i?['Shift']:[]})
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

 const report={gpuAlignment:process.env.SOLID_GPU_HEADED==='1',narrowLayout:true,gapEndpointMarkers:2,localizedRefusal:true,browser:browser.version(),rationalPatch:true,cancel:true,undo:true,sourceCurvesPreserved:true,downloads}
 await writeFile(path.join(directory,'rational-patch-browser.json'),JSON.stringify(report,null,2)+'\n')
 console.log(report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
