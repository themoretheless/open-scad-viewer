import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve('dist'),directory=path.resolve(process.argv[2]??'docs/qualification/loft-generalization-2026-10-02/browser')
await mkdir(directory,{recursive:true})
const server=createServer(async(req,res)=>{
 try{const file=path.resolve(root,'.'+(new URL(req.url,'http://localhost').pathname==='/'?'/index.html':decodeURIComponent(new URL(req.url,'http://localhost').pathname)))
  if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream');res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const {playwright}=await loadQualificationPlaywrightPackage(),browser=await playwright.chromium.launch({headless:true}),reports=[]
let currentPage
try{
 for(const viewport of [{width:1440,height:1000},{width:600,height:1000}]){
  const context=await browser.newContext({viewport,acceptDownloads:true}),page=await context.newPage(),errors=[]
  currentPage=page
  page.setDefaultTimeout(30000)
  page.on('pageerror',error=>errors.push(error.message))
  await page.addInitScript(()=>{
   const NativeWorker=window.Worker;window.loftHeld=[];window.loftTerminated=[];window.loftHoldBuild=false;window.loftHoldExact=false
   window.Worker=class extends NativeWorker {
    postMessage(message,...args){
     const mode=message?.type==='build'?'build':message?.kind==='exact-solid'?'exact':null
     if(mode&&(mode==='build'?window.loftHoldBuild:window.loftHoldExact)){
      this.loftMode=mode;window.loftHeld.push(mode);return
     }
     return super.postMessage(message,...args)
    }
    terminate(){if(this.loftMode)window.loftTerminated.push(this.loftMode);return super.terminate()}
   }
  })
  await page.goto(`http://127.0.0.1:${server.address().port}`)
  await page.getByRole('button',{name:'Исходный код',exact:true}).click()
  await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
  const input=page.locator('input[accept=".scad,.r,.mg,text/plain"]'),editor=page.locator('textarea.code-input')
  const load=async(file,source)=>{await input.setInputFiles({name:file,mimeType:'text/plain',buffer:Buffer.from(source)})}
  const build=async()=>{await page.getByRole('button',{name:'Собрать',exact:true}).click();await page.waitForFunction(()=>{const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Собрать');return b&&!b.disabled},{},{timeout:120000})}
  const assertions=[]
  for(const file of ['cartesian-control-tangent-loft.r','cartesian-auto-control-tangent-loft.r','cartesian-mapped-loft.r','mapped-natural-loft.r','authored-nonplanar-cap-loft.r']){
   await load(file,await readFile(path.resolve('examples/rush',file),'utf8'));await build()
   assert.equal(await page.locator('.message.error').count(),0,file)
   assert.match(await page.locator('.stats').innerText(),/Треугольники|треугольник|Triangles/)
   assertions.push({case:file,status:'built'});console.log(viewport.width,file,'built')
  }
  const cap=await readFile('examples/rush/authored-nonplanar-cap-loft.r','utf8')
  await load('exhausted.r',cap.replace('loft_embedding_limits()','loft_embedding_limits(facePairs: 1)'));await build()
  assert.match(await page.locator('.message.error').innerText(),/embedding|pairs|budget/i);assertions.push({case:'embedding-budget-refusal',status:'refused'});console.log(viewport.width,'budget refused')
  await page.getByRole('button',{name:'В Solid',exact:true}).click();await page.locator('.message.error').waitFor();await page.waitForFunction(()=>![...document.querySelectorAll('.editor-toolbar button')].some(b=>b.textContent.trim()==='…'))
  assert.match(await page.locator('.message.error').innerText(),/embedding|pairs|budget/i);assertions.push({case:'solid-budget-refusal',status:'refused without publishing a body'})
  await load('cancel-build.r',cap);await page.evaluate(()=>window.loftHoldBuild=true)
  await page.getByRole('button',{name:'Собрать',exact:true}).click();await page.waitForFunction(()=>window.loftHeld.includes('build'))
  await page.getByRole('button',{name:'Отменить сборку',exact:true}).click()
  await page.waitForFunction(()=>!document.body.innerText.includes('Отменить сборку'));assertions.push({case:'build-cancellation',status:'cancelled',fault:'explicitly held worker dispatch'})
  console.log(viewport.width,'build cancelled')
  await page.evaluate(()=>window.loftHoldBuild=false);await build();assert.equal(await page.locator('.message.error').count(),0)
  await page.evaluate(()=>window.loftHoldExact=true)
  await page.getByRole('button',{name:'В Solid',exact:true}).click();await page.waitForFunction(()=>window.loftHeld.includes('exact'))
  await page.getByRole('button',{name:'Отмена',exact:true}).click();await page.waitForFunction(()=>window.loftTerminated.includes('exact'))
  assertions.push({case:'solid-cancellation',status:'cancelled',fault:'explicitly held worker dispatch'})
  console.log(viewport.width,'solid cancelled')
  const before=await page.evaluate(()=>window.loftTerminated.length)
  await page.getByRole('button',{name:'В Solid',exact:true}).click()
  await page.waitForFunction(n=>window.loftHeld.filter(x=>x==='exact').length>=n,2)
  await editor.fill(cap+'\n// revised while Solid was building\n')
  await page.waitForFunction(n=>window.loftTerminated.length>n,before);assertions.push({case:'source-change',status:'superseded exact worker terminated'})
  console.log(viewport.width,'source superseded')
  await page.evaluate(()=>window.loftHoldExact=false)
  await page.getByRole('button',{name:'В Solid',exact:true}).click()
  const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
  await page.waitForFunction(()=>document.querySelector('main.main')?.inert===true,{},{timeout:120000})
  await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden',timeout:120000})
  const menu=solid.locator('summary[title="Файл"]');await menu.click()
  const pending=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click()
  const download=await pending,documentPath=path.join(directory,viewport.width+'-solid.json');await download.saveAs(documentPath);await menu.click()
  const document=JSON.parse(await readFile(documentPath,'utf8'));assert.equal(document.bodies.length,1);assert.equal(document.bodies[0].brep.faces.length,6)
  assertions.push({case:'solid-publication',status:'one retained six-face B-rep'})
  await solid.getByRole('button',{name:'Вписать',exact:true}).last().click()
  assert.ok(await solid.locator('svg[aria-label="Холст тел 3D"] polygon').count()>0)
  await page.screenshot({path:path.join(directory,viewport.width+'-solid.png'),fullPage:true})
  await solid.getByRole('status',{name:'Сохранено в браузере',exact:true,includeHidden:true}).waitFor({state:'attached'})
  await page.reload();await solid.waitFor();await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden',timeout:120000})
  await menu.click();const restoredDownload=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const restoredPath=path.join(directory,viewport.width+'-restored.json');await(await restoredDownload).saveAs(restoredPath);await menu.click()
  const restored=JSON.parse(await readFile(restoredPath,'utf8'));assert.equal(restored.bodies.length,1);assert.deepEqual(restored.bodies[0].brep,document.bodies[0].brep)
  assertions.push({case:'restore',status:'Solid restored after reload'})
  assert.deepEqual(errors,[])
  reports.push({viewport,assertions,pageErrors:errors,render:'headless CPU fallback; no GPU qualification',bodyText:await page.locator('body').innerText()})
  await context.close()
 }
 await writeFile(path.join(directory,'matrix.json'),JSON.stringify({scope:'finite UI scenarios; held dispatch proves lifecycle cancellation, not mid-kernel interruption latency',cases:reports},null,2)+'\n')
 console.log(JSON.stringify(reports.map(r=>({viewport:r.viewport,assertions:r.assertions,pageErrors:r.pageErrors})),null,2))
}catch(error){if(currentPage){await currentPage.screenshot({path:'/tmp/loft-ui-failure.png',fullPage:true});console.error((await currentPage.locator('body').innerText()).slice(-7000))}throw error}
finally{await browser.close();await new Promise(resolve=>server.close(resolve))}
