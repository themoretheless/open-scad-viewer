import {createServer} from 'vite'
import {chromium} from 'playwright'
import {mkdir,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import assert from 'node:assert/strict'
const directory=resolve(process.argv[2]??'output/qualification/forma-regions-browser');await mkdir(directory,{recursive:true})
const server=await createServer({root:process.cwd(),cacheDir:resolve(directory,'vite-cache'),optimizeDeps:{noDiscovery:true,include:['vue']},server:{host:'127.0.0.1',port:0,strictPort:true,open:false}})
let browser;const report={schema:'forma-regions-browser/1',passed:false,cases:[],errors:[],scope:'Actual browser DOM and interaction of finite App-state Forma region fixtures at desktop/mobile widths. No full application, GPU computation, CAD geometry or production runtime admission claim.'}
try{
 await server.listen();const port=server.httpServer.address().port;browser=await chromium.launch({headless:true})
 for(const width of [1440,390]){
  const page=await browser.newPage({viewport:{width,height:900}});page.on('pageerror',e=>report.errors.push(String(e)))
  await page.goto(`http://127.0.0.1:${port}/tests/fixtures/forma-regions-browser.html`)
  await page.getByRole('button',{name:'Mesh',exact:true}).click();await page.getByRole('button',{name:'Source',exact:true}).click()
  assert.equal(await page.getByRole('button',{name:'Mesh',exact:true}).getAttribute('aria-pressed'),'true')
  assert.equal(await page.getByRole('button',{name:'Source',exact:true}).getAttribute('aria-pressed'),'true')
  await page.getByRole('checkbox',{name:'Auto'}).uncheck();await page.getByRole('button',{name:'Build',exact:true}).click();await page.getByRole('button',{name:'Cancel build',exact:true}).click()
  await page.getByRole('button',{name:'Edges',exact:true}).click();assert.equal(await page.getByRole('button',{name:'Edges',exact:true}).getAttribute('aria-pressed'),'true')
  await page.getByRole('tab',{name:'Inspector',exact:true}).click();assert.equal(await page.getByRole('tab',{name:'Inspector',exact:true}).getAttribute('tabindex'),'0')
  await page.getByRole('button',{name:'Isolate',exact:true}).click();assert.equal(await page.getByRole('button',{name:'Isolate',exact:true}).isEnabled(),true)
  await page.getByRole('button',{name:'Compute',exact:true}).click();await page.getByRole('button',{name:'Cancel',exact:true}).click()
  for(const name of ['Material','Reference','More','Settings']) assert.equal(await page.getByRole('button',{name,exact:true}).count(),1)
  assert.equal(await page.getByLabel('File',{exact:true}).count(),1)
  const observed=await page.evaluate(()=>({state:window.__formaFixture.state,events:window.__formaFixture.events,overflow:document.documentElement.scrollWidth-innerWidth}))
  assert.equal(observed.state.autoRender,false);assert.equal(observed.state.rendering,false);assert.equal(observed.state.computeBusy,false);assert.ok(observed.overflow<=1,`Horizontal overflow ${observed.overflow} at ${width}`)
  await page.screenshot({path:resolve(directory,`forma-${width}.png`),fullPage:true});report.cases.push({width,events:observed.events,overflow:observed.overflow});await page.close()
 }
 assert.equal(report.errors.length,0);report.passed=true
}finally{await writeFile(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n');await browser?.close();await server.close()}
console.log(JSON.stringify({passed:report.passed,cases:report.cases.length,errors:report.errors.length}))
