import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist'),directory=path.resolve(process.argv[2]??'/tmp/solid-palette-focus')
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
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 page=await browser.newPage();page.on('pageerror',e=>errors.push(String(e)))
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 const trigger=solid.getByRole('button',{name:'Команда… Ctrl K',exact:true})
 const search=page.getByRole('combobox',{name:'Search commands / Поиск команд'})
 let tabs=0
 async function tabTo(locator){for(let i=0;i<500;i++){if(await locator.evaluate(e=>e===document.activeElement))return;await page.keyboard.press('Tab');tabs++}throw Error('Unreachable focus target')}
 async function focused(locator){await locator.waitFor({state:'visible'});await page.waitForFunction(el=>el===document.activeElement,await locator.elementHandle())}
 await tabTo(trigger)
 for(let i=0;i<3;i++){
  await page.keyboard.press(i===1?'ControlOrMeta+k':'Enter')
  await focused(search)
  assert.equal(await search.inputValue(),'')
  await page.keyboard.insertText('not-a-command')
  await page.keyboard.press('Escape')
  await search.waitFor({state:'detached'});await focused(trigger)
 }
 await page.keyboard.press('Enter');await focused(search)
 await page.keyboard.insertText('Box');await page.keyboard.press('Enter')
 await search.waitFor({state:'detached'});await focused(trigger)
 await solid.locator('[data-body]').first().waitFor({state:'visible'})
 await page.keyboard.press('ControlOrMeta+k');await focused(search)
 await page.keyboard.press('Escape');await focused(trigger)
 assert.deepEqual(errors,[])
 await page.screenshot({path:path.join(directory,'focus.png')})
 await writeFile(path.join(directory,'result.json'),JSON.stringify({ok:true,tabs,escapeCycles:4,command:'Box',focusRestored:true,errors},null,2))
}catch(e){if(page){await page.screenshot({path:path.join(directory,'failure.png')}).catch(()=>{});await writeFile(path.join(directory,'failure.txt'),await page.locator('body').innerText().catch(()=>''))}throw e}
finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
