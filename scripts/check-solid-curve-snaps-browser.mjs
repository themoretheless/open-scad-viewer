import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-sketch-snaps')
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
const errors=[]
try {
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage({acceptDownloads:true});page.on('pageerror',e=>{errors.push(e.stack??String(e));console.error(e.stack??String(e))})
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker;window.__snapRequests=0;window.__holdSnaps=false
  window.Worker=class extends NativeWorker {
   postMessage(message,...args){
    if(message?.job?.kind==='sketchSnaps'){
     window.__snapRequests++
     if(window.__holdSnaps){this.__held=true;window.__snapHeld=true;return}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__held)window.__snapTerminated=true;return super.terminate()}
  }
 })
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}).waitFor({timeout:10000})
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 async function closeMenu(){if(await menu.evaluate(e=>e.parentElement.open))await menu.click()}
 async function ready(){await page.waitForFunction(()=>!document.body.innerText.includes('Восстанавливаю геометрию'));await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'primitive-build',exact:true}).waitFor({state:'hidden'});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden'})}
 async function exportDoc(file){await ready();if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const download=await pending;await download.saveAs(path.join(directory,file));await closeMenu();return JSON.parse(await readFile(path.join(directory,file),'utf8'))}
 await ready()
 const reports=[]
 const cases=[
  {name:'xy',plane:{origin:[15,15,5],u:[1,0,0],v:[0,1,0]},start:0,sweep:360},
  {name:'retained-vertical',retained:true,plane:{origin:[20,15,5],u:[0,1,0],v:[0,0,1]},start:0,sweep:360},
  {name:'vertical',plane:{origin:[20,15,5],u:[0,1,0],v:[0,0,1]},start:0,sweep:360},
  {name:'tilted-arc',plane:{origin:[15,15,5],u:[Math.SQRT1_2,0,Math.SQRT1_2],v:[0,1,0]},start:110,sweep:-140},
 ]
 for(const item of cases){
  const fixture=JSON.parse(await readFile('tests/fixtures/solid-surface-boundary.json','utf8'));fixture.surfaces=fixture.surfaces.slice(0,1)
  fixture.sketches=[{id:'target',name:'Target',closed:item.sweep===360,plane:item.plane,points:Array.from({length:65},(_,i)=>{const a=(item.start+item.sweep*i/64)*Math.PI/180;return [10*Math.cos(a),10*Math.sin(a)]}),analytic:{kind:item.sweep===360?'circle':'arc',center:[0,0],radius:10,start:item.start,sweep:item.sweep}}]
  if(item.retained){
   const control=[[[10,0],[10,10],[0,10]],[[0,10],[-10,10],[-10,0]],[[-10,0],[-10,-10],[0,-10]],[[0,-10],[10,-10],[10,0]]]
   delete fixture.sketches[0].analytic
   fixture.sketches[0].retainedProfile={loops:[control.map(controlPoints=>({degree:2,knots:[0,0,0,1,1,1],weights:[1,Math.SQRT1_2,1],controlPoints}))]}
  }
  await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'curve-target.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await closeMenu();await ready()
  await solid.getByRole('status',{name:'sketch-snap-preparation',exact:true}).waitFor({state:'hidden'})
  await solid.getByRole('button',{name:'Surface A',exact:true}).click()
  await solid.getByRole('status',{name:'surface-display',exact:true}).waitFor({state:'hidden'})
  const before=await exportDoc(`${item.name}-before.json`)
  const fraction=5.5/64,angle=(item.start+item.sweep*fraction)*Math.PI/180
  const target=item.plane.origin.map((v,i)=>v+10*Math.cos(angle)*item.plane.u[i]+10*Math.sin(angle)*item.plane.v[i])
  const screen=await solid.locator('svg[aria-label="Холст тел 3D"]').evaluate((svg,p)=>{
   const yaw=Math.PI/4,pitch=Math.atan(1/Math.sqrt(2)),x=p[0]*Math.cos(yaw)-p[1]*Math.sin(yaw),y=(p[0]*Math.sin(yaw)+p[1]*Math.cos(yaw))*Math.sin(pitch)-p[2]*Math.cos(pitch)
   const point=new DOMPoint(x,y).matrixTransform(svg.getScreenCTM());return {x:point.x,y:point.y}
  },target)
  const cv=await solid.locator('.nurbs-cage circle').first().boundingBox();assert.ok(cv)
  await page.mouse.move(cv.x+cv.width/2,cv.y+cv.height/2);await page.mouse.down();await page.mouse.move(screen.x,screen.y,{steps:8});await page.mouse.up()
  await page.waitForFunction(()=>!document.body.innerText.includes('Вычисляется преобразование.'))
  const changed=await exportDoc(`${item.name}-changed.json`),point=changed.surfaces[0].surface.controlPoints[0][0]
  assert.notDeepEqual(point,before.surfaces[0].surface.controlPoints[0][0])
  const delta=point.map((v,i)=>v-item.plane.origin[i]),dot=(a,b)=>a.reduce((n,v,i)=>n+v*b[i],0),u=dot(delta,item.plane.u),v=dot(delta,item.plane.v)
  const normal=[item.plane.u[1]*item.plane.v[2]-item.plane.u[2]*item.plane.v[1],item.plane.u[2]*item.plane.v[0]-item.plane.u[0]*item.plane.v[2],item.plane.u[0]*item.plane.v[1]-item.plane.u[1]*item.plane.v[0]]
  assert.ok(Math.abs(dot(delta,normal))<1e-8,'Point lies on target plane')
  assert.ok(Math.abs(Math.hypot(u,v)-10)<1e-8,'Point lies on analytic circle, not a display chord')
  const degrees=Math.atan2(v,u)*180/Math.PI,travel=((item.sweep>0?degrees-item.start:item.start-degrees)%360+360)%360
  assert.ok(travel<=Math.abs(item.sweep)+1e-7,'Point lies within directed arc interval')
  const interval=travel/Math.abs(item.sweep)*64
  assert.ok(Math.abs(interval-Math.round(interval))>.05,'Point is between display samples')
  assert.deepEqual(changed.sketches,before.sketches);assert.deepEqual(changed.bodies,before.bodies)
  const untouched=structuredClone(changed);untouched.surfaces[0].surface.controlPoints[0][0]=before.surfaces[0].surface.controlPoints[0][0];assert.deepEqual(untouched,before)
  await solid.getByRole('button',{name:'↶',exact:true}).click();assert.deepEqual(await exportDoc(`${item.name}-undone.json`),before)
  await solid.getByRole('button',{name:'↷',exact:true}).click();assert.deepEqual(await exportDoc(`${item.name}-redone.json`),changed)
  reports.push({case:item.name,point,radius:Math.hypot(u,v),planeDistance:dot(delta,normal),sampleInterval:interval})
 }
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'curve-snaps.png')})
 const report={browser:browser.version(),cases:reports,undoRedo:true}
 await writeFile(path.join(directory,'curve-snap-browser.json'),JSON.stringify(report,null,2));console.log(report)
}catch(error){console.error('Page errors:',errors);if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw error}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
