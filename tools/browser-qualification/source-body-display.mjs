import {chromium} from 'playwright'
import {preview} from 'vite'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {gunzipSync} from 'node:zlib'
import assert from 'node:assert/strict'
const fixture=process.env.SOURCE_DISPLAY_FIXTURE?JSON.parse(await readFile(process.env.SOURCE_DISPLAY_FIXTURE,'utf8')):JSON.parse(gunzipSync(await readFile('docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-document.json.gz')).toString())
const expectedEdges=Number(process.env.SOURCE_DISPLAY_EXPECTED_EDGES||20)
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
 await page.waitForFunction(count=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===count,expectedEdges,{timeout:120000})
 assert.equal(await edges.count(),expectedEdges)
 const faceTriangles=await workspace.locator('[data-source-face-preview] polygon').count(),unresolvedFaces=await workspace.locator('[data-source-face-preview] rect').count()
 if(process.env.SOURCE_DISPLAY_FIXTURE){assert.ok(faceTriangles>0);assert.ok(unresolvedFaces>0)}else{assert.equal(faceTriangles,1536);assert.equal(unresolvedFaces,0)}
 await workspace.getByRole('button',{name:'Fit',exact:true}).last().click()
 const location=await edges.first().evaluate(edge=>{
  const p=edge.getPointAtLength(edge.getTotalLength()/2),screen=new DOMPoint(p.x,p.y).matrixTransform(edge.getScreenCTM())
  return {x:screen.x,y:screen.y}
 })
 await page.mouse.move(location.x,location.y);await page.mouse.click(location.x,location.y)
 assert.equal(await workspace.locator('[data-source-body-preview=edges] [aria-pressed=true]').count(),1)
 const edge=edges.first();await edge.focus();await edge.press('Enter');await edge.waitFor({state:'attached'})
 assert.equal(await edge.getAttribute('aria-pressed'),'true')
 await edge.press('Escape');assert.equal(await edge.getAttribute('aria-pressed'),'false')
 await workspace.getByRole('button',{name:'Retry source edges',exact:true}).click()
 await page.waitForFunction(count=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===count,expectedEdges,{timeout:120000})
 await mkdir(output,{recursive:true});await page.screenshot({path:output+'/preview.png',fullPage:true})
 await page.reload();await workspace.waitFor({state:'attached'})
 if(!await workspace.isVisible())await page.getByRole('button',{name:'Solid',exact:true}).click()
 await page.waitForFunction(count=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===count,expectedEdges,{timeout:120000})
 await workspace.getByRole('tab',{name:'Scene',exact:true}).click()
 const name=fixture.sourceBodies[0].name
 await workspace.getByRole('button',{name:'Tools',exact:true}).click()
 await workspace.getByRole('button',{name:'Box',exact:true}).click()
 await page.waitForFunction(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1'))?.bodies.length===1,{},{timeout:120000})
 await workspace.getByRole('tab',{name:'Scene',exact:true}).click()
 assert.equal(await workspace.locator('[data-scene-key^="object:"] button[aria-pressed=true]').count(),1)
 await workspace.getByRole('button',{name,exact:true}).click()
 assert.equal(await workspace.locator('[data-scene-key^="object:"] button[aria-pressed=true]').count(),0)

 await workspace.getByRole('button',{name:'Lock: '+name,exact:true}).click()
 assert.equal(await workspace.getByRole('button',{name,exact:true}).isDisabled(),true)
 await workspace.getByRole('button',{name:'Unlock: '+name,exact:true}).click()
 await workspace.getByRole('button',{name:'Isolate: '+name,exact:true}).click()
 await workspace.getByRole('button',{name:'Isolate: '+name,exact:true}).click()
 await workspace.getByRole('button',{name:'Hide: '+name,exact:true}).click()
 await page.waitForFunction(()=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===0)
 await page.reload();await workspace.waitFor({state:'attached'})
 if(!await workspace.isVisible())await page.getByRole('button',{name:'Solid',exact:true}).click()
 await workspace.getByRole('tab',{name:'Scene',exact:true}).click()
 await workspace.getByRole('button',{name:'Show: '+name,exact:true}).waitFor({timeout:120000})
 assert.equal(await edges.count(),0)
 await workspace.getByRole('button',{name:'Show: '+name,exact:true}).click()
 await page.waitForFunction(count=>document.querySelectorAll('[data-source-body-preview="edges"] polyline').length===count,expectedEdges,{timeout:120000})
 await workspace.getByRole('button',{name:'Manufacturing: G-code / Laser',exact:true}).click()
 await workspace.getByText('Native source bodies need a closed mesh before manufacturing.',{exact:false}).waitFor()
 assert.deepEqual(errors,[])
 await writeFile(output+'/report.json',JSON.stringify({passed:true,edgeCount:expectedEdges,faceTriangles,unresolvedFaces,manufacturingRestriction:true,selectionHandoff:true,visibilityPersistence:true,lock:true,isolation:true,mouseSelection:true,keyboardSelection:true,retry:true,reload:true,errors,scope:process.env.SOURCE_DISPLAY_FIXTURE?'Root-valued native source body display with explicit unresolved face bands; no geometry editing or closed mesh qualification':'Equal-radius native source canal edge display; face preview without geometry editing or mesh qualification'},null,2))
}finally{await browser?.close();await server?.httpServer.close()}
