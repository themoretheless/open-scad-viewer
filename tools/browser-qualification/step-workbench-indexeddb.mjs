// Parameterized STEP /7–/9 workbench IndexedDB lifecycle qualification.
// Usage: node tools/browser-qualification/step-workbench-indexeddb.mjs <7|8|9>
// The versioned step-vN-workbench-indexeddb.mjs files are thin wrappers kept
// for existing invocation points.
import {chromium} from 'playwright'
import {spawn} from 'node:child_process'
import {mkdir,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import {pathToFileURL} from 'node:url'

const configs={
  7:{port:4179,fixture:'tests/fixtures/step-v7/self-authored-ap242-periodic-cylinder.step',edit:'// STEP /7 lifecycle edit'},
  8:{port:4180,fixture:'tests/fixtures/step-v8/self-authored-ap242-sphere.step',edit:'// STEP /8 lifecycle edit'},
  9:{port:4181,fixture:'tests/fixtures/step-v9/self-authored-ap242-open-shell.step',edit:'// STEP /9 retained sheet lifecycle'},
}

export async function runStepWorkbenchIndexeddb(version){
  const config=configs[version]
  if(!config)throw Error(`unsupported STEP interchange version: ${version}`)
  const port=config.port,url=`http://127.0.0.1:${port}`
  const server=spawn('npm',['run','dev','--','--host','127.0.0.1','--port',String(port)],{
    cwd:process.cwd(),stdio:['ignore','pipe','pipe'],
  })
  let serverError=''
  server.stderr.on('data',chunk=>{serverError+=chunk})
  const wait=async()=>{
    for(let attempt=0;attempt<480;attempt++){
      try{if((await fetch(url)).ok)return}catch{}
      await new Promise(resolve=>setTimeout(resolve,250))
    }
    throw Error(`Vite did not start: ${serverError}`)
  }
  let browser
  try{
    await wait()
    browser=await chromium.launch({headless:true,args:['--enable-features=WebAssemblyUnlimitedSyncCompilation']})
    const page=await browser.newPage({acceptDownloads:true})
    await page.addInitScript(()=>localStorage.setItem('scad-lang','en'))
    await page.goto(url,{waitUntil:'domcontentloaded',timeout:120000})
    await page.getByRole('button',{name:'CAD operations',exact:true}).waitFor({timeout:120000})
    await page.getByRole('button',{name:'CAD operations',exact:true}).evaluate(button=>button.click())
    await page.locator('.cad-workbench').waitFor({timeout:30000})
    const retained=page.locator('.cad-workbench input[accept=".step,.stp"]').first()
    await retained.setInputFiles(resolve(config.fixture))
    await page.locator('.cad-workbench').getByText(/AP242 semantics: not-checked/).waitFor({timeout:120000})

    const editor=page.locator('textarea').first()
    await editor.fill((await editor.inputValue())+'\n'+config.edit)
    await page.getByRole('button',{name:'Save in browser',exact:true}).evaluate(button=>button.click())
    await page.getByText('Saved to IndexedDB',{exact:true}).waitFor({timeout:30000})
    let beforeReload=null
    if(version!==9){
      beforeReload=await page.evaluate(async()=>{
        const request=indexedDB.open('open-scad-viewer')
        const database=await new Promise((resolve,reject)=>{request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error)})
        const transaction=database.transaction('workspace','readonly')
        const get=transaction.objectStore('workspace').get('active')
        const row=await new Promise((resolve,reject)=>{get.onsuccess=()=>resolve(get.result);get.onerror=()=>reject(get.error)})
        database.close()
        return {source:row.snapshot.source,revision:row.snapshot.revision}
      })
      if(!beforeReload.source.includes(config.edit))throw Error('edited project was not saved to IndexedDB')
    }

    await page.reload({waitUntil:'domcontentloaded',timeout:120000})
    if(!(await editor.inputValue()).includes(config.edit))throw Error('IndexedDB project did not reload')
    await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(button=>button.textContent?.trim()==='CAD operations'))
    await page.evaluate(()=>[...document.querySelectorAll('button')].find(button=>button.textContent?.trim()==='CAD operations')?.click())
    await page.locator('.cad-workbench').waitFor({timeout:30000})
    await page.evaluate(()=>[...document.querySelectorAll('button')].find(button=>button.textContent?.trim()==='Export retained model')?.click())
    await page.locator('.cad-workbench').getByText(new RegExp(`step-interchange/${version}; identity preserved`)).waitFor({timeout:30000})
    const exported=await page.evaluate(async(version)=>{
      const store=await import('/src/services/cadStepIndexedDb.ts')
      const step=await import('/src/services/cadNurbsStep.ts')
      const row=await store.loadProjectStepModel()
      if(!row)throw Error('project STEP model is missing')
      if(version===7){
        const text=step.exportDirectStepV7(row.model).text
        if(step.importDirectStepV7(text).model.bodies.length!==1)throw Error('export reimport failed')
        return {text}
      }
      if(version===8){
        const result=step.exportDirectStepV8(row.model)
        if(!result.certificate.complete||result.certificate.regularity[0]?.carrier!=='sphere')throw Error('regularity certificate missing')
        if(step.importDirectStepV8(result.text).model.bodies.length!==1)throw Error('export reimport failed')
        return {text:result.text}
      }
      const exported=step.exportDirectStepV9(row.model)
      const imported=step.importDirectStepV9(exported.text)
      if(imported.model.bodies.length!==0||imported.model.shells.length!==1||imported.model.shells[0].closed)
        throw Error('open-shell export reimport failed')
      return {exportBytes:new TextEncoder().encode(exported.text).byteLength}
    },version)
    let result
    if(version===9){
      result={capability:'step-interchange/9',route:'retained-open-shell',
        exportBytes:exported.exportBytes,semanticConformance:'not-checked',reimported:true,
        userAgent:await page.evaluate(()=>navigator.userAgent)}
    }else{
      await retained.setInputFiles({name:'reimport.step',mimeType:'application/step',buffer:Buffer.from(exported.text)})
      await page.locator('.cad-workbench').getByText(/retained-ap242-brep/).waitFor({timeout:120000})
      result={capability:`step-interchange/${version}`,indexedDbRevision:beforeReload.revision,
        sourceBytes:new TextEncoder().encode(beforeReload.source).byteLength,exportBytes:Buffer.byteLength(exported.text),
        semanticConformance:'not-checked',...(version===8?{regularityCarrier:'sphere'}:{}),reimported:true,
        userAgent:await page.evaluate(()=>navigator.userAgent)}
    }
    const output=`output/qualification/step-interchange-${version}-v1`
    await mkdir(output,{recursive:true})
    await writeFile(`${output}/browser-workbench-indexeddb.json`,JSON.stringify(result,null,2)+'\n')
    console.log(JSON.stringify(result))
  }finally{
    await browser?.close()
    server.kill('SIGTERM')
  }
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  await runStepWorkbenchIndexeddb(Number(process.argv[2]))
}
