import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-framed-sweep')
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
 const curves=[
  {id:'profile',name:'Profile',curve:{degree:1,controlPoints:[[1,0,0],[1.2,0,0]],weights:[1,1],knots:[0,0,1,1]}},
  {id:'path',name:'Path',curve:{degree:2,controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1],knots:[0,0,0,1,1,1]}}
 ]
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'sweep.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],curves}))})
 await solid.getByRole('button',{name:'Path',exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:'Profile',exact:true}).click()
 await solid.getByRole('button',{name:'Path',exact:true}).click({modifiers:['Shift']})
 async function sweep(){
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  await search.fill('Sweep');await search.press('Enter')
  await solid.getByLabel('Sweep orientation',{exact:true}).selectOption('framed')
 }
 await sweep()
 await solid.getByLabel('Sweep sections',{exact:true}).fill('3')
 await solid.getByText('Измеренное отклонение превышает предел.',{exact:false}).waitFor()
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 await solid.getByLabel('Sweep sections',{exact:true}).fill('32')
 await solid.locator('[data-diagnostic="sweep-refinement"]').filter({hasText:'125'}).waitFor()
 await page.screenshot({path:path.join(directory,'sweep-preview.png')})
 await solid.getByRole('button',{name:'Esc',exact:true}).click()
 const canceled=await download('Скачать проект JSON','sweep-canceled.json')
 assert.equal(canceled.surfaces.length,0)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await sweep()
 await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
 const committed=await download('Скачать проект JSON','sweep-committed.json')
 assert.equal(committed.surfaces.length,1)
 assert.equal(committed.surfaces[0].name,'Framed sweep')
 assert.deepEqual(committed.curves,curves)
 const points=committed.surfaces[0].surface.controlPoints
 assert.equal(points[0].length,32)
 for(const point of points[1])assert.ok(Math.abs(Math.hypot(point[0],point[1])-1.2)<1e-12)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:'↶',exact:true}).click()
 const undone=await download('Скачать проект JSON','sweep-undone.json')
 assert.equal(undone.surfaces.length,0)
 const report={browser:browser.version(),framedSweep:true,refusal:true,cancel:true,undo:true,sourceCurvesPreserved:true,downloads}
 await writeFile(path.join(directory,'framed-sweep-browser.json'),JSON.stringify(report,null,2)+'\n')
 console.log(report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
