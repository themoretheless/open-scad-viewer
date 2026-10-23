import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const output=path.resolve(process.argv[2]??'/tmp/scalar-sweep-browser'),dist=path.resolve('dist')
await mkdir(output,{recursive:true})
const hash=bytes=>createHash('sha256').update(bytes).digest('hex')
const report={schema:'scalar-sweep-browser/1',scope:'Finite browser build, refusal and recovery scenarios; no Solid or global geometry guarantee.',recordedAt:new Date().toISOString(),geometryWasmSha256:hash(await readFile('public/wasm/geometry-kernel.wasm')),distIndexSha256:hash(await readFile(path.join(dist,'index.html'))),cases:[]}
const server=createServer(async(req,res)=>{try{const url=new URL(req.url,'http://localhost');if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}const file=path.resolve(dist,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)));if(!file.startsWith(dist+path.sep)){res.writeHead(403).end();return}res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream');res.end(await readFile(file))}catch{res.writeHead(404).end()}})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const {playwright}=await loadQualificationPlaywrightPackage(),browser=await playwright.chromium.launch({headless:true,channel:'chromium',args:['--enable-unsafe-webgpu']})
report.browser=browser.version()
try{
 for(const width of [1440,600]){
  const context=await browser.newContext({viewport:{width,height:1000}}),page=await context.newPage(),errors=[]
  page.on('pageerror',e=>errors.push(e.message))
  try{
   await page.goto(`http://127.0.0.1:${server.address().port}`)
   const sourceButton=page.getByRole('button',{name:'Исходный код',exact:true});if(await sourceButton.getAttribute('aria-pressed')!=='true')await sourceButton.click()
   await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
   const build=async(source)=>{await page.locator('input[accept=".scad,.r,.mg,text/plain"]').setInputFiles({name:'scalar-sweep.r',mimeType:'text/plain',buffer:Buffer.from(source)});await page.getByRole('button',{name:'Собрать',exact:true}).click();await page.waitForFunction(()=>{const b=[...document.querySelectorAll('.editor-toolbar button')].find(b=>b.textContent.trim()==='Собрать');return b&&!b.disabled},{},{timeout:120000})}
   const good=async(source,label)=>{await build(source);assert.equal(await page.locator('.message.error').count(),0,label);assert.match(await page.locator('.stats').innerText(),/Треугольники|треугольник|Triangles/);report.cases.push({width,label,sourceSha256:hash(Buffer.from(source)),status:'passed'});await page.screenshot({path:path.join(output,width+'-'+label+'.png')})}
   const scaled=await readFile('examples/rush/scaled-sweep-weighted.r','utf8'),profile=await readFile('examples/rush/profile-sweep-scaled.r','utf8')
   await good(scaled,'scaled-rational');await good(profile,'profile-rmf')
   const curved=profile.replace('[[0,0,0],[0,0,8mm]]','[[0,0,0],[0,0,4mm],[4mm,0,8mm]]').replace('sections:17','sections:3').replace('max_deviation:0.1mm','max_deviation:0.0000000001mm');assert.notEqual(curved,profile)
   await build(curved);const refusal=await page.locator('.message.error').innerText();assert.match(refusal,/continuous deviation|exceeds/i);report.cases.push({width,label:'continuous-budget-refusal',sourceSha256:hash(Buffer.from(curved)),message:refusal,status:'passed'})
   const invalid=scaled.replace('values:[1,2]','values:[0,2]');assert.notEqual(invalid,scaled);await build(invalid);const invalidMessage=await page.locator('.message.error').innerText();assert.match(invalidMessage,/positive|scale/i);report.cases.push({width,label:'positive-scale-refusal',message:invalidMessage,status:'passed'})
   await good(profile,'recovery');assert.deepEqual(errors,[])
  }finally{await context.close()}
 }
 report.passed=true
}finally{await writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n');await browser.close();await new Promise(resolve=>server.close(resolve))}
console.log(report.cases.length,'scalar sweep browser scenarios passed')
