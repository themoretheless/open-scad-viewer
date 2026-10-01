import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
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
const renderErrors=[]
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true})
 page.on('pageerror',e=>renderErrors.push(String(e)));page.on('console',m=>{if(m.type()==='error')renderErrors.push(m.text())})
 await page.addInitScript(()=>{
  if(!navigator.gpu)return
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const make=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const device=await make(...args);window.__qualificationGpuDevice=device;window.__gpuValidationErrors??=[];device.addEventListener('uncapturederror',event=>window.__gpuValidationErrors.push({message:event.error.message,width:document.querySelector('.gpu-layer')?.width,height:document.querySelector('.gpu-layer')?.height}));return device}}return adapter}
 })
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
 const original={version:1,sketches:[],bodies:[]}
 await openMenu();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'empty.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(original))})
 if(await menu.evaluate(e=>e.parentElement.open))await activate(menu)
 async function command(name){await activate(solid.getByRole('button',{name:'Команда… Ctrl K',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}else{await search.fill(name);await search.press('Enter')}}
 async function input(locator,value){if(keyboard){await tabTo(locator);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await locator.fill(value)}
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 await command('Box')
 await solid.locator('[data-body]').first().waitFor({state:'visible'})
 await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'})
 await command('Create linked instance');await activate(apply)
 await command('Select instance source')
 async function snapshot(file){const d=await download('Скачать проект JSON',file);if(await menu.evaluate(e=>e.parentElement.open))await activate(menu);return d}
 async function historyRoundtrip(before,after,key){
  for(const [button,expected,file] of [['↶',before,key+'-undo.json'],['↷',after,key+'-redo.json']]){
   await activate(solid.getByRole('button',{name:button,exact:true}));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});assert.deepEqual(await snapshot(file),expected)
  }
 }
 const initial=await snapshot('initial.json'),source=initial.bodies.find(b=>!b.instance),linked=initial.bodies.find(b=>b.instance)
 await command('Delete')
 await solid.getByRole('alert').filter({hasText:'Нельзя удалить источник «'+source.name+'»: остаются связанные экземпляры — 1'}).waitFor()
 await page.screenshot({path:path.join(directory,'blocked-source-delete.png')})
 assert.deepEqual(await snapshot('blocked-source-delete.json'),initial)
 await activate(solid.getByRole('button',{name:'Новая группа',exact:true}))
 await solid.getByText('Изменяется группа. Esc — отменить.',{exact:true}).waitFor({state:'hidden'})
 const emptyGroup=await snapshot('empty-group.json'),group=emptyGroup.groups.at(-1).name
 await historyRoundtrip(initial,emptyGroup,'group-create')
 await activate(solid.getByRole('button',{name:'Активная группа: '+group,exact:true}))
 await activate(solid.getByRole('button',{name:source.name,exact:true}))
 await activate(solid.getByRole('button',{name:'Перенести выбор',exact:true}))
 await solid.getByText('Изменяется группа. Esc — отменить.',{exact:true}).waitFor({state:'hidden'})
 const grouped=await snapshot('grouped.json')
 assert.equal(grouped.bodies.find(b=>b.id===source.id).group,group)
 assert.deepEqual(grouped.bodies.find(b=>b.id===linked.id),initial.bodies.find(b=>b.id===linked.id))
 await historyRoundtrip(emptyGroup,grouped,'group-move')
 await activate(solid.getByRole('button',{name:'Удалить группу '+group,exact:true}))
 await solid.getByRole('alert').filter({hasText:'Нельзя удалить источник «'+source.name}).waitFor()
 assert.deepEqual(await snapshot('blocked-group-delete.json'),grouped)
 await activate(solid.getByRole('button',{name:'Заблокировать: '+source.name,exact:true}))
 assert.equal(await solid.getByRole('button',{name:source.name,exact:true}).isDisabled(),true)
 await activate(solid.getByRole('button',{name:'Удалить группу '+group,exact:true}))
 await solid.getByRole('alert').filter({hasText:'Сначала разблокируйте объекты группы'}).waitFor()
 assert.deepEqual(await snapshot('blocked-locked-group-delete.json'),grouped)
 await activate(solid.getByRole('button',{name:'Скрыть: '+linked.name,exact:true}))
 assert.equal(await solid.getByRole('button',{name:linked.name,exact:true}).isDisabled(),true)
 await activate(solid.getByRole('button',{name:'Изолировать группу: '+group,exact:true}))
 assert.equal(await solid.getByRole('button',{name:'Выйти из изоляции',exact:true}).getAttribute('aria-pressed'),'true')
 const preferences=await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-workspace-v1')))
 assert.ok(preferences.hidden.includes(linked.id));assert.ok(preferences.locked.includes(source.id));assert.ok(preferences.isolated.includes(source.id));assert.equal(preferences.group,group)
 await page.screenshot({path:path.join(directory,'group-workspace.png')})
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload()
 await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 await solid.getByRole('button',{name:'Разблокировать: '+source.name,exact:true}).waitFor()
 assert.equal(await solid.getByRole('button',{name:source.name,exact:true}).isDisabled(),true)
 assert.equal(await solid.getByRole('button',{name:linked.name,exact:true}).isDisabled(),true)
 assert.equal(await solid.getByRole('button',{name:'Выйти из изоляции',exact:true}).getAttribute('aria-pressed'),'true')
 assert.equal(await solid.getByRole('button',{name:'Активная группа: '+group,exact:true}).getAttribute('aria-pressed'),'true')
 assert.deepEqual(await snapshot('workspace-reloaded.json'),grouped)
 await activate(solid.getByRole('button',{name:'Разблокировать: '+source.name,exact:true}))
 await activate(solid.getByRole('button',{name:/^Показать всё/}))
 await activate(solid.getByRole('button',{name:'Выйти из изоляции',exact:true}))
 await activate(solid.getByRole('button',{name:linked.name,exact:true}));await command('Make independent')
 await solid.getByText('Создаётся независимое тело. Esc — отменить.',{exact:true}).waitFor({state:'hidden'})
 const detached=await snapshot('detached.json');assert.equal(detached.bodies.find(b=>b.id===linked.id).instance,undefined)
 await activate(solid.getByRole('button',{name:'Удалить группу '+group,exact:true}))
 const deleted=await snapshot('deleted-source-group.json');assert.deepEqual(deleted.bodies,[detached.bodies.find(b=>b.id===linked.id)])
 await historyRoundtrip(detached,deleted,'group-delete')
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload()
 await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 assert.deepEqual(await snapshot('final-reloaded.json'),deleted)
 await writeFile(path.join(directory,'gpu-validation-errors.json'),JSON.stringify(await page.evaluate(()=>window.__gpuValidationErrors??[]),null,2))
 assert.deepEqual(renderErrors,[])
 const report={browser:browser.version(),keyboard,tabPresses,downloads,groupCreateHistory:true,groupMoveHistory:true,groupDeleteHistory:true,sourceDeleteLocalized:true,lockedGroupDeleteLocalized:true,workspaceReload:true,finalReload:true}
 await writeFile(path.join(directory,'scene-state-browser.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
