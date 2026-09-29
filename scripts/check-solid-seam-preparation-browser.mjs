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
 const definition={degreeU:3,degreeV:3,knotsU:[0,0,0,0,1,1,1,1],knotsV:[0,0,0,0,1,1,1,1],controlPoints:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>[i,j,.15*i*i+.1*i*j+.2*j*j])),weights:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>1+.03*i+.02*j+.01*i*j))}
 const a={id:'prepare-a',name:'Prepare A',segmentsU:16,segmentsV:16,surface:definition}
 const b=structuredClone(a);b.id='prepare-b';b.name='Prepare B';b.surface.degreeV=2;b.surface.knotsV=[0,0,0,1,1,1];b.surface.controlPoints=b.surface.controlPoints.map(row=>row.slice(0,3));b.surface.weights=b.surface.weights.map(row=>row.slice(0,3))
 const periodic=process.argv.includes('--periodic'),mixed=process.argv.includes('--mixed')
 if(periodic||mixed){
  const ring=(degree,unique)=>({degreeU:3,degreeV:degree,knotsU:[0,0,0,0,1,1,1,1],knotsV:Array.from({length:unique+2*degree+1},(_,i)=>i),controlPoints:Array.from({length:4},(_,i)=>Array.from({length:unique+degree},(_,j)=>{const t=j%unique*Math.PI*2/unique;return [i,Math.cos(t),Math.sin(t)]})),weights:Array.from({length:4},(_,i)=>Array.from({length:unique+degree},(_,j)=>1+.02*i+.01*(j%unique))),periodicU:false,periodicV:true})
  a.surface=ring(2,4);if(!mixed)b.surface=ring(3,8)
 }
 const original={version:1,sketches:[],bodies:[],surfaces:[a,b]}
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'prepare.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 await solid.getByRole('button',{name:a.name,exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await solid.getByRole('button',{name:a.name,exact:true}).click();await solid.getByRole('button',{name:b.name,exact:true}).click({modifiers:['Shift']})
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});await search.fill(name);await search.press('Enter')}
 await command('Prepare surface boundaries')
 if(mixed){assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true);await solid.getByLabel('Снять периодическую связь',{exact:true}).check()}
 await solid.getByLabel('A · первая',{exact:true}).selectOption(a.id);await solid.getByLabel('B · вторая',{exact:true}).selectOption(b.id)
 await solid.getByLabel('Обратить направление B',{exact:true}).check()
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await solid.getByLabel('Допуск подготовки, мм',{exact:true}).fill('invalid')
 assert.equal(await apply.isDisabled(),true)
 await solid.getByLabel('Допуск подготовки, мм',{exact:true}).fill('0 mm')
 await solid.getByText('Отклонение подготовки превышает допуск. Увеличьте допуск или измените входы.',{exact:true}).first().waitFor()
 assert.equal(await apply.isDisabled(),true)
 await solid.getByLabel('Допуск подготовки, мм',{exact:true}).fill('0.000001 mm')
 await apply.click({trial:true})
 const proof=await solid.getByTestId('surface-preparation-report').textContent();assert.match(proof,/Допуск подтверждён/)
 await page.screenshot({path:path.join(directory,'preparation-preview.png')})
 await activate(solid.getByRole('button',{name:'Esc',exact:true}))
 const canceled=await download('Скачать проект JSON','preparation-canceled.json');assert.deepEqual(canceled.surfaces,original.surfaces)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Prepare surface boundaries');await activate(apply)
 const committed=await download('Скачать проект JSON','preparation-committed.json')
 assert.equal(committed.surfaces[0].id,a.id);assert.equal(committed.surfaces[1].id,b.id)
 assert.equal(committed.surfaces[1].surface.degreeV,Math.max(a.surface.degreeV,b.surface.degreeV))
 if(mixed){assert.equal(committed.surfaces[0].surface.periodicV,false);assert.equal(committed.surfaces[1].surface.periodicV,false)}
 assert.deepEqual(committed.surfaces[0].surface.knotsV,committed.surfaces[1].surface.knotsV)
 if(periodic)for(const item of committed.surfaces){assert.equal(item.surface.periodicV,true);for(const row of item.surface.controlPoints)assert.deepEqual(row.slice(0,3),row.slice(-3))}
 const bounds=[...proof.matchAll(/[AB] ≤ ([0-9.eE+-]+) mm/g)].map(m=>Number(m[1])*1.001)
 await writeFile(path.join(directory,'browser-oracle-fixtures.json'),JSON.stringify(original.surfaces.map((item,i)=>({source:item.surface,target:committed.surfaces[i].surface,reverse:i===1,bound:bounds[i]}))))
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Match surfaces')
 assert.equal(await solid.getByLabel('Обратить направление B',{exact:true}).isChecked(),false)
 await apply.click({trial:true})
 await activate(solid.getByRole('button',{name:'Esc',exact:true}))
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','preparation-undone.json');assert.deepEqual(undone.surfaces,original.surfaces)
 const report={browser:browser.version(),periodic,mixed,proof,refusal:true,cancel:true,undo:true,identityPreserved:true,reversalReset:true,downloads,keyboard,tabPresses}
 await writeFile(path.join(directory,'preparation-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
