import {chromium} from 'playwright'
import {createServer,preview} from 'vite'
import {mkdir,writeFile,readFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
const output=process.env.OFFSET_DEVIATION_OUTPUT||'output/qualification/offset-deviation'
const surface=z=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
 controlPoints:[[[0,0,z],[0,10,z]],[[10,0,z],[10,10,z]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false})
const fixture={version:1,sketches:[],bodies:[],curves:[],surfaces:[
 {id:'offset-source',name:'Source A',surface:surface(0),segmentsU:4,segmentsV:4},
 {id:'offset-candidate',name:'Candidate B',surface:surface(.25),segmentsU:4,segmentsV:4}]}
let server,browser,page,errors=[]
try{
 if(process.env.OFFSET_DEVIATION_PREVIEW)server=await preview({preview:{host:'127.0.0.1',port:4189,strictPort:true}})
 else {server=await createServer({server:{host:'127.0.0.1',port:4189,strictPort:true}});await server.listen()}
 browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 page=await browser.newPage({viewport:{width:1440,height:1000}})
 page.setDefaultTimeout(30000)
 page.on('pageerror',error=>{errors.push(error.message);console.error('Browser page error:',error.message)})
 await page.addInitScript(doc=>{localStorage.setItem('scad-lang','en');localStorage.setItem('scad-solid-modeler-v1',JSON.stringify(doc))},fixture)
 await page.goto('http://127.0.0.1:4189')
 const workspace=page.locator('.direct-workspace');await workspace.waitFor({state:'attached',timeout:30000})
 if(!await workspace.isVisible())await page.getByRole('button',{name:'Solid',exact:true}).click()
 await workspace.getByRole('tab',{name:'Scene',exact:true}).click()
 await workspace.getByRole('button',{name:'Source A',exact:true}).click()
 await workspace.getByRole('button',{name:'Candidate B',exact:true}).click({modifiers:['Shift']})
 const panel=workspace.locator('.offset-deviation-check')
 await panel.getByRole('button',{name:'Inspect offset A → B',exact:true}).click()
 const before=await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1')).surfaces)
 await panel.getByRole('spinbutton',{name:'Offset, mm',exact:true}).fill('.25')
 await panel.getByRole('button',{name:'Inspect · Enter',exact:true}).click()
 await panel.getByText(/Tolerance reached\. Error at most/).waitFor({timeout:120000})
 const upper=Number(await panel.locator('output').textContent());assert.ok(upper>=0&&upper<=.001)
 await mkdir(output,{recursive:true});await page.screenshot({path:output+'/accepted.png',fullPage:true})
 await panel.getByRole('spinbutton',{name:'Tolerance, mm',exact:true}).fill('0')
 assert.equal(await panel.getByRole('button',{name:'Inspect · Enter',exact:true}).isDisabled(),true)
 assert.equal(await panel.locator('[data-offset-deviation]').count(),0,'Changing tolerance must clear the old report')
 await panel.getByRole('spinbutton',{name:'Tolerance, mm',exact:true}).fill('.001')
 await panel.getByRole('spinbutton',{name:'Offset, mm',exact:true}).fill('1')
 await panel.getByRole('button',{name:'Inspect · Enter',exact:true}).click()
 await panel.getByText(/Tolerance exceeded\. Error at least/).waitFor({timeout:120000})
 assert.ok(Number(await panel.locator('output').textContent())>.001)
 const distancePanel=workspace.locator('[aria-label="surface-distance"]')
 await distancePanel.getByRole('button',{name:'Distance between surfaces',exact:true}).click()
 await distancePanel.locator('[data-surface-distance]').waitFor({timeout:120000})
 const distanceText=await distancePanel.locator('[data-surface-distance]').textContent()
 assert.ok(distanceText.includes('0.25'))
 const after=await page.evaluate(()=>JSON.parse(localStorage.getItem('scad-solid-modeler-v1')).surfaces)
 assert.deepEqual(after,before,'Offset inspection must preserve both saved surfaces')
 await page.screenshot({path:output+'/mismatch.png',fullPage:true})
 assert.deepEqual(errors,[])
 const wasm=await readFile('public/wasm/geometry-kernel.wasm')
 const result={browser:browser.version(),wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,
  wholeDomainUpper:upper,surfaceDistance:distanceText,accepted:true,mismatch:true,invalidToleranceDisabled:true,staleReportCleared:true,surfacesUnchanged:true,pageErrors:errors}
 await writeFile(output+'/browser.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result))
}catch(error){
 await mkdir(output,{recursive:true})
 await writeFile(output+'/failure.json',JSON.stringify({error:String(error),pageErrors:errors,url:page?.url()},null,2)+'\n')
 if(page&&!page.isClosed())await page.screenshot({path:output+'/failure.png',fullPage:true}).catch(()=>{})
 throw error
}finally{try{await browser?.close()}finally{await server?.close()}}
