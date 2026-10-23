import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile,readdir} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-seam-preparation')
const keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
assert.ok(['system','dark','light','nord','solarized'].includes(theme))
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{
 try {
  const url=new URL(req.url,'http://localhost'),file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
  if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}
  if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const chunks=(await readdir(path.join(root,'assets'))).filter(n=>/^DirectModeler-[^/]+\.js$/.test(n));assert.equal(chunks.length,1)
const digest=async file=>createHash('sha256').update(await readFile(file)).digest('hex')
const artifacts={directModeler:{file:chunks[0],sha256:await digest(path.join(root,'assets',chunks[0]))},wasmSha256:await digest(path.join(root,'wasm/geometry-kernel.wasm')),sourceSha256:await digest('src/features/DirectModeler.vue')}
const errors=[]
let browser,page
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})
 const origin=`http://127.0.0.1:${server.address().port}`
 await page.goto(origin)
 await page.getByRole('combobox',{name:'Тема',exact:true}).selectOption(theme)
 let tabPresses=0
 async function tabTo(locator){
  for(let i=0;i<1500;i++){
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
 const a={id:'profile-a',name:'Open profile',closed:false,points:[[0,0],[10,0],[10,10],[0,10],[0,.1]]}
 const crossing=process.argv.includes('--crossing')
 if(crossing)a.points=[[0,0],[10,10],[0,10],[10,0],[0,0]]
 const original={version:1,sketches:[a],bodies:[]}
 await openMenu()
 await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'profile.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 await solid.getByRole('button',{name:a.name,exact:true}).waitFor()
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:a.name,exact:true}))
 const pane=solid.getByRole('region',{name:'2D — эскизы',exact:true})
 await activate(pane.getByRole('button',{name:'Вписать',exact:true}))
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name)}else await search.fill(name);await page.keyboard.press('Enter')}
 // A real worker refusal, generated from incompatible sketch planes.
 const bad={version:1,sketches:[{id:'bad-a',name:'First plane',closed:false,points:[[0,0],[10,0]]},{id:'bad-b',name:'Second plane',closed:false,points:[[10,0],[0,0]],plane:{origin:[0,0,1],u:[1,0,0],v:[0,1,0]}}],bodies:[]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'planes.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(bad))})
 await solid.getByRole('button',{name:'First plane',exact:true}).waitFor();if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'First plane',exact:true}))
 if(keyboard){const second=solid.getByRole('button',{name:'Second plane',exact:true});await tabTo(second);await page.keyboard.press('Shift+Enter')}else await solid.getByRole('button',{name:'Second plane',exact:true}).click({modifiers:['Shift']})
 const badBefore=await download('Скачать проект JSON','plane-before.json');if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Prepare profile')
 const refusal=solid.getByRole('alert').filter({hasText:'Все линии должны использовать одну плоскость эскиза.'})
 await refusal.waitFor();assert.match(await refusal.innerText(),/First plane, Second plane:/)
 assert.equal(await solid.getByRole('button',{name:'Готово · Enter',exact:true}).isDisabled(),true)
 assert.equal((await solid.innerText()).includes('Profile inputs must use'),false)
 await page.screenshot({path:path.join(directory,'plane-refusal.png')})
 await activate(solid.getByRole('button',{name:'Повторить вычисление',exact:true}));await refusal.waitFor()
 await page.keyboard.press('Escape')
 const failed=await download('Скачать проект JSON','plane-canceled.json');assert.deepEqual(failed,badBefore)
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'profile.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 await solid.getByRole('button',{name:a.name,exact:true}).waitFor();if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:a.name,exact:true}))
 const before=await download('Скачать проект JSON','profile-before.json');if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Prepare profile')
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true}),tolerance=solid.getByLabel('Допуск разрыва, мм',{exact:true})
 async function inputTolerance(value){if(keyboard){await tabTo(tolerance);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await tolerance.fill(value)}
 assert.equal(await apply.isDisabled(),true)
 if(crossing){
  const edges=solid.locator('[data-diagnostic="profile-segment"]');await edges.first().waitFor()
  assert.equal(await edges.count(),2)
  assert.deepEqual(await edges.evaluateAll(nodes=>nodes.map(n=>n.getAttribute('points'))),['0,0 10,-10','0,-10 10,0'])
  const proof=await solid.getByRole('alert').filter({hasText:'Отмеченные сегменты пересекаются'}).textContent();assert.match(proof,/сегменты пересекаются/)
  await page.screenshot({path:path.join(directory,'profile-intersection.png')})
  await activate(solid.getByRole('button',{name:'Esc',exact:true}))
  const canceled=await download('Скачать проект JSON','profile-canceled.json');assert.deepEqual(canceled,before)
  assert.deepEqual(errors,[]);assert.equal(await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible'),true)
 const report={artifacts,nativePlaneRefusal:true,retry:true,browser:browser.version(),proof,locatedSegments:[0,2],refused:true,cancel:true,keyboard,tabPresses}
  await writeFile(path.join(directory,'profile-diagnostics-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
 }else{
 await solid.locator('[data-diagnostic="profile-preparation"] circle').first().waitFor()
 assert.equal(await solid.locator('[data-diagnostic="profile-preparation"] circle').count(),2)
 await page.screenshot({path:path.join(directory,'profile-gap.png')})
 await inputTolerance('invalid');assert.equal(await apply.isDisabled(),true)
 await inputTolerance('0.011 cm');await apply.click({trial:true});assert.equal(await tolerance.inputValue(),'0.011 cm')
 assert.equal(await solid.locator('[data-preview="prepared-profile"]').count(),1)
 const proof=await solid.getByTestId('profile-preparation-report').textContent();assert.match(proof,/Контур замкнут/)
 await page.screenshot({path:path.join(directory,'profile-preview.png')})
 await activate(solid.getByRole('button',{name:'Esc',exact:true}))
 const canceled=await download('Скачать проект JSON','profile-canceled.json');assert.deepEqual(canceled,before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await command('Prepare profile');await activate(apply)
 const committed=await download('Скачать проект JSON','profile-committed.json')
 assert.deepEqual(committed.sketches,[{...a,closed:true}])
 const points=committed.sketches[0].points
 await writeFile(path.join(directory,'browser-oracle-fixtures.json'),JSON.stringify([{chains:[a.points],tolerance:.011*10,result:{accepted:true,points,connectors:[{a:a.points.at(-1),b:a.points[0]}]}}]))
 const area=Math.abs(points.reduce((sum,p,i)=>{const q=points[(i+1)%points.length];return sum+p[0]*q[1]-p[1]*q[0]},0))/2
 assert.equal(area,100)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↶',exact:true}))
 const undone=await download('Скачать проект JSON','profile-undone.json');assert.deepEqual(undone,before)
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 await activate(solid.getByRole('button',{name:'↷',exact:true}))
 const redone=await download('Скачать проект JSON','profile-redone.json');assert.deepEqual(redone,committed)
 await page.reload();await solid.getByRole('button',{name:a.name,exact:true}).waitFor();await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 const reloaded=await download('Скачать проект JSON','profile-reloaded.json');assert.deepEqual(reloaded,committed)
 assert.deepEqual(errors,[]);assert.equal(await solid.locator('.gpu-layer').evaluate(c=>c.style.visibility==='visible'),true)
 const report={artifacts,nativePlaneRefusal:true,retry:true,browser:browser.version(),proof,area,refusal:true,cancel:true,undo:true,redo:true,reloadExact:true,completeDocumentPreserved:true,identityAndCoordinatesPreserved:true,downloads,keyboard,tabPresses}
 await writeFile(path.join(directory,'profile-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
 }
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
