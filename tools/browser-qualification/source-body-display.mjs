import {chromium} from 'playwright'
import {preview} from 'vite'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {gunzipSync} from 'node:zlib'
import assert from 'node:assert/strict'
const fixture=JSON.parse(gunzipSync(await readFile('docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-document.json.gz')).toString())
const output=process.env.SOURCE_DISPLAY_OUTPUT||'output/qualification/source-body-display'
let server,browser
try{
 server=await preview({preview:{host:'127.0.0.1',port:4187,strictPort:true}})
 browser=await chromium.launch({headless:true,args:['--enable-features=WebAssemblyUnlimitedSyncCompilation'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 const page=await browser.newPage(),errors=[]
 page.on('pageerror',e=>errors.push(e.message))
 await page.addInitScript(document=>{localStorage.setItem('scad-lang','en');localStorage.setItem('scad-solid-modeler-v1',JSON.stringify(document))},fixture)
 await page.goto('http://127.0.0.1:4187')
 const workspace=page.locator('.direct-workspace');await workspace.waitFor({state:'attached'})
 if(!await workspace.isVisible())await page.getByRole('button',{name:'Solid',exact:true}).click()
 const edges=workspace.locator('[data-source-body-preview="edges"] polyline')
 await page.waitForFunction(()=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===20,{},{timeout:120000})
 assert.equal(await edges.count(),20)
 await workspace.getByRole('button',{name:'Fit',exact:true}).last().click()
 const location=await edges.first().evaluate(edge=>{
  const p=edge.getPointAtLength(edge.getTotalLength()/2),screen=new DOMPoint(p.x,p.y).matrixTransform(edge.getScreenCTM())
  return {x:screen.x,y:screen.y}
 })
 await page.mouse.move(location.x,location.y);await page.mouse.click(location.x,location.y)
 assert.equal(await workspace.locator('[data-source-body-preview=edges] [aria-pressed=true]').count(),1)
 const edge=edges.first();await edge.focus();await edge.press('Enter');await edge.waitFor()
 assert.equal(await edge.getAttribute('aria-pressed'),'true')
 await edge.press('Escape');assert.equal(await edge.getAttribute('aria-pressed'),'false')
 await workspace.getByRole('button',{name:'Retry source edges',exact:true}).click()
 await page.waitForFunction(()=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===20,{},{timeout:120000})
 await mkdir(output,{recursive:true});await page.screenshot({path:output+'/preview.png',fullPage:true})
 await page.reload();await workspace.waitFor({state:'attached'})
 if(!await workspace.isVisible())await page.getByRole('button',{name:'Solid',exact:true}).click()
 await page.waitForFunction(()=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===20,{},{timeout:120000})
 assert.deepEqual(errors,[])
 await writeFile(output+'/report.json',JSON.stringify({passed:true,edgeCount:20,mouseSelection:true,keyboardSelection:true,retry:true,reload:true,errors,scope:'Equal-radius native source canal edge display; no face rendering or geometry editing'},null,2))
}finally{await browser?.close();await server?.httpServer.close()}
