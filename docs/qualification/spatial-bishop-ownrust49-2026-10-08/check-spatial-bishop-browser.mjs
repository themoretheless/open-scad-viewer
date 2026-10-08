import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const dist=path.resolve('dist'),output=path.resolve(process.argv[2]??'/tmp/spatial-bishop-browser')
await mkdir(output,{recursive:true})
const sha=data=>createHash('sha256').update(data).digest('hex')
const catalogBytes=await readFile('docs/design/sweep-qualification-catalog.json')
const catalog=JSON.parse(catalogBytes)
const report={schema:'spatial-bishop-browser/1',scope:'Finite spatial G2 surface Rush rendering and typed scale refusal; no closed Solid shell proof.',geometryWasmSha256:sha(await readFile('public/wasm/geometry-kernel.wasm')),distIndexSha256:sha(await readFile(path.join(dist,'index.html'))),selectionCatalogSha256:sha(catalogBytes),cases:[]}
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
try{for(const width of [1440,600]){
 const file='spatial-bishop-g2-surface-6.r',source=await readFile(path.join('examples/rush',file),'utf8')
 const context=await browser.newContext({viewport:{width,height:1000}}),page=await context.newPage()
 const entry={width,file,sourceSha256:sha(source),checks:[],errors:[],status:'running'};report.cases.push(entry)
 page.on('pageerror',e=>entry.errors.push(e.message));page.setDefaultTimeout(30000)
 try{
  await page.goto(`http://127.0.0.1:${server.address().port}`)
  const sourceButton=page.getByRole('button',{name:'Исходный код',exact:true})
  if(await sourceButton.getAttribute('aria-pressed')!=='true')await sourceButton.click()
  await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
  const load=text=>page.locator('input[accept=".scad,.r,.mg,text/plain"]').setInputFiles({name:'spatial.r',mimeType:'text/plain',buffer:Buffer.from(text)})
  const build=async()=>{await page.getByRole('button',{name:'Собрать',exact:true}).click();await page.waitForFunction(()=>{const b=[...document.querySelectorAll('.editor-toolbar button')].find(b=>b.textContent.trim()==='Собрать');return b&&!b.disabled},{},{timeout:120000})}
  await load(source);await build();assert.equal(await page.locator('.message.error').count(),0);entry.checks.push('spatial-rush-build')
  await page.locator('.editor-toolbar .more-menu > summary').click();await page.getByRole('button',{name:'Перенести сцену в Mesh',exact:true}).click();await sourceButton.click()
  const svg=page.locator('.mesh-workspace svg.mesh-view');await svg.waitFor();assert.equal(await svg.locator('polygon.face').count(),1024);await svg.hover();for(let i=0;i<20;i++)await page.mouse.wheel(0,-100);
  await page.waitForFunction(()=>document.querySelector('.mesh-view > g')?.getAttribute('transform')?.includes('scale(8)')); await page.screenshot({path:path.join(output,`${width}-${file}.png`),fullPage:true});entry.checks.push('mesh-svg-1024-faces');await sourceButton.click()
  const invalid=source.replace('values:[1,1]','values:[0,1]');assert.notEqual(invalid,source);await load(invalid);await build();await page.locator('.message.error').waitFor();entry.checks.push('invalid-scale-refusal')
  await load(source);await build();assert.equal(await page.locator('.message.error').count(),0);entry.checks.push('fresh-success-after-refusal')
  assert.deepEqual(entry.errors,[]);entry.status='passed'
 }catch(error){entry.status='failed';entry.error=error.message;entry.bodyText=(await page.locator('body').innerText()).slice(-10000);await page.screenshot({path:path.join(output,`${width}-failure.png`),fullPage:true});throw error}
 finally{await context.close();await save()}
}}finally{await browser.close();server.close();await save()}
console.log(`${report.cases.length} spatial Bishop browser cases passed`)
