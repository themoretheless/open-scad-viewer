import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
const history20=true
const keyboard=process.argv.includes('--keyboard')
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
const errors=[]
try{
 const {playwright}=await loadQualificationPlaywrightPackage();browser=await playwright.chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.on('pageerror',e=>errors.push(String(e)))
 if(process.argv.includes('--cpu'))await page.addInitScript(()=>Object.defineProperty(navigator,'gpu',{configurable:true,value:undefined}))
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 let tabs=0
 async function focusByTab(locator){
  for(let i=0;i<500;i++){
   if(await locator.evaluate(e=>e===document.activeElement))return
   await page.keyboard.press('Tab');tabs++
  }
  throw new Error('Control is not reachable by Tab: '+await locator.getAttribute('aria-label'))
 }
 async function activate(locator){if(keyboard){await locator.waitFor({state:'visible'});await page.waitForFunction(element=>!element.disabled,await locator.elementHandle());await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 let lastDownload=0
 async function exportDoc(name){if(history20){const wait=1100-(Date.now()-lastDownload);if(wait>0)await new Promise(resolve=>setTimeout(resolve,wait));lastDownload=Date.now()}await ready();if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu);const event=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await(await event).saveAs(path.join(directory,name));await activate(menu);return JSON.parse(await readFile(path.join(directory,name),'utf8'))}

 const fixtureRoot=process.env.CAD_MIXED20_UI_OUTPUT??'/tmp/cad-mixed20-ui-fixtures'
 const apply=solid.getByRole('button',{name:'Готово · Enter',exact:true})
 async function command(name){
  await activate(page.getByRole('button',{name:'Команды',exact:true}))
  const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
  if(keyboard){await focusByTab(search);await page.keyboard.insertText(name);await page.keyboard.press('Enter')}
  else {await search.fill(name);await search.press('Enter')}
 }
 async function input(label,value){const field=solid.getByLabel(label,{exact:true});if(keyboard){await focusByTab(field);await page.keyboard.press('ControlOrMeta+A');await page.keyboard.insertText(value)}else await field.fill(value)}
 const bounds=b=>[0,1,2].map(axis=>{const values=b.brep.vertices.map(v=>v.point[axis]);return [Math.min(...values),Math.max(...values)]})
 for(const name of ['bracket','enclosure','flange']){
  const initial=JSON.parse(await readFile(path.join(fixtureRoot,name,'initial.json'),'utf8'))
  const expected=JSON.parse(await readFile(path.join(fixtureRoot,name,'expected.json'),'utf8'))
  const title=initial.bodies[0].name
  await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'mixed.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(initial))});await activate(menu);await ready()
  await activate(solid.getByRole('tab',{name:'Сцена',exact:true}))
  const snapshots=[await exportDoc(name+'-0.json')]
  let current=snapshots[0],step=0,cancelledPreviews=0
  async function save(){
   current=await exportDoc(name+'-'+(++step)+'.json')
   const actual=current.bodies.find(b=>b.id===name),oracle=expected[step].bodies[0]
   assert.ok(actual);const ab=bounds(actual),eb=bounds(oracle)
   for(let a=0;a<3;a++)for(let j=0;j<2;j++)assert.ok(Math.abs(ab[a][j]-eb[a][j])<1e-6,`${name}/${step}: bounds`)
   for(const kind of ['vertices','edges','loops','faces','shells','bodies']){
    const ids=actual.brep.topologyIds[kind];assert.equal(ids.length,actual.brep[kind].length);assert.equal(new Set(ids).size,ids.length)
   }
   snapshots.push(current)
  }
  async function cancelPreview(){
   await page.waitForFunction(element=>!element.disabled,await apply.elementHandle())
   if(keyboard)await page.keyboard.press('Escape');else await solid.getByRole('button',{name:'Esc',exact:true}).last().click()
   assert.deepEqual(await exportDoc(name+'-cancel-preview-'+(++cancelledPreviews)+'.json'),current)
  }
  async function select(){await ready();await activate(solid.getByRole('button',{name:title,exact:true}))}
  async function move(){await select();await command('Transform selection');await input('X','1 mm');await activate(apply);await save()}
  for(let cycle=0;cycle<6;cycle++){
   await select();await activate(solid.getByRole('button',{name:'Грани',exact:true}));await ready()
   const selector=solid.getByRole('combobox',{name:'Выбрать грань',exact:true}),z=bounds(current.bodies.find(b=>b.id===name))[2][1]
   const top=await selector.locator('option').evaluateAll((options,z)=>{const o=options.find(o=>o.value!=='-1'&&Math.abs(Number(o.textContent.split('·')[1]?.replace('mm','').split(',')[2])-z)<1e-8);return o?Number(o.value):-1},z)
   assert.ok(top>=0)
   if(keyboard){await focusByTab(selector);await page.keyboard.press('Home');for(let i=0;i<=top;i++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Tab')}
   else {
    await solid.getByRole('button',{name:'ISO',exact:true}).click()
    const mesh=current.bodies.find(b=>b.id===name).mesh,candidates=[]
    for(let i=0;i<mesh.indices.length;i+=3){const points=mesh.indices.slice(i,i+3).map(j=>mesh.positions.slice(j*3,j*3+3));if(points.every(p=>Math.abs(p[2]-z)<1e-7))candidates.push([0,1,2].map(a=>points.reduce((sum,p)=>sum+p[a],0)/3))}
    const points=await solid.locator('svg[aria-label="Холст тел 3D"]').evaluate((svg,candidates)=>{
     return candidates.map(([x,y,z])=>new DOMPoint((x-y)*Math.SQRT1_2,(x+y)*Math.SQRT1_2/Math.sqrt(3)-z*Math.sqrt(2/3)).matrixTransform(svg.getScreenCTM()))
      .filter(p=>document.elementFromPoint(p.x,p.y)?.closest('svg')===svg).map(p=>({x:p.x,y:p.y}))
    },candidates)
    let picked=false
    for(const point of points){
     await page.mouse.click(point.x,point.y)
     if(await selector.inputValue()===String(top)&&await solid.getByRole('button',{name:title,exact:true}).getAttribute('aria-pressed')==='true'){picked=true;break}
     await select()
    }
    assert.ok(picked,`${name}: viewport picking selects the top cap`)
   }
   await command('Push / Pull');await input('Расстояние, мм','1 mm');await cancelPreview();await command('Push / Pull');await input('Расстояние, мм','1 mm');await activate(apply);await save()
   await select();const tool=solid.getByRole('button',{name:`Tool ${cycle}`,exact:true})
   if(keyboard){await focusByTab(tool);await page.keyboard.press('Shift+Enter')}else await tool.click({modifiers:['Shift']})
   await command(name==='bracket'?'Union bodies':'B-rep A − B')
   if(name!=='bracket')await activate(solid.getByRole('button',{name:'OK',exact:true}))
   await tool.waitFor({state:'detached'});await save();await move()
  }
  await select();await activate(solid.getByRole('button',{name:'Рёбра',exact:true}));await ready()
  const b=current.bodies.find(b=>b.id===name).brep,z=bounds(current.bodies.find(b=>b.id===name))[2][1]
  const edges=b.edges.flatMap((e,i)=>e.vertices.every(v=>{const p=b.vertices[v].point;return name==='flange'?e.curve.degree===2&&Math.abs(p[2]-z)<1e-8&&Math.abs(Math.hypot(p[0]-6,p[1])-20)<1e-8:Math.abs(p[0]-6)<1e-8&&Math.abs(p[1])<1e-8})?[i]:[])
  assert.ok(edges.length)
  if(!keyboard){const rect=await solid.locator('svg[aria-label="Холст тел 3D"]').boundingBox();await page.mouse.move(rect.x+rect.width*.5,rect.y+rect.height*.5);await page.mouse.down({button:'right'});await page.mouse.move(rect.x+rect.width*.5+80,rect.y+rect.height*.5+35,{steps:12});await page.mouse.up({button:'right'})}
  for(const [j,index] of edges.entries()){
   const edge=solid.locator(`[data-topology-edge="${b.topologyIds.edges[index]}"]`)
   if(keyboard){await focusByTab(edge);await page.keyboard.press(j?'Shift+Enter':'Enter')}
   else {
    const point=await edge.evaluate(e=>{for(const f of [.5,.25,.75,.1,.9]){const p=e.getPointAtLength(e.getTotalLength()*f).matrixTransform(e.getScreenCTM());if(document.elementFromPoint(p.x,p.y)===e)return {x:p.x,y:p.y}}return null})
    assert.ok(point,'Visible edge target');if(j)await page.keyboard.down('Shift');try{await page.mouse.click(point.x,point.y)}finally{if(j)await page.keyboard.up('Shift')}
   }
  }
  await command('Скруглить 3D');await input('Радиус / размер, мм','1 mm');await cancelPreview();await command('Скруглить 3D');await input('Радиус / размер, мм','1 mm');await activate(apply);await save();await move()
  assert.equal(cancelledPreviews,7);assert.equal(step,20);assert.equal(new Set(snapshots.map(d=>JSON.stringify(d))).size,21)
  await page.screenshot({path:path.join(directory,name+'.png')})
  for(let i=19;i>=0;i--){await activate(solid.getByRole('button',{name:'↶',exact:true}));assert.deepEqual(await exportDoc(name+'-undo-'+i+'.json'),snapshots[i])}
  for(let i=1;i<=20;i++){await activate(solid.getByRole('button',{name:'↷',exact:true}));assert.deepEqual(await exportDoc(name+'-redo-'+i+'.json'),snapshots[i])}
  await solid.getByRole('status',{name:'Сохранено в браузере',exact:true}).waitFor();await page.reload();await solid.getByRole('button',{name:title,exact:true}).waitFor();assert.deepEqual(await exportDoc(name+'-reload.json'),current)
 }
 assert.deepEqual(errors,[]);await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,renderer:process.argv.includes('--cpu')?'cpu-fallback':'browser-default',keyboard,tabs,edits:20,cancelledPreviewsPerPart:7,cases:['bracket','enclosure','flange'],errors},null,2))
}catch(e){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw e}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
