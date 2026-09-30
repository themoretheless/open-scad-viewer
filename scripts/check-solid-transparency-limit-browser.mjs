import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-transparency-limit')
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
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 const page=await browser.newPage({acceptDownloads:true}),errors=[]
 page.on('pageerror',e=>errors.push(String(e)))
 await page.addInitScript(()=>{Object.defineProperty(navigator,'gpu',{value:undefined})})
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true}),menu=solid.locator('summary[title="Файл"]')
 await solid.waitFor()
 const positions=Array.from({length:600},(_,i)=>[[-10,-10],[10,-10],[0,10]].flatMap(([x,y])=>[x,y,Math.sin(i*1.7)*x+Math.cos(i*2.3)*y+i*.001])).flat()
 const body={id:'complex',name:'Crossing planes',material:{name:'Glass',color:'#4488aa',opacity:.5},mesh:{positions,indices:Array.from({length:positions.length/3},(_,i)=>i)}}
 const simple={id:'simple',name:'Simple',material:body.material,mesh:{positions:[30,0,0,40,0,0,30,10,0],indices:[0,1,2]}}
 const fixture={version:1,sketches:[],bodies:[body,simple]}
 await menu.click();await solid.locator('input[accept=".json,application/json"]').setInputFiles({name:'complex.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(fixture))})
 if(await menu.evaluate(e=>e.parentElement.open))await menu.click()
 const warning=solid.getByRole('status',{name:'transparency-limit',exact:true})
 await warning.waitFor({timeout:30000})
 assert.ok(await solid.locator('[data-body="complex"]').count()>0)
 async function exportDoc(name){
  if(await menu.evaluate(e=>!e.parentElement.open))await menu.click()
  const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click()
  const item=await pending;await item.saveAs(path.join(directory,name))
  if(await menu.evaluate(e=>e.parentElement.open))await menu.click()
  return JSON.parse(await readFile(path.join(directory,name),'utf8'))
 }
 const before=await exportDoc('before.json')
 await page.screenshot({path:path.join(directory,'limited.png')})
 await solid.getByRole('button',{name:'Скрыть: Crossing planes',exact:true}).click()
 await warning.waitFor({state:'hidden'})
 assert.ok(await solid.locator('[data-body="simple"]').count()>0)
 assert.deepEqual(await exportDoc('reduced.json'),before)
 await solid.getByRole('button',{name:'Показать: Crossing planes',exact:true}).click()
 await warning.waitFor();assert.deepEqual(await exportDoc('restored.json'),before)
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),triangles:601,realBspLimit:true,visibleFallback:true,hideClearsWarning:true,showRestoresWarning:true,documentUnchanged:true,errors}
 await writeFile(path.join(directory,'report.json'),JSON.stringify(report,null,2));console.log(report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
