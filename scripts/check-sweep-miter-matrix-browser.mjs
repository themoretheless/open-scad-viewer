import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const dist=path.resolve('dist')
const output=path.resolve(process.argv[2]??'docs/qualification/sweep-ui-matrix-2026-10-03')
const filter=process.argv.find(a=>a.startsWith('--mode='))?.slice(7)
const widthFilter=Number(process.argv.find(a=>a.startsWith('--width='))?.slice(8))
const sources=[
 'miter-periodic-hollow.r','miter-unclamped-hollow.r',
 'progressive-miter-reconstructed-conic-half.r','progressive-miter-reconstructed-conic-one.r','progressive-miter-reconstructed-conic-two.r',
 'closed-miter-frame-guide-affine-hollow-corrected.r','closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r',
 'progressive-miter-reconstructed-guide-frame-affine-hollow.r','progressive-miter-reconstructed-moving-frame-guide-affine-hollow.r',
 'miter-combined-frame-affine-hollow.r','miter-affine-hollow.r',
 'miter-combined-frame-guide-affine-hollow.r','miter-combined-frame-guide-affine-hollow-corrected.r','miter-moving-frame-guide-affine-hollow-corrected.r','miter-authored-frame-hollow.r','miter-guide-hollow.r','miter-guide-affine-hollow.r',
 'miter-combined-frame-affine-hollow-corrected.r','miter-authored-frame-hollow-corrected.r',
 'miter-guide-hollow-corrected.r','miter-guide-affine-hollow-corrected.r',
 'miter-unsegmented-plain-hollow.r','miter-unsegmented-affine-hollow.r',
 'miter-unsegmented-frame-affine-hollow.r','miter-unsegmented-guide-affine-hollow.r',
 'miter-hollow-body.r','miter-hollow-corrected.r','closed-miter-hollow-body.r',
 'progressive-miter-certified-hollow.r','progressive-miter-circle-corrected-hollow.r','progressive-miter-spatial-circle-corrected-hollow.r','progressive-miter-affine-circle-corrected-hollow.r','progressive-miter-oblique-circle-corrected-hollow.r','miter-rational-curved-guide-frame-affine-hollow.r','progressive-miter-station-g2-hollow.r','progressive-miter-reconstructed-stations.r','progressive-miter-reconstructed-sharp.r','closed-miter-frame-guide-affine-hollow.r','miter-g1-profile-frame-guide-affine.r',
].filter(file=>!filter||file===filter)
assert.ok(sources.length,'Unknown mode filter')
const viewports=[{width:1440,height:1000},{width:600,height:1000}].filter(v=>!widthFilter||v.width===widthFilter)
assert.ok(viewports.length,'Unknown viewport filter')
await mkdir(output,{recursive:true})
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const report={schema:'sweep-miter-ui-matrix/1',recordedAt:new Date().toISOString(),
 scope:'Finite retained-miter UI scenarios; held dispatch tests lifecycle cancellation, not mid-kernel interruption latency. Headless CPU fallback is not GPU qualification.',
 geometryWasmSha256:sha(await readFile('public/wasm/geometry-kernel.wasm')),
 appSourceSha256:sha(await readFile('src/App.vue')),distIndexSha256:sha(await readFile(path.join(dist,'index.html'))),cases:[]}
report.sourceHashes = Object.fromEntries(await Promise.all([
 'src/App.vue', 'src/services/sweepRetainedCorrespondence.ts', 'src/services/geometry/brep.ts',
 'src/features/DirectModeler.vue', 'src/generated/geometry-kernels/bytes.ts',
].map(async file => [file, sha(await readFile(file))])))
const save=()=>writeFile(path.join(output,'matrix.json'),JSON.stringify(report,null,2)+'\n')
const server=createServer(async(req,res)=>{
 try {
  const url=new URL(req.url,'http://localhost')
  if(url.pathname==='/favicon.ico'){res.writeHead(204).end();return}
  const file=path.resolve(dist,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
  if(!file.startsWith(dist+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const {playwright}=await loadQualificationPlaywrightPackage()
const browser=await playwright.chromium.launch({headless:true,channel:'chromium',args:['--enable-unsafe-webgpu']})
report.browser=browser.version()
try {
 for(const viewport of viewports)for(const file of sources){
  const source=await readFile(path.join('examples/rush',file),'utf8')
  const name=viewport.width+'-'+file.replace(/\.r$/,'')
  const directory=path.join(output,name);await mkdir(directory,{recursive:true})
  const context=await browser.newContext({viewport,acceptDownloads:true})
  const page=await context.newPage(),pageErrors=[]
  page.setDefaultTimeout(30000)
  page.on('pageerror',error=>pageErrors.push(error.message))
  page.on('console',message=>{if(message.type()==='error')pageErrors.push(message.text())})
  const entry={viewport,file,sourceSha256:sha(source),assertions:[],pageErrors,status:'running'}
  report.cases.push(entry);await save()
  try {
   await page.addInitScript(()=>{
    const NativeWorker=window.Worker
    window.sweepHeld=[];window.sweepTerminated=[];window.sweepHoldBuild=false;window.sweepHoldExact=false
    window.Worker=class extends NativeWorker {
     postMessage(message,...args){
      const mode=message?.type==='build'?'build':message?.kind==='exact-solid'?'exact':null
      if(mode&&(mode==='build'?window.sweepHoldBuild:window.sweepHoldExact)){
       this.sweepMode=mode;window.sweepHeld.push(mode);return
      }
      return super.postMessage(message,...args)
     }
     terminate(){if(this.sweepMode)window.sweepTerminated.push(this.sweepMode);return super.terminate()}
    }
   })
   await page.goto(`http://127.0.0.1:${server.address().port}`)
   const sourceButton=page.getByRole('button',{name:'Исходный код',exact:true})
   const openSource=async()=>{if(await sourceButton.getAttribute('aria-pressed')!=='true')await sourceButton.click()}
   await openSource()
   await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
   const editor=page.locator('textarea.code-input')
   const load=async(text)=>page.locator('input[accept=".scad,.r,.mg,text/plain"]').setInputFiles({name:'sweep-matrix.r',mimeType:'text/plain',buffer:Buffer.from(text)})
   const build=async()=>{
    await page.getByRole('button',{name:'Собрать',exact:true}).click()
    await page.waitForFunction(()=>{const b=[...document.querySelectorAll('.editor-toolbar button')].find(b=>b.textContent.trim()==='Собрать');return b&&!b.disabled},{},{timeout:120000})
   }
   const assertBuilt=async()=>{
    assert.equal(await page.locator('.message.error').count(),0,file)
    assert.match(await page.locator('.stats').innerText(),/Треугольники|треугольник|Triangles/)
   }
   const toSolid=async()=>{
    const button=page.getByRole('button',{name:'В Solid',exact:true})
    if(!await button.isVisible())await page.locator('.editor-toolbar .more-menu > summary').click()
    await button.click()
   }
   const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
   const download=async(label)=>{
    const sourceWasOpen=await sourceButton.getAttribute('aria-pressed')==='true'
    if(sourceWasOpen)await sourceButton.click()
    const menu=solid.locator('summary[title="Файл"]')
    if(await menu.evaluate(e=>!e.parentElement.open))await menu.click()
    const pending=page.waitForEvent('download')
    await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click()
    const item=await pending;assert.equal(await item.failure(),null)
    const file=path.join(directory,label+'.json');await item.saveAs(file)
    if(await menu.evaluate(e=>e.parentElement.open))await menu.click()
    const document=JSON.parse(await readFile(file,'utf8'))
    if(sourceWasOpen)await openSource()
    return document
   }
   const waitSolidFinished=()=>page.waitForFunction(()=>![...document.querySelectorAll('.editor-toolbar button')].some(b=>b.textContent.trim()==='…'),{},{timeout:120000})
   await load(source);await build();await assertBuilt()
   entry.assertions.push({case:'build',status:'passed'})
   entry.viewportEvidence=await page.getByTestId('sweep-final-evidence').allTextContents()
   assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*[✓?]\s*\/\s*[✓?]\s*\(\d+\)/.test(text)&&/Стыки вдоль пути:\s*(C0|G1|G2)/.test(text)), 'Profile smoothness must remain scoped and retain path C0')
   entry.assertions.push({case:'scoped-profile-smoothness-presentation',status:'passed'})
   if(file.startsWith('progressive-miter-reconstructed-conic-')||file==='closed-miter-frame-guide-affine-hollow.r'||file==='closed-miter-frame-guide-affine-hollow-corrected.r'||file==='closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r'||file==='progressive-miter-circle-corrected-hollow.r'||file==='progressive-miter-spatial-circle-corrected-hollow.r'||file==='progressive-miter-affine-circle-corrected-hollow.r'||file==='progressive-miter-oblique-circle-corrected-hollow.r'||file==='miter-rational-curved-guide-frame-affine-hollow.r'){
    assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*✓\s*\/\s*✓/.test(text)), 'Every profile seam in this mode must certify G2')
    entry.assertions.push({case:'profile-G2',status:'passed'})
   }
   if(file.startsWith('progressive-miter-reconstructed-conic-')||file==='progressive-miter-station-g2-hollow.r'||file==='progressive-miter-reconstructed-stations.r'||file==='progressive-miter-reconstructed-guide-frame-affine-hollow.r'||file==='progressive-miter-reconstructed-moving-frame-guide-affine-hollow.r'){
    assert.ok(entry.viewportEvidence.some(text=>/Стыки вдоль пути:\s*G2/.test(text)), 'Actual station G2 must reach viewport')
    entry.assertions.push({case:'station-G2',status:'passed'})
   }
   if(file==='progressive-miter-reconstructed-sharp.r'){
    assert.ok(entry.viewportEvidence.some(text=>/Стыки вдоль пути:\s*C0/.test(text)), 'Original sharp miter must remain C0')
    entry.assertions.push({case:'original-sharp-station-C0',status:'passed'})
   }
   if(file==='miter-g1-profile-frame-guide-affine.r'){
    assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*✓\s*\/\s*\?/.test(text)), 'Independent G1 must survive unproved G2')
    entry.assertions.push({case:'independent-G1-with-unproved-G2',status:'passed'})
   }
   if(file!=='miter-hollow-body.r'&&file!=='miter-hollow-corrected.r'&&file!=='closed-miter-hollow-body.r'){
    assert.ok(entry.viewportEvidence.some(text=>/Непрерывная ошибка границы:\s*доказана/.test(text)), 'Complete progressive boundary proof must reach the viewport')
    assert.ok(entry.viewportEvidence.some(text=>text.includes('в допуске')), 'Complete boundary proof must remain within its source budget')
    entry.assertions.push({case:'continuous-boundary-presentation',status:'passed'})
   }
   if(file==='progressive-miter-reconstructed-stations.r'){
    for(const [label,bad,reason]of [
     ['reconstruction-work-refusal',source.replace('quantum: 0.125mm, max_work: 10000','quantum: 0.125mm, max_work: 1'),/work-limit/],
     ['complete-bound-refusal',source.replace('max_deviation: 2mm','max_deviation: 1mm'),/complete boundary error/],
    ]){
     await load(bad);await build();assert.match(await page.locator('.message.error').innerText(),reason)
     assert.equal(await page.getByTestId('sweep-final-evidence').count(),0,'Refusal must clear final certificate presentation')
     entry.assertions.push({case:label,status:'passed'})
     await load(source);await build();await assertBuilt()
    }
   }
   await page.screenshot({path:path.join(directory,'preview.png'),fullPage:true})
   await editor.fill(source+'\n// New source revision whose build will be cancelled.\n')
   await page.evaluate(()=>window.sweepHoldBuild=true)
   await page.getByRole('button',{name:'Собрать',exact:true}).click()
   await page.waitForFunction(()=>window.sweepHeld.includes('build'))
   await page.getByRole('button',{name:'Отменить сборку',exact:true}).click()
   await page.waitForFunction(()=>!document.body.innerText.includes('Отменить сборку'))
   assert.equal(await page.getByTestId('sweep-final-evidence').count(),0,'Cancelled build must not retain final certificate presentation')
   entry.assertions.push({case:'build-cancellation',status:'passed',fault:'held worker dispatch'})
   await page.evaluate(()=>window.sweepHoldBuild=false)
   await build();await assertBuilt()
   await page.evaluate(()=>window.sweepHoldExact=true)
   await toSolid();await page.waitForFunction(()=>window.sweepHeld.includes('exact'))
   await page.getByRole('button',{name:'Отмена',exact:true}).click()
   await page.waitForFunction(()=>window.sweepTerminated.includes('exact'))
   await waitSolidFinished()
   assert.equal((await download('cancelled')).bodies.length,0)
   entry.assertions.push({case:'solid-cancellation',status:'passed',fault:'held worker dispatch'})
   const before=await page.evaluate(()=>window.sweepTerminated.length)
   await toSolid();await page.waitForFunction(()=>window.sweepHeld.filter(x=>x==='exact').length===2)
   await editor.fill(source+'\n// Source supersedes the held Solid request.\n')
   await page.waitForFunction(n=>window.sweepTerminated.length>n,before)
   await waitSolidFinished()
   assert.equal((await download('superseded')).bodies.length,0)
   entry.assertions.push({case:'source-change',status:'passed',fault:'held worker dispatch'})
   await page.evaluate(()=>window.sweepHoldExact=false)
   await load(source);await toSolid()
   if(file==='miter-hollow-body.r'){
    await waitSolidFinished()
    const error=page.locator('.message.error');await error.waitFor()
    assert.match(await error.innerText(),/Solid geometry.*boundary embedding/i)
    assert.equal((await download('unproved-solid-refused')).bodies.length,0)
    entry.assertions.push({case:'unproved-spatial-solid-refusal',status:'passed'})
    await solid.getByRole('status',{name:'Сохранено в браузере',exact:true,includeHidden:true}).waitFor({state:'attached'})
    await page.reload();await solid.waitFor()
    await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden',timeout:120000})
    assert.equal((await download('refused-restored')).bodies.length,0)
    entry.assertions.push({case:'refused-empty-project-restoration',status:'passed'})
    await openSource();await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()

    const invalid=source.replace(/\bpoints: \[\[([^\]]+)\],\[[^\]]+\]/,(_,first)=>`points: [[${first}],[${first}]`)
    assert.notEqual(invalid,source)
    await load(invalid);await build();await page.locator('.message.error').waitFor()
    entry.assertions.push({case:'invalid-path-build-refusal',status:'passed'})
    await toSolid();await waitSolidFinished();await page.locator('.message.error').waitFor()
    assert.equal((await download('invalid-refused')).bodies.length,0)
    entry.assertions.push({case:'invalid-path-solid-refusal',status:'passed',emptyProjectPreserved:true})
    assert.deepEqual(pageErrors,[])
    entry.status='passed'
    console.log(viewport.width,file,entry.assertions.length,'checks passed (Solid refused)')
    continue
   }
   await page.waitForFunction(()=>document.querySelector('main.main')?.inert===true,{},{timeout:120000})
   await solid.getByRole('status',{name:'display-refinement',exact:true}).waitFor({state:'hidden',timeout:120000})
   const document=await download('solid')
   assert.equal(document.bodies.length,1)
   assert.ok(document.bodies[0].brep?.faces.length>0)
   entry.solid={faces:document.bodies[0].brep.faces.length,shells:document.bodies[0].brep.shells.length,
    geometrySha256:sha(JSON.stringify(document.bodies[0].brep))}
   entry.assertions.push({case:'solid-publication',status:'passed'})
   await solid.getByRole('button',{name:'Вписать',exact:true}).last().click()
   const gpuCanvas=solid.locator('canvas.gpu-layer')
   if(await gpuCanvas.isVisible()){
    await page.waitForFunction(canvas=>canvas.width>1&&canvas.height>1,await gpuCanvas.elementHandle())
    await solid.locator('.fps-badge').filter({hasText:/draw [\d.]+ ms/}).waitFor()
    entry.displayBackend='webgpu'
   }else{
    assert.ok(await solid.locator('svg[aria-label="Холст тел 3D"] polygon').count()>0)
    entry.displayBackend='svg'
   }
   await page.screenshot({path:path.join(directory,'solid.png'),fullPage:true})
   await solid.getByRole('status',{name:'Сохранено в браузере',exact:true,includeHidden:true}).waitFor({state:'attached'})
   await page.reload();await solid.waitFor()
   await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden',timeout:120000})
   const restored=await download('restored')
   assert.equal(restored.bodies.length,1)
   assert.deepEqual(restored.bodies[0].brep,document.bodies[0].brep)
   entry.assertions.push({case:'restore',status:'passed'})
   await openSource();await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
    if(file==='miter-moving-frame-guide-affine-hollow-corrected.r'){
     const overBudget=source.replace('max_deviation: 0.6mm','max_deviation: 0.45mm');assert.notEqual(overBudget,source)
     await load(overBudget);await build();await page.locator('.message.error').waitFor()
     assert.match(await page.locator('.message.error').innerText(),/complete boundary error exceeds max_deviation/)
     entry.assertions.push({case:'full-boundary-budget-build-refusal',status:'passed'})
     await toSolid();await waitSolidFinished();await page.locator('.message.error').waitFor()
     const preserved=await download('boundary-budget-refused');assert.equal(preserved.bodies.length,1);assert.deepEqual(preserved.bodies[0].brep,document.bodies[0].brep)
     entry.assertions.push({case:'full-boundary-budget-solid-refusal',status:'passed'})
    }
   const invalid=source.replace(/\bpoints: \[\[([^\]]+)\],\[[^\]]+\]/,(_,first)=>`points: [[${first}],[${first}]`)
   assert.notEqual(invalid,source,'Negative fixture must create a zero-length path edge')
   await load(invalid);await build()
   const error=page.locator('.message.error');await error.waitFor()
   entry.refusalMessage=await error.innerText();assert.match(entry.refusalMessage,/edge|length|degener|point|miter|zero/i)
   entry.assertions.push({case:'invalid-path-build-refusal',status:'passed'})
   await toSolid();await waitSolidFinished();await error.waitFor()
   const refused=await download('refused')
   assert.equal(refused.bodies.length,1)
   assert.deepEqual(refused.bodies[0].brep,document.bodies[0].brep)
   entry.assertions.push({case:'invalid-path-solid-refusal',status:'passed',retainedBodyPreserved:true})
   const resourceLimited=source.includes('cap_correction_max_work:')
    ?source.replace(/cap_correction_max_work:\s*\d+/,'cap_correction_max_work: 1')
    :source.includes('brep_progressive_miter_sweep')
     ?source.replace(/max_steps:\s*\d+/,'max_steps: 1').replace(/max_deviation:\s*[\d.eE+-]+mm/,'max_deviation: 0.000000000000000000000000000001mm')
     :null
   if(resourceLimited){
    assert.notEqual(resourceLimited,source)
    await load(resourceLimited);await build();await error.waitFor()
    entry.resourceRefusalMessage=await error.innerText()
    assert.match(entry.resourceRefusalMessage,/budget|work|step|deviation|certif|unproved|could not be proved/i)
    await toSolid();await waitSolidFinished();await error.waitFor()
    const resourceRefused=await download('resource-refused')
    assert.equal(resourceRefused.bodies.length,1)
    assert.deepEqual(resourceRefused.bodies[0].brep,document.bodies[0].brep)
    entry.assertions.push({case:'bounded-work-refusal',status:'passed',retainedBodyPreserved:true})
   }
   assert.deepEqual(pageErrors,[])
   entry.status='passed'
   console.log(viewport.width,file,entry.assertions.length,'checks passed')
  }catch(error){
   entry.status='failed';entry.error=error.message
   entry.bodyText=(await page.locator('body').innerText()).slice(-10000)
   await page.screenshot({path:path.join(directory,'failure.png'),fullPage:true})
   throw error
  }finally{await save();await context.close()}
 }
 assert.ok(report.cases.every(c=>c.status==='passed'))
 report.passed=true;await save()
}finally{await browser.close();await new Promise(resolve=>server.close(resolve))}
