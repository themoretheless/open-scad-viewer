import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-periodic-surface')
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
 const points=Array.from({length:6},(_,i)=>Array.from({length:6},(_,j)=>{
  const u=i%4*Math.PI/2,v=j%4*Math.PI/2;return [(2+.5*Math.cos(v))*Math.cos(u),(2+.5*Math.cos(v))*Math.sin(u),.5*Math.sin(v)]
 }))
 const surface={id:'periodic-surface',name:'Periodic surface',segmentsU:16,segmentsV:16,surface:{degreeU:2,degreeV:2,knotsU:Array.from({length:9},(_,i)=>i),knotsV:Array.from({length:9},(_,i)=>i),controlPoints:points,weights:points.map((row,i)=>row.map((_,j)=>[1,.8,1.2,1][i%4]*[1,.9,1.1,1][j%4])),periodicU:true,periodicV:true}}
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'periodic-surface.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({version:1,sketches:[],bodies:[],surfaces:[surface]}))})
 await solid.getByRole('button',{name:'Periodic surface',exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:'Periodic surface',exact:true}).click()
 async function rebuild(){
  await solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}).click()
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  await search.fill('Rebuild surface');await search.press('Enter')
 }
 const axes=[]
 for(const axis of ['u','v']){
  await rebuild()
  await solid.getByLabel('Направление',{exact:true}).selectOption(axis)
  await solid.getByLabel('Новая степень',{exact:true}).fill('3')
  await solid.getByLabel('Управляющих точек',{exact:true}).fill('15')
  await solid.getByLabel('Допуск отклонения, мм',{exact:true}).fill('0')
  assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 await solid.getByText('Отклонение превышает допуск. Измените параметры или увеличьте допуск.',{exact:true}).first().waitFor()
  await solid.getByLabel('Допуск отклонения, мм',{exact:true}).fill('0.2')
  await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click({trial:true})
  const applyContrast=await solid.getByRole('button',{name:'Готово · Enter',exact:true}).evaluate(el=>{
   const style=getComputedStyle(el),canvas=document.createElement('canvas');canvas.width=canvas.height=1
   const ctx=canvas.getContext('2d')
   const luminance=color=>{ctx.clearRect(0,0,1,1);ctx.fillStyle=color;ctx.fillRect(0,0,1,1);const rgb=[...ctx.getImageData(0,0,1,1).data].slice(0,3).map(x=>{x/=255;return x<=.04045?x/12.92:((x+.055)/1.055)**2.4});return .2126*rgb[0]+.7152*rgb[1]+.0722*rgb[2]}
   const a=luminance(style.color),b=luminance(style.backgroundColor)
   return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)
  })
  assert.ok(applyContrast>=4.5,`Hovered Apply contrast is ${applyContrast}`)
  const boundText=await solid.locator('.operation-card output').textContent()
  const deviationUpperMm=Number(boundText.match(/: ([0-9.eE+-]+) mm/)[1])
  assert.ok(Number.isFinite(deviationUpperMm)&&deviationUpperMm<=.2)
  if(process.env.SOLID_GPU_HEADED==='1'){
   await solid.getByRole('status',{name:'surface-display',exact:true}).waitFor({state:'hidden'})
   const failure=await solid.locator('canvas.gpu-layer').getAttribute('data-gpu-error')
   if(failure){await writeFile(path.join(directory,'gpu-failure.json'),JSON.stringify({failure,axis},null,2));throw Error(failure)}
   await solid.locator('canvas.gpu-layer').waitFor({state:'visible'})
   assert.ok(await solid.locator('[data-preview-body]').count()>0)
   assert.ok(await solid.locator('[data-preview-body]').evaluateAll(nodes=>nodes.every(n=>getComputedStyle(n).fill==='rgba(0, 0, 0, 0)'&&getComputedStyle(n).stroke==='none')))
  }
  await page.screenshot({path:path.join(directory,`surface-${axis}-preview.png`)})
  await solid.getByRole('button',{name:'Esc',exact:true}).click()
  const canceled=await download('Скачать проект JSON',`surface-${axis}-canceled.json`)
  assert.deepEqual(canceled.surfaces,[surface])
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await rebuild()
  await solid.getByRole('button',{name:'Готово · Enter',exact:true}).click()
  const committed=await download('Скачать проект JSON',`surface-${axis}-committed.json`)
  assert.equal(committed.surfaces.length,1)
  const result=committed.surfaces[0]
  assert.equal(result.id,surface.id)
  assert.equal(result.surface.periodicU,true);assert.equal(result.surface.periodicV,true)
  assert.equal(axis==='u'?result.surface.controlPoints.length:result.surface.controlPoints[0].length,15)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await solid.getByRole('button',{name:'↶',exact:true}).click()
  const undone=await download('Скачать проект JSON',`surface-${axis}-undone.json`)
  assert.deepEqual(undone.surfaces,[surface])
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  axes.push({axis,deviationUpperMm,applyContrast,refusal:true,cancel:true,undo:true,identityPreserved:true})
 }
 const report={browser:browser.version(),periodicSurfaceRebuild:true,axes,downloads}
 await writeFile(path.join(directory,'periodic-surface-browser.json'),JSON.stringify(report,null,2)+'\n')
 console.log(report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
