import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-profile-offset')
const history20=process.argv.includes('--history-20')
const keyboard=process.argv.includes('--keyboard'),theme=process.argv.find(a=>a.startsWith('--theme='))?.slice(8)??'system'
assert.ok(!(history20&&process.argv.includes('--step')),'STEP manifest currently describes the single-edit case')
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
const errors=[],stepParts=[]
try{
 const {playwright}=await loadQualificationPlaywrightPackage();browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage({acceptDownloads:true,viewport:{width:1440,height:1000}});page.on('pageerror',e=>errors.push(String(e)))
 await page.addInitScript(()=>{
  const NativeWorker=window.Worker
  window.__holdCap=false;window.__crashCapReplies=[];window.__capWorkerSerial=0;window.__capWorkerId=0;window.__failCapReplies=[];window.__lateCapReplies=[];window.__capTerminations=0;window.__capSuccessReplies=0
  window.Worker=class extends NativeWorker{
   constructor(...args){super(...args);this.__capId=++window.__capWorkerSerial}
   postMessage(message,...args){
    if(window.__holdCap&&message?.job?.kind==='bodyEdit'&&message.job.options?.operation==='push'){
     this.__capHeld=true;window.__capWorkerId=this.__capId
     const callback=this.onmessage
     this.onmessage=event=>{if(event.data?.ok===true)window.__capSuccessReplies++;window.__lateCapReplies.push(()=>callback?.call(this,event));if(message.job.options.distance===1)window.__crashCapReplies.push(()=>this.dispatchEvent(new ErrorEvent('error',{message:'Injected worker failure'})));if(message.job.options.distance===1)window.__failCapReplies.push(()=>callback?.call(this,{data:{...event.data,ok:false,error:{name:'Error',code:'CAD_CRASH',message:'Injected calculation failure'}}}))}
    }
    return super.postMessage(message,...args)
   }
   terminate(){if(this.__capHeld)window.__capTerminations++;return super.terminate()}
  }
 })
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
 async function activate(locator){if(keyboard){await focusByTab(locator);await page.keyboard.press('Enter')}else await locator.click()}
 async function ready(){for(const name of ['history-restore','display-refinement','topology-preparation'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden'})}
 let lastDownload=0
 async function exportDoc(name){if(history20){const wait=1100-(Date.now()-lastDownload);if(wait>0)await new Promise(resolve=>setTimeout(resolve,wait));lastDownload=Date.now()}await ready();if(!await menu.evaluate(e=>e.parentElement.open))await activate(menu);const event=page.waitForEvent('download');await activate(solid.getByRole('button',{name:'Скачать проект JSON',exact:true}));await(await event).saveAs(path.join(directory,name));await activate(menu);return JSON.parse(await readFile(path.join(directory,name),'utf8'))}
 const fixture=JSON.parse(await readFile('docs/qualification/cad-roadmap-2026-09-28/history20-parts-2026-09-30/browser/flange-reload.json','utf8'))
 if(process.argv.includes('--material'))fixture.bodies[0].material={name:'Copper',color:'#c06030'}
 await activate(menu);await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'flange.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))});await activate(menu);await ready()
 await solid.getByRole('button',{name:'flange',exact:true}).waitFor();await ready()
 const paints=await solid.locator('polygon[data-body]').evaluateAll(nodes=>nodes.map(n=>{const c=getComputedStyle(n);return {fill:c.fill,stroke:c.stroke,width:c.strokeWidth}}))
 assert.ok(paints.length>0);assert.ok(paints.every(p=>p.fill===p.stroke&&p.width==='1px'))
 await page.screenshot({path:path.join(directory,'before.png')})
 if(process.argv.includes('--stroke-study')){for(const width of ['0','1','1.5','2']){await solid.locator('polygon[data-body]').evaluateAll((nodes,width)=>nodes.forEach(n=>n.setAttribute('stroke-width',width)),width);await page.screenshot({path:path.join(directory,'stroke-'+width+'.png')})}}
 const culled=await solid.locator('polygon[data-body]').evaluateAll(nodes=>{let removed=0;for(const n of nodes){const p=n.getAttribute('points').split(' ').map(s=>s.split(',').map(Number));const area=(p[1][0]-p[0][0])*(p[2][1]-p[0][1])-(p[1][1]-p[0][1])*(p[2][0]-p[0][0]);if(area<=0){n.style.display='none';removed++}}return {total:nodes.length,removed}})
 await page.screenshot({path:path.join(directory,'culled.png')});assert.ok(culled.total>0);assert.equal(culled.removed,0);assert.deepEqual(await exportDoc('unchanged.json'),fixture);assert.deepEqual(errors,[]);await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,...culled,documentUnchanged:true},null,2));console.log(culled)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
