import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
import {createBrepBox,transformNurbsBrep,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {stringifyMeshJson} from '../src/services/meshJson'
const directory=path.resolve(process.argv[2]??'/tmp/variable-radius-browser'),root=path.resolve('dist'),keyboard=process.argv.includes('--keyboard')
await mkdir(directory,{recursive:true})
const source=createBrepBox([-7,3,-2],[3,11,4]),a=.37,b=-.61
const brep=transformNurbsBrep(source,[[Math.cos(a)*Math.cos(b),-Math.sin(a),Math.cos(a)*Math.sin(b),17],[Math.sin(a)*Math.cos(b),Math.cos(a),Math.sin(a)*Math.sin(b),-9],[-Math.sin(b),0,Math.cos(b),23],[0,0,0,1]])
const initial={version:1,sketches:[],bodies:[{id:'rotated',name:'Rotated cuboid',brep,mesh:tessellateNurbsBrep(brep),material:{name:'Copper',color:'#cc7744'}}]}
const server=createServer(async(req,res)=>{try{const url=new URL(req.url!,'http://localhost');if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}const file=path.resolve(root,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)));if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream');res.end(await readFile(file))}catch{res.writeHead(404).end()}})
await new Promise<void>(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser:any,page:any;const errors:string[]=[];let tabs=0
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1100}});page.on('pageerror',(e:Error)=>errors.push(String(e)))
 await page.addInitScript(()=>{
  const Native=window.Worker,w=window as any;w.__bodyEditRequests=0
  window.Worker=class extends Native{
   constructor(...args:any[]){super(...args);this.addEventListener('message',(event:any)=>{
    if(w.__holdBodyEdit&&event.data?.kind==='bodyEdit'&&event.data.ok){
     event.stopImmediatePropagation();const deliver=this.onmessage;w.__lateBodyEdit=()=>deliver?.call(this,event);w.__holdBodyEdit=false
    }
   },true)}
   postMessage(message:any,...args:any[]){
    if(message?.job?.kind==='bodyEdit'){
     w.__bodyEditRequests++
     if(w.__failBodyEdit){w.__failBodyEdit=false;queueMicrotask(()=>this.onmessage?.({data:{version:1,id:message.id,kind:'bodyEdit',ok:false,error:{name:'Error',code:'CAD_TRANSPORT',message:'Injected worker transport failure'}}} as any));return}
    }
    return super.postMessage(message,...args)
   }
  }
 })
 await page.goto(`http://127.0.0.1:${(server.address() as any).port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]'),apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 async function tabTo(control:any){for(let i=0;i<500;i++){if(await control.evaluate((e:HTMLElement)=>e===document.activeElement))return;await page.keyboard.press('Tab');tabs++}throw Error('Unreachable control')}
 async function activate(control:any){await control.waitFor({state:'visible'});if(keyboard){await tabTo(control);await page.keyboard.press('Enter')}else await control.click()}
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 let lastDownload=0
 async function exportDoc(name:string){const delay=1100-(Date.now()-lastDownload);if(delay>0)await new Promise(r=>setTimeout(r,delay));await ready();if(!await menu.evaluate((e:HTMLElement)=>(e.parentElement as HTMLDetailsElement).open))await activate(menu);const event=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await(await event).saveAs(path.join(directory,name));lastDownload=Date.now();await activate(menu);return JSON.parse(await readFile(path.join(directory,name),'utf8'))}
 async function choose(control:any,value:string){
  await page.waitForFunction(({element,value}:any)=>Array.from(element.options).some((option:any)=>option.value===value),{element:await control.elementHandle(),value})
  if(!keyboard){await control.selectOption(value);return}
  if(await control.getAttribute('aria-label')==='Выбрать ребро'){
   await tabTo(control);await page.keyboard.press('Home')
   const values=await control.locator('option').evaluateAll((nodes:HTMLOptionElement[])=>nodes.map(n=>n.value))
   for(let i=0;i<values.indexOf(value);i++)await page.keyboard.press('ArrowDown')
   await page.keyboard.press('Tab');assert.equal(await control.inputValue(),value);return
  }
  const label=await control.locator('option').evaluateAll((nodes:HTMLOptionElement[],value:string)=>nodes.find(n=>n.value===value)?.textContent,value)
  assert.ok(label);await tabTo(control);await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');await tabTo(control)
  // Native type-ahead works in headless macOS where popup arrows are ignored.
  const cdp=await page.context().newCDPSession(page)
  try{for(const letter of label)await cdp.send('Input.dispatchKeyEvent',{type:'char',text:letter,key:letter})}finally{await cdp.detach()}
  assert.equal(await control.inputValue(),value)
 }

 async function fill(label:string,value:string){const field=solid.getByLabel(label,{exact:true});if(keyboard){await tabTo(field);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value);await page.keyboard.press('Tab')}else await field.fill(value)}
 await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'rotated.json',mimeType:'application/json',buffer:Buffer.from(stringifyMeshJson(initial))});await activate(menu);await ready();await activate(solid.getByRole('tab',{name:'Сцена',exact:true}));await activate(solid.getByRole('button',{name:'Rotated cuboid',exact:true}))
 const before=await exportDoc('before.json')
 const invalid=JSON.parse(stringifyMeshJson(initial));invalid.bodies[0].mesh.positions={'0':0}
 await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'invalid.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(invalid))});await activate(menu)
 await solid.getByText('Проект не открыт. Сетка тела повреждена: проверьте координаты вершин и индексы треугольников в JSON или экспортируйте проект заново.',{exact:false}).waitFor()
 assert.deepEqual(await exportDoc('invalid-import-preserved.json'),before)
 await activate(solid.getByRole('button',{name:'Rotated cuboid',exact:true}))
 async function begin(){await activate(solid.getByRole('button',{name:'Рёбра',exact:true}));await ready();await choose(solid.getByRole('combobox',{name:'Выбрать ребро',exact:true}),'0');await activate(page.getByRole('button',{name:'Команды',exact:true}));const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'});if(keyboard){await tabTo(search);await page.keyboard.insertText('Скруглить 3D')}else await search.fill('Скруглить 3D');await search.press('Enter');await choose(solid.getByRole('combobox',{name:'Тип скругления',exact:true}),'variable');await fill('Радиус A, мм','0.5 mm');await fill('Радиус B, мм','1.5 mm');await apply.click({trial:true})}
 if(process.argv.includes('--faults')){
  await begin();await page.evaluate(()=>{(window as any).__failBodyEdit=true});await fill('Радиус B, мм','1.6 mm')
  const retry=solid.getByRole('button',{name:'Повторить вычисление',exact:true});await retry.waitFor();assert.equal(await apply.isDisabled(),true)
  assert.deepEqual(await exportDoc('worker-error-preserved.json'),before)
  await activate(retry);await apply.click({trial:true});await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('retry-cancelled.json'),before)
  await begin();await page.evaluate(()=>{(window as any).__holdBodyEdit=true});await fill('Радиус B, мм','1.7 mm');await page.waitForFunction(()=>typeof(window as any).__lateBodyEdit==='function')
  await page.keyboard.press('Escape');await page.evaluate(()=>{(window as any).__lateBodyEdit()});assert.deepEqual(await exportDoc('late-result-preserved.json'),before)
 }
 await begin();await page.keyboard.press('Escape');assert.deepEqual(await exportDoc('cancelled.json'),before)
 await begin();const requests=await page.evaluate(()=>(window as any).__bodyEditRequests);assert.ok(requests>0);await page.screenshot({path:path.join(directory,'preview.png')});await activate(apply)
 const changed=await exportDoc('applied.json');assert.equal(changed.bodies[0].id,'rotated');assert.deepEqual(changed.bodies[0].material,before.bodies[0].material)
 const [p,q]=source.edges[0].vertices.map(i=>source.vertices[i].point),length=Math.hypot(...p.map((x,i)=>x-q[i]));const expectedVolume=480-(1-Math.PI/4)*length*(.25+.75+2.25)/3,volume=analyzeNurbsBrep(changed.bodies[0].brep).signedVolumeMm3;assert.ok(Math.abs(volume-expectedVolume)<1e-4)
 await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await exportDoc('undo.json'),before);await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await exportDoc('redo.json'),changed)
 await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await solid.getByRole('button',{name:'Rotated cuboid',exact:true}).waitFor();assert.deepEqual(await exportDoc('reload.json'),changed);assert.deepEqual(errors,[])
 const report={passed:true,workerFaults:process.argv.includes('--faults'),keyboard,tabs,workerRequests:requests,cancel:true,invalidImportPreservesDocument:true,undoRedo:true,reload:true,volumeMm3:volume,expectedVolumeMm3:expectedVolume,errors,scope:'one rotated cuboid edge through current UI; all-edge qualification is native/product/worker evidence'};await writeFile(path.join(directory,'report.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise<void>(resolve=>server.close(()=>resolve()))}
