import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-surface-match')
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
 const definition={degreeU:3,degreeV:3,knotsU:[2,2,2,2,5,5,5,5],knotsV:[-1,-1,-1,-1,3,3,3,3],controlPoints:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>[i,j,.15*i*i+.1*i*j+.2*j*j])),weights:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>1+.03*i+.02*j+.01*i*j))}
 const a={id:'reference',name:'Reference A',segmentsU:16,segmentsV:16,surface:definition}
 const b=structuredClone(a);b.id='edited';b.name='Edited B';b.surface.controlPoints.forEach(row=>row.forEach(p=>p[0]+=5))
 const original={version:1,sketches:[],bodies:[],surfaces:[a,b]}
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'surface-match.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 await solid.getByRole('button',{name:a.name,exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:a.name,exact:true}).click()
 await solid.getByRole('button',{name:b.name,exact:true}).click({modifiers:['Shift']})
 async function match(){
  await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}))
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  await search.fill('Match surfaces');await search.press('Enter')
  await solid.getByLabel('A · опорная',{exact:true}).selectOption(a.id)
  await solid.getByLabel('B · изменяемая',{exact:true}).selectOption(b.id)
 }
 const checks=[]
 for(const order of [1,2]){
  await match()
  await solid.getByLabel('Порядок',{exact:true}).selectOption(String(order))
  await solid.getByLabel('Допуск производных, мм',{exact:true}).fill('0')
  await solid.getByText('Ошибка производных превышает допуск. Измените масштаб или допуск.',{exact:true}).first().waitFor()
  const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
  assert.equal(await apply.isDisabled(),true)
  await solid.getByLabel('Допуск производных, мм',{exact:true}).fill('0.000001')
  await apply.waitFor();await apply.click({trial:true})
  const report=await solid.getByTestId('surface-match-report').textContent()
  assert.match(report,/Допуск подтверждён/)
  await page.screenshot({path:path.join(directory,`g${order}-preview.png`)})
  await activate(solid.getByRole('button',{name:'Esc',exact:true}))
  const canceled=await download('Скачать проект JSON',`g${order}-canceled.json`)
  assert.deepEqual(canceled.surfaces,original.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await match()
  await activate(apply)
  const committed=await download('Скачать проект JSON',`g${order}-committed.json`)
  assert.deepEqual(committed.surfaces[0],a)
  assert.equal(committed.surfaces[1].id,b.id)
  assert.equal(committed.surfaces[1].name,b.name)
  assert.notDeepEqual(committed.surfaces[1].surface,b.surface)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  await activate(solid.getByRole('button',{name:'↶',exact:true}))
  const undone=await download('Скачать проект JSON',`g${order}-undone.json`)
  assert.deepEqual(undone.surfaces,original.surfaces)
  if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
  checks.push({order,report,refusal:true,cancel:true,undo:true,identityPreserved:true})
 }
 const report={browser:browser.version(),checks,downloads,keyboard,tabPresses}
 await writeFile(path.join(directory,'surface-match-browser.json'),JSON.stringify(report,null,2)+'\n')
 console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
