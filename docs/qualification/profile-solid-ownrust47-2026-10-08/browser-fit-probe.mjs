import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from '/Users/themoretheless/.codex/worktrees/sweep-structure/open-scad-viewer/scripts/qualificationPlaywrightPackage.mjs'
const dist=path.resolve('dist'),output=path.resolve(process.argv[2]??'/tmp/profile-solid-browser')
await mkdir(output,{recursive:true})
const sha=data=>createHash('sha256').update(data).digest('hex')
const catalogBytes=await readFile('docs/design/sweep-qualification-catalog.json')
const catalog=JSON.parse(catalogBytes)
const report={schema:'profile-solid-browser/1',scope:'Finite retained-body lifecycle scenarios. Held dispatch checks cancellation, not interruption latency or GPU qualification.',geometryWasmSha256:sha(await readFile('public/wasm/geometry-kernel.wasm')),distIndexSha256:sha(await readFile(path.join(dist,'index.html'))),selectionCatalogSha256:sha(catalogBytes),cases:[]}
const save=()=>writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n')
const server=createServer(async(req,res)=>{try{
 const url=new URL(req.url,'http://localhost')
 if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}
 const file=path.resolve(dist,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
 if(!file.startsWith(dist+path.sep)){res.writeHead(403).end();return}
 res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
 res.end(await readFile(file))
}catch{res.writeHead(404).end()}})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const {playwright}=await loadQualificationPlaywrightPackage()
const browser=await playwright.chromium.launch({headless:true,channel:'chromium',args:['--enable-unsafe-webgpu']})
report.browser=browser.version()
try{for(const width of [1440,600])for(const file of catalog.browser.profileSolid){
 const source=await readFile(path.join('examples/rush',file),'utf8'),context=await browser.newContext({viewport:{width,height:1000},acceptDownloads:true}),page=await context.newPage()
 const entry={width,file,sourceSha256:sha(source),checks:[],errors:[],status:'running'};report.cases.push(entry)
 page.on('pageerror',e=>entry.errors.push(e.message));page.setDefaultTimeout(30000)
 try{
  await page.addInitScript(()=>{const NativeWorker=window.Worker;window.profileHeld=[];window.profileTerminated=[];window.profileHoldBuild=false;window.profileHoldSolid=false
   window.Worker=class extends NativeWorker{postMessage(message,...args){const mode=message?.type==='build'?'build':message?.kind==='exact-solid'?'solid':null;if(mode&&(mode==='build'?window.profileHoldBuild:window.profileHoldSolid)){this.heldMode=mode;window.profileHeld.push(mode);return}super.postMessage(message,...args)}terminate(){if(this.heldMode)window.profileTerminated.push(this.heldMode);super.terminate()}}
  })
  await page.goto(`http://127.0.0.1:${server.address().port}`)
  const sourceButton=page.getByRole('button',{name:'Исходный код',exact:true})
  const open=async()=>{if(await sourceButton.getAttribute('aria-pressed')!=='true')await sourceButton.click()}
  await open();await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
  const load=text=>page.locator('input[accept=".scad,.r,.mg,text/plain"]').setInputFiles({name:'profile.r',mimeType:'text/plain',buffer:Buffer.from(text)})
  const build=async()=>{await page.getByRole('button',{name:'Собрать',exact:true}).click();await page.waitForFunction(()=>{const b=[...document.querySelectorAll('.editor-toolbar button')].find(b=>b.textContent.trim()==='Собрать');return b&&!b.disabled},{},{timeout:120000})}
  const good=async()=>{await load(source);await build();assert.equal(await page.locator('.message.error').count(),0);const text=await page.getByTestId('sweep-final-evidence').innerText();assert.match(text,/Геометрия тела:\s*доказана/);assert.match(text,/Непрерывная ошибка границы:\s*не доказана/)}
  const toSolid=async()=>{const b=page.getByRole('button',{name:'В Solid',exact:true});if(!await b.isVisible())await page.locator('.editor-toolbar .more-menu > summary').click();await b.click()}
  const done=()=>page.waitForFunction(()=>![...document.querySelectorAll('.editor-toolbar button')].some(b=>b.textContent.trim()==='…'),{},{timeout:120000})
  const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
  const download=async()=>{const wasOpen=await sourceButton.getAttribute('aria-pressed')==='true';if(wasOpen)await sourceButton.click();const menu=solid.locator('summary[title="Файл"]');if(await menu.evaluate(e=>!e.parentElement.open))await menu.click();const waiting=page.waitForEvent('download');await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click();const item=await waiting;assert.equal(await item.failure(),null);const data=JSON.parse(await readFile(await item.path(),'utf8'));if(await menu.evaluate(e=>e.parentElement.open))await menu.click();if(wasOpen)await open();return data}
  await good();entry.checks.push('native-body-evidence')
  await page.locator('textarea.code-input').fill(source+'\n// Cancelled revision\n');await page.evaluate(()=>window.profileHoldBuild=true);await page.getByRole('button',{name:'Собрать',exact:true}).click();await page.waitForFunction(()=>window.profileHeld.includes('build'));await page.getByRole('button',{name:'Отменить сборку',exact:true}).click();await page.waitForFunction(()=>!document.body.innerText.includes('Отменить сборку'));assert.equal(await page.getByTestId('sweep-final-evidence').count(),0);entry.checks.push('build-cancellation')
  await page.evaluate(()=>window.profileHoldBuild=false);await good()
  await page.evaluate(()=>window.profileHoldSolid=true);await toSolid();await page.waitForFunction(()=>window.profileHeld.includes('solid'));await page.getByRole('button',{name:'Отмена',exact:true}).click();await page.waitForFunction(()=>window.profileTerminated.includes('solid'));await done();assert.equal((await download()).bodies.length,0);entry.checks.push('solid-cancellation')
  const before=await page.evaluate(()=>window.profileTerminated.length);await toSolid();await page.waitForFunction(()=>window.profileHeld.filter(x=>x==='solid').length===2);await page.locator('textarea.code-input').fill(source+'\n// Superseded Solid source\n');await page.waitForFunction(n=>window.profileTerminated.length>n,before);await done();assert.equal((await download()).bodies.length,0);entry.checks.push('source-change')
  await page.evaluate(()=>window.profileHoldSolid=false);await load(source);await toSolid();await page.waitForFunction(()=>document.querySelector('main.main')?.inert===true,{},{timeout:120000});await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden',timeout:120000});const published=await download();assert.equal(published.bodies.length,1);assert.ok(published.bodies[0].brep.faces.length>0);entry.checks.push('solid-publication')
  await solid.getByRole('button',{name:'Вписать',exact:true}).last().click();
  const canvas=solid.locator('canvas.gpu-layer');
  if(await canvas.isVisible()){await solid.locator('.fps-badge').filter({hasText:/draw [\d.]+ ms/}).waitFor();entry.checks.push('visible-gpu-draw')}else{assert.ok(await solid.locator('svg[aria-label="Холст тел 3D"] polygon').count()>0);entry.checks.push('visible-svg-geometry')}
  await page.screenshot({path:path.join(output,`${width}-${file}.png`),fullPage:true});await solid.getByRole('status',{name:'Сохранено в браузере',exact:true,includeHidden:true}).waitFor({state:'attached'});await page.reload();await solid.waitFor();await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden',timeout:120000});assert.deepEqual((await download()).bodies[0].brep,published.bodies[0].brep);entry.checks.push('restoration')
  await open();await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck();const invalid=source.replace('values:[1,1]','values:[0,1]');assert.notEqual(invalid,source);await load(invalid);await build();const error=page.locator('.message.error');await error.waitFor();assert.match(await error.innerText(),/positive|scale/i);await toSolid();await done();await error.waitFor();assert.deepEqual((await download()).bodies[0].brep,published.bodies[0].brep);entry.checks.push('refusal-preserves-body')
  await good();assert.deepEqual(entry.errors,[]);entry.checks.push('recovery');entry.status='passed'
 }catch(error){entry.status='failed';entry.error=error.message;entry.bodyText=(await page.locator('body').innerText()).slice(-10000);await page.screenshot({path:path.join(output,`${width}-${file}-failure.png`),fullPage:true});throw error}
 finally{await save();await context.close()}
}report.passed=true;await save()}
finally{await browser.close();await new Promise(resolve=>server.close(resolve))}
console.log(report.cases.length,'profile Solid browser cases passed')
