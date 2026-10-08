import {closedCorrectedRegularBodySources,correctedRegularBodySources,sweepBodyBoundarySources} from './sweep-body-matrix-sources.mjs'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {mkdir,readFile,readdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
import {createNativeCancelProbe} from './sweep-native-cancel-probe.mjs'

const dist=path.resolve(process.argv.find(a=>a.startsWith('--dist-directory='))?.slice('--dist-directory='.length)??'dist')
const output=path.resolve(process.argv[2]??'docs/qualification/sweep-ui-matrix-2026-10-03')
const filter=process.argv.find(a=>a.startsWith('--mode='))?.slice(7)
const widthFilter=Number(process.argv.find(a=>a.startsWith('--width='))?.slice(8))
const startAt=process.argv.find(a=>a.startsWith('--start-at='))?.slice('--start-at='.length)
const nativeRunningCancel=process.argv.includes('--native-running-cancel')
const nativeCancel=process.argv.includes('--native-cancel')||nativeRunningCancel
const surfaceMode=process.argv.includes('--surface')
const bodyBoundaryMode=process.argv.includes('--body-boundary')
const sourceDirectory=path.resolve(process.argv.find(a=>a.startsWith('--source-directory='))?.slice('--source-directory='.length)??'examples/rush')
const lawMatrixMode=process.argv.includes('--law-matrix')
assert.ok(!lawMatrixMode||!surfaceMode&&!bodyBoundaryMode,'Law matrix requires miter mode')
assert.ok(!(surfaceMode&&bodyBoundaryMode),'Select one UI matrix mode')
// Visible windows require an explicit invocation; inherited environment must
// never turn a background qualification run into a focus-stealing browser.
const headed=process.argv.includes('--headed')
const gpuBackend=process.argv.find(a=>a.startsWith('--gpu-backend='))?.slice('--gpu-backend='.length)??'auto'
assert.ok(['auto','metal','swiftshader'].includes(gpuBackend),'Unknown requested GPU backend')
const evidenceOnly=process.argv.includes('--evidence-only')
assert.ok(!evidenceOnly||surfaceMode&&!headed,'Evidence-only requires a headless surface run')
const sources=(bodyBoundaryMode?[...sweepBodyBoundarySources]:surfaceMode?['closed-antipodal-spatial-rmf-progressive-sweep.r','closed-spatial-rmf-affine-certified.r','closed-conic-direction-frame-c1-progressive-sweep.r','closed-arc-length-guided-frame-c2-progressive-sweep.r','closed-arc-length-guided-source-bound-progressive-sweep.r','closed-planar-guided-frame-c2-progressive-sweep.r','closed-planar-path-frame-c2-progressive-sweep.r','corrected-frenet-sweep.r','corrected-frenet-antiparallel-progressive-sweep.r','authored-closed-cartesian-c2-progressive-sweep.r','authored-closed-nonclamped-c2-progressive-sweep.r','mixed-knots-curved-frenet-progressive-sweep.r','authored-closed-piecewise-c2-progressive-sweep.r','decomposed-profile-g1-progressive-sweep.r','decomposed-profile-g2-progressive-sweep.r','closed-authored-arc-length-progressive-sweep.r','closed-authored-source-bound-progressive-sweep.r','authored-closed-projective-c2-progressive-sweep.r','authored-closed-multispan-c2-progressive-sweep.r','authored-closed-c2-progressive-sweep.r','nonaxial-planar-rmf-progressive-sweep.r','arc-length-curved-nonaxial-planar-rmf-progressive-sweep.r','arc-length-curved-planar-rmf-progressive-sweep.r','arc-length-curved-contact-progressive-sweep.r','arc-length-curved-guided-progressive-sweep.r','arc-length-curved-frenet-progressive-sweep.r','arc-length-curved-fixed-normal-progressive-sweep.r','arc-length-curved-fixed-progressive-sweep.r','arc-length-curved-authored-progressive-sweep.r','arc-length-authored-progressive-sweep.r','arc-length-guided-progressive-sweep.r','arc-length-progressive-sweep.r','arc-length-affine-progressive-sweep.r','planar-rmf-progressive-sweep.r','oblique-rmf-progressive-sweep.r','progressive-sweep.r','affine-progressive-sweep.r','authored-progressive-sweep.r','fixed-progressive-sweep.r','fixed-normal-progressive-sweep.r','frenet-progressive-sweep.r','spatial-frenet-progressive-sweep.r','spatial-frenet-affine-progressive-sweep.r','guided-progressive-sweep.r','contact-progressive-sweep.r','closed-guided-progressive-sweep.r','closed-contact-progressive-sweep.r','closed-fixed-progressive-sweep.r','closed-fixed-normal-progressive-sweep.r','closed-fixed-normal-full-turn-progressive-sweep.r','closed-planar-rmf-full-turn-progressive-sweep.r','closed-frenet-progressive-sweep.r']:[
 'sharp-profile-nonuniform-station-g2-miter.r',
 'miter-moving-affine-full-boundary-refined.r',
 'closed-rational-frame-guide-affine-hollow-placed.r',
 'progressive-miter-reconstructed-rational-profile.r',
 'progressive-miter-reconstructed-periodic-profile.r',
 'progressive-miter-reconstructed-conic-half.r','progressive-miter-reconstructed-conic-one.r','progressive-miter-reconstructed-conic-two.r',
 'closed-periodic-frame-guide-affine-hollow.r','closed-periodic-frame-guide-affine-hollow-placed.r','closed-miter-frame-guide-affine-hollow-corrected.r','closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r',
 'progressive-miter-reconstructed-guide-frame-affine-hollow.r','progressive-miter-reconstructed-moving-frame-guide-affine-hollow.r',
 'miter-combined-frame-affine-hollow.r','miter-affine-hollow.r','miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r','miter-periodic-moving-axis-guide-affine-hollow.r','miter-periodic-moving-axis-guide-affine-hollow-placed.r','miter-periodic-moving-axis-guide-affine-hollow-authored-caps-placed.r',
 'miter-combined-frame-guide-affine-hollow.r','miter-combined-frame-guide-affine-hollow-corrected.r','miter-moving-frame-guide-affine-hollow-corrected.r','miter-authored-frame-hollow.r','miter-guide-hollow.r','miter-guide-affine-hollow.r',
 'miter-combined-frame-affine-hollow-corrected.r','miter-authored-frame-hollow-corrected.r',
 'miter-guide-hollow-corrected.r','miter-guide-affine-hollow-corrected.r',
 'miter-unsegmented-plain-hollow.r','miter-unsegmented-affine-hollow.r',
 'miter-unsegmented-frame-affine-hollow.r','miter-unsegmented-guide-affine-hollow.r',
 'miter-hollow-body.r','miter-hollow-corrected.r','closed-miter-hollow-body.r',
 'progressive-miter-certified-hollow.r','progressive-miter-circle-corrected-hollow.r','progressive-miter-spatial-circle-corrected-hollow.r','progressive-miter-affine-circle-corrected-hollow.r','progressive-miter-oblique-circle-corrected-hollow.r','miter-rational-curved-guide-frame-affine-hollow.r','progressive-miter-station-g2-hollow.r','nonuniform-station-g2-hollow-miter.r','progressive-miter-reconstructed-stations.r','progressive-miter-reconstructed-sharp.r','closed-miter-frame-guide-affine-hollow.r','miter-g1-profile-frame-guide-affine.r',
])
const selectedSources=(lawMatrixMode?(await readdir(sourceDirectory)).filter(file=>/^miter-laws-(open|closed)-[a-z-]+\.r$/.test(file)).sort():sources).filter(file=>!filter||file===filter)
if(lawMatrixMode&&!filter){
 const expected=[false,true].flatMap(closed=>Array.from({length:8},(_,bits)=>{
  const laws=[bits&1?'affine':null,bits&2?'authored':null,bits&4?'guide':null].filter(Boolean).join('-')||'plain'
  return `miter-laws-${closed?'closed':'open'}-${laws}.r`
 })).sort()
 assert.deepEqual(selectedSources,expected,'Complete law matrix requires each open/closed affine/authored/guide combination exactly once')
}
sources.splice(0,sources.length,...selectedSources)
assert.ok(sources.length,'Unknown mode filter')
const viewports=[{width:1440,height:1000},{width:600,height:1000}].filter(v=>!widthFilter||v.width===widthFilter)
assert.ok(viewports.length,'Unknown viewport filter')
const caseKey=(viewport,file)=>`${viewport.width}:${file}`
const fullInventory=viewports.flatMap(viewport=>sources.map(file=>caseKey(viewport,file)))
const startIndex=startAt===undefined?0:fullInventory.indexOf(startAt)
assert.ok(startIndex>=0,'Unknown start-at case; use WIDTH:SOURCE.r from the selected inventory')
const plannedCaseKeys=fullInventory.slice(startIndex)
const plannedCaseSet=new Set(plannedCaseKeys)
await mkdir(output,{recursive:true})
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const wasm=await readFile('public/wasm/geometry-kernel.wasm')
const wasmSha=sha(wasm)
assert.equal(sha(await readFile('src/generated/geometry-kernels/kernel_bg.wasm')),wasmSha,'Public and source WASM must match')
assert.equal(sha(await readFile(path.join(dist,'wasm/geometry-kernel.wasm'))),wasmSha,'Rebuild dist after packaging the latest WASM')
assert.ok((await readFile('src/generated/geometry-kernels/identity.ts','utf8')).includes(wasmSha),'Generated identity must match public WASM')
const workerFiles=(await readdir(path.join(dist,'assets'))).filter(name=>/^geometry\.worker-.*\.js$/.test(name))
assert.ok(workerFiles.length,'Built geometry worker asset must exist')
for(const file of workerFiles)assert.ok((await readFile(path.join(dist,'assets',file),'utf8')).includes(wasmSha),'Geometry worker must embed the current WASM fingerprint')
const report={schema:'sweep-miter-ui-matrix/1' ,recordedAt:new Date().toISOString(),
 scope:(bodyBoundaryMode?'Finite constructor-owned progressive body boundary UI scenarios; ':surfaceMode?'Finite progressive surface UI scenarios; ':'Finite retained-miter UI scenarios; ')+' held dispatch tests lifecycle cancellation, not mid-kernel interruption latency. Headless CPU fallback is not GPU qualification.',
 nativeCancelProbe:nativeCancel,nativeRunningCancelProbe:nativeRunningCancel,
 geometryWasmSha256:wasmSha,
 artifactProvenance:{sha256:wasmSha,byteLength:wasm.length,sourcePublicDistVerified:true,geometryWorkerFiles:workerFiles},
 appSourceSha256:sha(await readFile('src/App.vue')),distIndexSha256:sha(await readFile(path.join(dist,'index.html'))),cases:[]}
if(nativeRunningCancel)report.scope+=' Running probes require authenticated native CPU samples at the profile tail without Debugger suspension, then actual Worker destruction. Measured UI cancellation includes scheduling and is not cooperative kernel polling latency.'
else if(nativeCancel)report.scope+=' Native probes pause actual geometry abi_request activations and verify Worker destruction; they do not measure unconstrained cancellation latency.'
report.evidenceOnly=evidenceOnly
report.gpuBackendRequested=gpuBackend
report.selection={startAt:startAt??null,fullInventory,plannedCaseKeys}
if(evidenceOnly)report.scope+=' Evidence-only run excludes viewport rendering qualification.'
report.sourceHashes = Object.fromEntries(await Promise.all([
 'src/App.vue', 'src/services/sweepRetainedCorrespondence.ts', 'src/services/geometry/brep.ts',
 'src/services/nurbsMiterOwnedBoundary.ts','crates/brep-core/src/sweep_miter_owned.rs','crates/geometry-bridge/src/cad_miter_owned.rs',
 'crates/nurbs-core/src/analysis/curve_measure.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep/endpoint_jets.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep.rs','crates/nurbs-core/src/sweeps/progressive_sweep/authored_error.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep/authored_error/spatial_parameter.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep/rmf_transport.rs',
 'src/services/nurbsSectionProjection.ts','src/services/nurbsCircleSectionRepair.ts',
 'src/services/miterBodyLayout.ts','src/services/sweepBoundaryCertificate.ts',
 'src/features/DirectModeler.vue', 'src/generated/geometry-kernels/bytes.ts',
 'scripts/check-sweep-miter-matrix-browser.mjs','scripts/sweep-native-cancel-probe.mjs','scripts/sweep-body-matrix-sources.mjs',
 'crates/geometry-bridge/src/brep.rs',
 'crates/brep-core/src/face_contact_groups.rs','crates/brep-core/src/face_contacts.rs',
 'crates/brep-core/src/boundary_embedding.rs','crates/brep-core/src/sweep_cap_contacts.rs',
 'crates/brep-core/src/shell_nesting.rs','crates/geometry-bridge/src/cad_face_contacts.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep/authored_error/corrected_regular.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep/authored_error/arc_pose.rs',
 'crates/nurbs-core/src/sweeps/progressive_sweep/profile_domain.rs',
 'crates/nurbs-core/src/sweeps/progressive_miter/authored_frame_certificate/frenet_path_values.rs',
].map(async file => [file, sha(await readFile(file))])))
const save=()=>writeFile(path.join(output,'matrix.json'),JSON.stringify(report,null,2)+'\n')
const server=createServer(async(req,res)=>{
 try {
  const url=new URL(req.url,'http://localhost')
  const file=path.resolve(dist,'.'+(url.pathname==='/'?'/index.html':decodeURIComponent(url.pathname)))
  if(!file.startsWith(dist+path.sep)){res.writeHead(403).end();return}
  res.setHeader('Content-Type',file.endsWith('.html')?'text/html':file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
const {playwright}=await loadQualificationPlaywrightPackage()
const gpuArgs=gpuBackend==='metal'?['--use-angle=metal','--enable-features=WebGPU']
 :gpuBackend==='swiftshader'?['--use-webgpu-adapter=swiftshader']:[]
const browser=await playwright.chromium.launch({headless:!headed,args:['--enable-unsafe-webgpu',...gpuArgs]})
report.browser=browser.version()
report.headed=headed
try {
 for(const viewport of viewports)for(const file of sources){
  if(!plannedCaseSet.has(caseKey(viewport,file)))continue
  const source=await readFile(path.join(sourceDirectory,file),'utf8')
  const name=viewport.width+'-'+file.replace(/\.r$/,'')
  const directory=path.join(output,name);await mkdir(directory,{recursive:true})
  const context=await browser.newContext({viewport,acceptDownloads:true})
  const page=await context.newPage(),pageErrors=[]
  page.setDefaultTimeout(30000)
  page.on('pageerror',error=>pageErrors.push(error.message))
  const entry={viewport,file,sourceSha256:sha(source),assertions:[],pageErrors,status:'running'}
  report.cases.push(entry);await save()
  try {
   await page.addInitScript(()=>{
    const NativeWorker=window.Worker
    window.sweepHeld=[];window.sweepTerminated=[];window.sweepTerminateCalls=[];window.sweepHoldBuild=false;window.sweepHoldExact=false
    window.Worker=class extends NativeWorker {
     postMessage(message,...args){
      const mode=message?.type==='build'?'build':message?.kind==='exact-solid'?'exact':null
      if(mode)this.sweepNativeMode=mode
      if(mode&&(mode==='build'?window.sweepHoldBuild:window.sweepHoldExact)){
       this.sweepMode=mode;window.sweepHeld.push(mode);return
      }
      return super.postMessage(message,...args)
     }
     terminate(){window.sweepTerminateCalls.push({mode:this.sweepNativeMode,atEpochMs:Date.now()});if(this.sweepMode)window.sweepTerminated.push(this.sweepMode);return super.terminate()}
    }
   })
   await page.goto(`http://127.0.0.1:${server.address().port}`)
   entry.gpuAdapter=await page.evaluate(async()=>{
    const adapter=await navigator.gpu?.requestAdapter()
    return adapter?{vendor:adapter.info.vendor,architecture:adapter.info.architecture,device:adapter.info.device,description:adapter.info.description}:null
   })
   if(gpuBackend!=='auto')assert.ok(entry.gpuAdapter,'Requested backend must provide an actual WebGPU adapter')
   const sourceButton=page.getByRole('button',{name:'Исходный код',exact:true})
   const openSource=async()=>{if(await sourceButton.getAttribute('aria-pressed')!=='true')await sourceButton.click()}
   await openSource()
   await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()
   const editor=page.locator('textarea.code-input')
   const load=async(text)=>page.locator('input[type="file"][accept*=".scad"][accept*=".r"]').setInputFiles({name:'sweep-matrix.r',mimeType:'text/plain',buffer:Buffer.from(text)})
   const build=async()=>{
    await page.getByRole('button',{name:'Собрать',exact:true}).click()
    await page.waitForFunction(()=>{const b=[...document.querySelectorAll('.editor-toolbar button')].find(b=>b.textContent.trim()==='Собрать');return b&&!b.disabled},{},{timeout:120000})
   }
   const assertBuilt=async()=>{
    assert.equal(await page.locator('.message.error').count(),0,`${file}: ${(await page.locator('.message.error').allTextContents()).join(' ')}`)
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
   if(gpuBackend!=='auto'&&!surfaceMode){
    await page.locator('.no-gpu').waitFor({state:'hidden',timeout:30000})
    await page.locator('canvas.gpu-canvas').waitFor({state:'visible'})
    entry.assertions.push({case:'viewport-renderer-ready',status:'passed'})
   }
   if(bodyBoundaryMode){
    const evidence=page.getByTestId('sweep-body-final-evidence')
    await evidence.waitFor({state:'visible'})
    assert.match(await evidence.innerText(),/Непрерывная ошибка границы sweep-тела:\s*доказана/)
    const presented=(await evidence.innerText()).match(/оценка\s*≈\s*([0-9.,eE+-]+)\s*mm/)
    assert.ok(presented,'Body boundary upper estimate must be visible')
    const presentedUpper=Number(presented[1].replace(',','.'))
    const bodyBudget=Number(source.match(/max_deviation:\s*([0-9.]+)mm/)[1])
    assert.ok(presentedUpper>0&&presentedUpper<=bodyBudget,'A positive upper estimate must fit the original source budget')
    entry.assertions.push({case:'nonzero-small-bound-presentation',status:'passed'})
    entry.assertions.push({case:'constructor-owned-complete-body-boundary-presentation',status:'passed'})
    const strict=source.replace(/max_deviation:\s*[0-9.]+mm/,'max_deviation: 0.000000000000000000000000000001mm')
    assert.notEqual(strict,source)
    await load(strict);await build()
    assert.match(await page.locator('.message.error').innerText(),/continuous retained-patch error|refinement.*budget/i)
    assert.equal(await evidence.count(),0,'Refused source must clear body-boundary proof')
    await load(source);await build();await assertBuilt();await evidence.waitFor({state:'visible'})
    entry.assertions.push({case:'body-boundary-strict-refusal-and-restoration',status:'passed'})
   }
   if(surfaceMode){
    if(evidenceOnly){
     entry.viewportRenderingQualified=false
     entry.webGpuUnavailable=await page.locator('.no-gpu').isVisible()
    }else{
     await page.locator('.no-gpu').waitFor({state:'hidden',timeout:30000})
     await page.locator('canvas.gpu-canvas').waitFor({state:'visible'})
     entry.assertions.push({case:'viewport-renderer-ready',status:'passed'})
    }
    const patchEvidence=page.getByTestId('sweep-patch-final-evidence')
    await patchEvidence.waitFor({state:'visible'})
    assert.match(await patchEvidence.innerText(),/Область: сохранённые поверхности/)
    assert.match(await patchEvidence.innerText(),/Регулярность поверхностей:\s*(доказана|не доказана)/)
    entry.assertions.push({case:'separate-retained-surface-regularity-presentation',status:'passed'})
    if(file==='decomposed-profile-g1-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*не доказана/)
     assert.match(await patchEvidence.innerText(),/G1 между частями одного профиля:\s*доказана/)
     const corner=source.replace('[1.5mm,0.25mm,0mm]','[1.5mm,0.375mm,0mm]')
     assert.notEqual(corner,source)
     await load(corner);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G1 между частями одного профиля:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G1 между частями одного профиля:\s*доказана/)
     entry.assertions.push({case:'decomposition-G1-curvature-jump-corner-refusal-and-restoration',status:'passed'})
    }
    if(file==='decomposed-profile-g2-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*доказана\s*\(31\)/)
     const corner=source.replace('[0.5mm,0mm,0mm]','[0.5mm,0.25mm,0mm]')
     assert.notEqual(corner,source)
     await load(corner);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*доказана\s*\(31\)/)
     entry.assertions.push({case:'decomposition-G2-scoped-corner-refusal-and-restoration',status:'passed'})
    }
    if(file==='authored-closed-cartesian-c2-progressive-sweep.r'||file==='authored-closed-nonclamped-c2-progressive-sweep.r'||file==='authored-closed-piecewise-c2-progressive-sweep.r'||file==='authored-closed-c2-progressive-sweep.r'||file==='authored-closed-multispan-c2-progressive-sweep.r'||file==='authored-closed-projective-c2-progressive-sweep.r'||file==='closed-authored-source-bound-progressive-sweep.r'||file==='closed-authored-arc-length-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'closed-authored-original-frame-C2-separate-presentation',status:'passed'})
    }
    if(file==='closed-conic-direction-frame-c1-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C1 замкнутого исходного кадра:\s*доказана/)
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     const bad=source.replace('[1mm,1mm,0]','[1.0000000000000002mm,1mm,0]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C1 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C1 замкнутого исходного кадра:\s*доказана/)
     assert.match(await patchEvidence.innerText(),/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*доказана/)
     entry.assertions.push({case:'closed-direction-C1-single-bit-refusal-and-restoration-without-C2-promotion',status:'passed'})
    }
    if(file==='closed-planar-path-frame-c2-progressive-sweep.r'||file==='closed-planar-guided-frame-c2-progressive-sweep.r'||file==='closed-arc-length-guided-frame-c2-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     const bad=file.includes('guided-frame-c2')?source.replace('[2mm,1mm,1mm]','[2mm,1mm,1.0000000000000002mm]'):source.replace('[3mm,3mm,0]','[3mm,3.0000000000000004mm,0]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'closed-path-source-C2-single-bit-refusal-and-restoration',status:'passed'})
     if(file==='closed-planar-guided-frame-c2-progressive-sweep.r')assert.match(await patchEvidence.innerText(),/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*не доказана/)
    }
    if(file==='authored-closed-cartesian-c2-progressive-sweep.r'){
     const bad=source.replace('[-0.28125,0.125,1]','[-0.28124999999999994,0.125,1]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'cartesian-closed-frame-C2-single-bit-refusal-and-restoration',status:'passed'})
    }
    if(file==='authored-closed-piecewise-c2-progressive-sweep.r'){
     const bad=source.replace('[-0.109375,0.0625,1]','[-0.10937499999999999,0.0625,1]')
     assert.notEqual(bad,source,'The original interior second jet must change by one binary64 step')
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'piecewise-original-closed-C2-single-bit-refusal-and-restoration',status:'passed'})
    }
    if(file==='mixed-knots-curved-frenet-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     const bad=source.replace('[0.5mm,0.25mm,0]','[0.5mm,0.25000000000000006mm,0]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     entry.assertions.push({case:'mixed-original-path-C2-frame-single-bit-refusal-and-restoration',status:'passed'})
    }
    if(['nonaxial-planar-rmf-progressive-sweep.r','arc-length-guided-progressive-sweep.r','arc-length-curved-guided-progressive-sweep.r'].includes(file)){
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     entry.assertions.push({case:'original-frame-C2-separate-presentation',status:'passed'})
    }
    if(file==='corrected-frenet-sweep.r'||file==='corrected-frenet-antiparallel-progressive-sweep.r'){
     const text=await patchEvidence.innerText()
     assert.match(text,/C2 исходного кадра:\s*доказана/)
     assert.match(text,/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*доказана/)
     const bad=file==='corrected-frenet-antiparallel-progressive-sweep.r'
      ?source.replace('[1mm,-1mm,0]','[1.0000000000000002mm,-1mm,0]')
      :source.replace('[1mm,1mm,0]','[1mm,1mm,0.0000000000000001mm]')
     assert.notEqual(bad,source)
     await load(bad);await build()
     if(file==='corrected-frenet-sweep.r'){
      const refusal=page.locator('.message.error');await refusal.waitFor()
      entry.correctedSpatialRefusal=await refusal.innerText()
      assert.match(entry.correctedSpatialRefusal,/Progressive sweep .* exceeds /)
      assert.equal(await patchEvidence.count(),0,'Spatial refinement refusal must clear the old planar C2 proof')
     }else{
      await assertBuilt()
      assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*не доказана/)
     }
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     assert.match(await patchEvidence.innerText(),/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*доказана/)
     entry.assertions.push({case:file==='corrected-frenet-sweep.r'?'corrected-spatial-perturbation-budget-refusal-and-planar-source-C2-restoration':'corrected-planar-source-C2-and-error-off-plane-refusal-and-restoration',status:'passed'})
    }else if(file.includes('frenet')||file.includes('fixed-normal')){
     assert.match(await patchEvidence.innerText(),/Регулярность исходного кадра:\s*доказана/)
     entry.assertions.push({case:'whole-original-frame-regularity-presentation',status:'passed'})
    }
    if(file!=='closed-planar-guided-frame-c2-progressive-sweep.r'&&(file.startsWith('arc-length-')||file==='oblique-rmf-progressive-sweep.r'||file==='progressive-sweep.r'||file==='affine-progressive-sweep.r'||file.startsWith('closed-')||file==='fixed-progressive-sweep.r'||file==='fixed-normal-progressive-sweep.r'||file==='frenet-progressive-sweep.r'||file==='spatial-frenet-progressive-sweep.r'||file==='spatial-frenet-affine-progressive-sweep.r')){
     const sourceBudget=source.match(/max_deviation:\s*([0-9.]+)mm/)
     assert.ok(sourceBudget,'Surface fixture must declare its error budget')
     const targetBudget=Number(sourceBudget[1])
     assert.ok(Number.isFinite(targetBudget)&&targetBudget>0,'Source error budget must be positive')
     const text=await patchEvidence.innerText()
     assert.match(text,/профиля:\s*доказана/)
     const bound=text.match(/≤([0-9.,eE+-]+)\s*\/\s*([0-9.,eE+-]+)\s*mm/)
     assert.ok(bound,'Surface must show the final retained error and budget')
     assert.equal(Number(bound[2].replace(',','.')),targetBudget,'Refinement must use the source error budget')
     assert.ok(Number(bound[1].replace(',','.'))>=0&&Number(bound[1].replace(',','.'))<=targetBudget,'Final retained error must fit the source budget')
     entry.assertions.push({case:file==='closed-spatial-rmf-affine-certified.r'?'closed-spatial-rmf-configured-0.25mm-evidence':file==='oblique-rmf-progressive-sweep.r'||file==='progressive-sweep.r'||file==='affine-progressive-sweep.r'?'rmf-original-source-error-evidence':file==='fixed-progressive-sweep.r'?'fixed-refined-error-evidence':file==='closed-fixed-normal-full-turn-progressive-sweep.r'?'closed-full-turn-0.01mm-evidence':'closed-refined-two-mm-evidence',status:'passed'})
     // At 257 stations this full-turn fixture has sampled refinement below
     // 0.0006 mm, but its continuous bound is above it. Exercise the actual
     // continuous admission gate rather than an earlier sampled refusal.
     const strictBudget=file==='closed-planar-rmf-full-turn-progressive-sweep.r'?'0.0006':'0.000000000000000000000000000001'
     let strict=source.replace(/max_deviation:\s*[0-9.]+mm/,`max_deviation: ${strictBudget}mm`)
     if(file.startsWith('arc-length-'))strict=strict.replace(/max_sections:\s*(33|17|9)/,'max_sections: 3')
     assert.notEqual(strict,source,'Strict closed fixture must change the error budget')
     await load(strict);await build()
     const refusal=page.locator('.message.error');await refusal.waitFor()
     assert.match(await refusal.innerText(),/continuous retained-patch error/i)
     assert.equal(await page.getByTestId('sweep-patch-final-evidence').count(),0,'Refused source must clear final patch proof')
     await load(source);await build();await assertBuilt()
     await patchEvidence.waitFor({state:'visible'})
     assert.match(await patchEvidence.innerText(),/профиля:\s*доказана/)
     entry.assertions.push({case:file==='oblique-rmf-progressive-sweep.r'||file==='progressive-sweep.r'||file==='affine-progressive-sweep.r'?'rmf-strict-bound-refusal-and-restoration':file==='fixed-progressive-sweep.r'?'fixed-strict-bound-refusal-and-restoration':'closed-strict-bound-refusal-and-restoration',status:'passed'})
    }
    entry.assertions.push({case:'scoped-retained-patch-evidence',status:'passed'})
   }
   entry.viewportEvidence=await page.getByTestId('sweep-final-evidence').allTextContents()
   if(!surfaceMode&&!bodyBoundaryMode){
   assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*[✓?]\s*\/\s*[✓?]\s*\(\d+\)/.test(text)&&/Стыки вдоль пути:\s*(C0|G1|G2)/.test(text)), 'Profile smoothness must remain scoped and retain path C0')
   entry.assertions.push({case:'scoped-profile-smoothness-presentation',status:'passed'})
   if(file==='sharp-profile-nonuniform-station-g2-miter.r'){
    assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*\?\s*\/\s*\?/.test(text)&&/Стыки вдоль пути:\s*G2/.test(text)), 'Sharp profile refusal must preserve independent nonuniform station G2 in the viewport')
    entry.assertions.push({case:'sharp-profile-refusal-preserves-station-G2',status:'passed'})
   }
   if(file==='closed-rational-frame-guide-affine-hollow-placed.r'){
    assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*✓\s*\//.test(text)&&/Стыки вдоль пути:\s*C0/.test(text)), 'Closed rational profile G1 and sharp path C0 must reach viewport')
    entry.assertions.push({case:'closed-rational-profile-G1-sharp-path-C0',status:'passed'})
   }
   if(file==='progressive-miter-reconstructed-rational-profile.r'){
    assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*✓\s*\//.test(text)&&/Стыки вдоль пути:\s*G2/.test(text)), 'Rational profile G1 and station G2 must reach viewport presentation')
    entry.assertions.push({case:'rational-profile-G1-and-station-G2',status:'passed'})
   }
   if(file.startsWith('progressive-miter-reconstructed-conic-')||file==='closed-miter-frame-guide-affine-hollow.r'||file==='closed-miter-frame-guide-affine-hollow-corrected.r'||file==='closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r'||file==='progressive-miter-circle-corrected-hollow.r'||file==='progressive-miter-spatial-circle-corrected-hollow.r'||file==='progressive-miter-affine-circle-corrected-hollow.r'||file==='progressive-miter-oblique-circle-corrected-hollow.r'||file==='miter-rational-curved-guide-frame-affine-hollow.r'){
    assert.ok(entry.viewportEvidence.some(text=>/G1 \/ G2 стыков профиля:\s*✓\s*\/\s*✓/.test(text)), 'Every profile seam in this mode must certify G2')
    entry.assertions.push({case:'profile-G2',status:'passed'})
   }
   if(file.startsWith('progressive-miter-reconstructed-conic-')||file==='progressive-miter-station-g2-hollow.r'||file==='nonuniform-station-g2-hollow-miter.r'||file==='progressive-miter-reconstructed-stations.r'||file==='progressive-miter-reconstructed-guide-frame-affine-hollow.r'||file==='progressive-miter-reconstructed-moving-frame-guide-affine-hollow.r'){
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
     ['complete-bound-refusal',source.replace('max_deviation: 2mm','max_deviation: 0.1mm'),/complete boundary error/],
    ]){
     assert.notEqual(bad,source,'Refusal fixture must change its source')
     await load(bad);await build();assert.match(await page.locator('.message.error').innerText(),reason)
     assert.equal(await page.getByTestId('sweep-final-evidence').count(),0,'Refusal must clear final certificate presentation')
     entry.assertions.push({case:label,status:'passed'})
     await load(source);await build();await assertBuilt()
    }
   }
   }
   if(surfaceMode||bodyBoundaryMode){
    const closeDock=page.getByRole('button',{name:'Закрыть боковую панель',exact:true})
    if(await closeDock.isVisible())await closeDock.click()
    const materialMenu=page.locator('.viewport-material-menu')
    assert.equal(await materialMenu.evaluate(e=>e.open),false,'Material controls must start collapsed')
    await materialMenu.locator('summary').click()
    await materialMenu.getByRole('combobox',{name:'Модель затенения',exact:true}).waitFor({state:'visible'})
    const materialBounds=await materialMenu.locator('.material-controls').boundingBox()
    assert.ok(materialBounds&&materialBounds.x>=0&&materialBounds.x+materialBounds.width<=viewport.width,'Opened material controls must fit the viewport')
    await materialMenu.locator('summary').click()
    await materialMenu.getByRole('combobox',{name:'Модель затенения',exact:true}).waitFor({state:'hidden'})
    entry.assertions.push({case:'material-panel-open-close',status:'passed'})
    await page.locator('.canvas-panel').getByRole('button',{name:'Вписать',exact:true}).click()
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))
    entry.assertions.push({case:'viewport-fit-through-UI',status:'passed'})
   }
   const persistenceError=page.locator('.persistence-error')
   entry.previewWorkspacePersistence={error:await persistenceError.count()?await persistenceError.getAttribute('title'):null}
   await save()
   await page.screenshot({path:path.join(directory,'preview.png'),fullPage:true})
   if(nativeCancel){
    assert.ok(!surfaceMode,'Native cancellation probe requires a retained body source')
    for(const buildAction of ['cancel','source-change']){
    const probe=await createNativeCancelProbe(browser,wasm)
    try {
     await probe.attachExisting()
     // Preserve the fixture's valid station and body face budgets. Increasing
     // initial sections can exceed the body ceiling even below the arc-length
     // ceiling, turning the probe into a request that would never build.
     // These authored fixtures finish tiny-budget requests too quickly.
     // Their independently qualified 65-station requests retain the valid
     // error budget and produce certified 256/258-face Solids.
     const busySource=bodyBoundaryMode
      ?file==='periodic-corrected-frenet-nonplanar-body-boundary.r'
       // The proved periodic grid fits the original aggregate work ceiling.
       ?source.replace(/initial_sections:\s*5\b/, 'initial_sections: 33')
       :correctedRegularBodySources.includes(file)&&file.includes('hollow')
       // Eight walls per interval plus two caps: 129 sections need 1026
       // faces and are refused by the original 1024-face constructor limit.
       ?source.replace(/initial_sections:\s*3\b/, 'initial_sections: 65')
       :correctedRegularBodySources.includes(file)||closedCorrectedRegularBodySources.includes(file)
       ?source.replace(/initial_sections:\s*(3|5)\b/, 'initial_sections: 129')
       :file==='closed-contact-hollow-body-boundary.r'||file==='closed-arc-length-contact-hollow-body-boundary.r'||file==='closed-arc-length-rational-phase-contact-hollow-body-boundary.r'
       ?source.replace('initial_sections: 17','initial_sections: 65')
       :file==='oblique-rmf-circle-hollow-corrected-body-boundary.r'
       ?source.replace('initial_sections: 2,max_sections: 2','initial_sections: 65,max_sections: 65')
       :file==='curved-rmf-circle-hollow-corrected-body-boundary.r'
       ?source.replace('initial_sections: 3,max_sections: 129','initial_sections: 129,max_sections: 129')
       :file.startsWith('arc-length-curved-')&&file.endsWith('-body-boundary.r')
       // Use the source's existing section ceiling to keep the actual native
       // request observable; tighten inverse-length tolerance within the existing work budget.
       ?source.replace('initial_sections: 3,max_sections: 17','initial_sections: 17,max_sections: 17').replace('length_tolerance: 0.001mm','length_tolerance: 0.000000001mm')
       :file==='authored-multispan-body-boundary.r'
       ?source.replace('initial_sections: 5,max_sections: 17','initial_sections: 65,max_sections: 65')
       :file==='closed-authored-tube-body-boundary.r'
       ?source.replace('initial_sections: 17','initial_sections: 65')
       :file==='corrected-frenet-straight-affine-hollow-body-boundary.r'
       ?source.replace('initial_sections: 3,max_sections: 3','initial_sections: 33,max_sections: 33')
       :file==='progressive-hollow-boundary.r'||file==='progressive-unsegmented-hollow-boundary.r'
       ?source.replace('initial_sections: 3,max_sections: 3','initial_sections: 65,max_sections: 65')
       :source.replace(/max_deviation:\s*[0-9.]+mm/,'max_deviation: 0.0000001mm')
      :source+'\n// Native cancellation observes the original miter geometry in a new revision.\n'
     assert.notEqual(busySource,source)
     await editor.fill(busySource)
     if(nativeRunningCancel){
      await probe.armRunningGeometry()
      await page.evaluate(()=>{const button=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Собрать');if(!button||button.disabled)throw Error('Native build action is unavailable');button.click()})
     }else await page.getByRole('button',{name:'Собрать',exact:true}).click()
     const nativeStack=await (nativeRunningCancel?probe.observeRunningGeometry():probe.pauseGeometryRequest())
     const cancelStartedAt=Date.now()
     const observationToCancelInitiationMs=nativeRunningCancel?cancelStartedAt-nativeStack.observedAtEpochMs:null
     if(buildAction==='source-change')await editor.fill(busySource+'\n// Supersede an active Rust build request.\n')
     else if(nativeRunningCancel)await page.evaluate(()=>{const button=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Отменить сборку');if(!button)throw Error('Native build already completed before cancellation');button.click()})
     else await page.getByRole('button',{name:'Отменить сборку',exact:true}).click()
     await probe.assertDestroyed()
     const workerDestroyedAfterMs=Date.now()-cancelStartedAt
     const terminateCall=await page.evaluate(start=>window.sweepTerminateCalls.find(call=>call.mode==='build'&&call.atEpochMs>=start),cancelStartedAt)
     assert.ok(terminateCall,'Cancellation must call Worker.terminate on the active build')
     const terminateCalledAfterMs=terminateCall.atEpochMs-cancelStartedAt
     await page.waitForFunction(()=>!document.body.innerText.includes('Отменить сборку'))
     assert.equal(await page.getByTestId('sweep-body-final-evidence').count(),0)
     assert.equal(await page.getByTestId('sweep-final-evidence').count(),0)
     entry.assertions.push({case:buildAction==='cancel'?'native-build-cancellation':'native-build-source-change',status:'passed',sourceSha256:sha(busySource),nativeStack,observationToCancelInitiationMs,terminateCalledAfterMs,workerDestroyedAfterMs,
      scope:nativeRunningCancel?'Authenticated running native CPU tail; unpaused Worker termination, UI scheduling included':'Actual geometry abi_request paused by Debugger; Worker destruction, not unconstrained latency'})
    }finally{await probe.close()}
    await load(source);await build();await assertBuilt()
    entry.assertions.push({case:buildAction==='cancel'?'native-build-cancel-restoration':'native-build-source-change-restoration',status:'passed'})
    }
    for(const action of ['cancel','source-change']){
     const exactSource=bodyBoundaryMode&&(file.includes('rational-straight')||file==='corrected-frenet-straight-affine-hollow-body-boundary.r')
      ?source.replace(/initial_sections: 3,max_sections: (3|129)/,file==='corrected-frenet-straight-affine-hollow-body-boundary.r'?'initial_sections: 33,max_sections: 33':'initial_sections: 65,max_sections: 65')
      :source
     if(exactSource!==source){await load(exactSource);await build();await assertBuilt()}
     const exactProbe=await createNativeCancelProbe(browser,wasm)
     try {
      const beforeTargets=await exactProbe.targetIds()
      await toSolid()
      await exactProbe.attachNew(beforeTargets)
      const nativeStack=await (nativeRunningCancel?exactProbe.observeRunningGeometry():exactProbe.pauseGeometryRequest())
      const cancelStartedAt=Date.now()
      const observationToCancelInitiationMs=nativeRunningCancel?cancelStartedAt-nativeStack.observedAtEpochMs:null
      if(action==='cancel')await page.getByRole('button',{name:'Отмена',exact:true}).click()
      else await editor.fill(source+'\n// Supersede an active Rust exact-solid request.\n')
      await exactProbe.assertDestroyed()
      const workerDestroyedAfterMs=Date.now()-cancelStartedAt
      const terminateCall=await page.evaluate(start=>window.sweepTerminateCalls.find(call=>call.mode==='exact'&&call.atEpochMs>=start),cancelStartedAt)
      if(!terminateCall){
       entry.nativeSolidCancellationFailure={action,nativeStack,cancelStartedAt,workerDestroyedAfterMs,observationToCancelInitiationMs,
        terminationCalls:await page.evaluate(()=>window.sweepTerminateCalls)}
       await save()
      }
      assert.ok(terminateCall,'Cancellation must call Worker.terminate on the active Solid')
      const terminateCalledAfterMs=terminateCall.atEpochMs-cancelStartedAt
      await waitSolidFinished()
      assert.equal((await download('native-exact-'+action)).bodies.length,0)
      entry.assertions.push({case:'native-solid-'+action,status:'passed',sourceSha256:sha(exactSource),nativeStack,observationToCancelInitiationMs,terminateCalledAfterMs,workerDestroyedAfterMs,
       scope:nativeRunningCancel?'Authenticated running native CPU tail; unpaused termination prevents stale Solid':'Actual geometry abi_request paused by Debugger; terminated Worker cannot publish stale Solid'})
     }finally{await exactProbe.close()}
     await load(source);await build();await assertBuilt()
     entry.assertions.push({case:'native-solid-'+action+'-restoration',status:'passed'})
    }
   }
   await editor.fill(source+'\n// New source revision whose build will be cancelled.\n')
   await page.evaluate(()=>window.sweepHoldBuild=true)
   await page.getByRole('button',{name:'Собрать',exact:true}).click()
   await page.waitForFunction(()=>window.sweepHeld.includes('build'))
   await page.getByRole('button',{name:'Отменить сборку',exact:true}).click()
   await page.waitForFunction(()=>!document.body.innerText.includes('Отменить сборку'))
   assert.equal(await page.getByTestId('sweep-final-evidence').count(),0,'Cancelled build must not retain final certificate presentation')
   if(surfaceMode)assert.equal(await page.getByTestId('sweep-patch-final-evidence').count(),0,'Cancelled source must clear patch certificate presentation')
   if(bodyBoundaryMode)assert.equal(await page.getByTestId('sweep-body-final-evidence').count(),0,'Cancelled source must clear body certificate presentation')
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
   if(surfaceMode||file==='miter-hollow-body.r'){
    await waitSolidFinished()
    const error=page.locator('.message.error');await error.waitFor()
    assert.match(await error.innerText(),surfaceMode?/Solid source requires native B-rep bodies/i:/Solid geometry.*boundary embedding/i)
    assert.equal((await download('unproved-solid-refused')).bodies.length,0)
    entry.assertions.push({case:surfaceMode?'surface-solid-refusal':bodyBoundaryMode?'bounded-closed-body-solid-refusal':'unproved-spatial-solid-refusal',status:'passed'})
    await solid.getByRole('status',{name:'Сохранено в браузере',exact:true,includeHidden:true}).waitFor({state:'attached'})
    await page.reload();await solid.waitFor()
    await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden',timeout:120000})
    assert.equal((await download('refused-restored')).bodies.length,0)
    entry.assertions.push({case:'refused-empty-project-restoration',status:'passed'})
    await openSource();await page.getByRole('checkbox',{name:'Авто',exact:true}).uncheck()

    const invalid=surfaceMode
     ?source.replace(/^(path\s*=\s*bezier_curve\(points:\s*)\[\[([^\]]+)\],\[[^\]]+\]/m,(_,prefix,first)=>`${prefix}[[${first}],[${first}]`)
       .replace(/^(path\s*=\s*line_curve\(start:\s*)(\[[^\]]+\])(\s*,end:\s*)\[[^\]]+\]/m,(_,prefix,first,separator)=>`${prefix}${first}${separator}${first}`)
       .replace(/^(path\s*=\s*circle_curve\([^\n]*radius:\s*)[^)]+/m,(_,prefix)=>`${prefix}0mm`)
       .replace(/^path\s*=\s*nurbs_curve\([\s\S]*?\)\s*\n/m,'path = bezier_curve(points: [[0,0,0],[0,0,0]])\n')
     :bodyBoundaryMode
      ?source.replace(/^(path\s*=\s*circle_curve\([^\n]*radius:\s*)[^)]+/m,(_,prefix)=>`${prefix}0mm`)
      :source.replace(/\bpoints: \[\[([^\]]+)\],\[[^\]]+\]/,(_,first)=>`points: [[${first}],[${first}]`)
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
   const solidGpu=solid.locator('canvas.gpu-layer').last()
   if(await solidGpu.isVisible()){
    const canvasBounds=await solidGpu.boundingBox()
    assert.ok(canvasBounds&&canvasBounds.width>0&&canvasBounds.height>0)
    entry.solid.renderer='webgpu'
   }else{
    assert.ok(await solid.locator('svg[aria-label="Холст тел 3D"] polygon').count()>0)
    entry.solid.renderer='cpu-svg'
   }
   if(gpuBackend!=='auto')assert.equal(entry.solid.renderer,'webgpu','Requested GPU qualification must render the actual Solid with WebGPU')
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
   const invalid=file.includes('rational-straight')
    ?source.replace(/^path\s*=\s*nurbs_curve\(.*$/m,'path = bezier_curve(points: [[0,0,0],[0,0,0],[0,0,0]])')
    :correctedRegularBodySources.includes(file)||file==='corrected-frenet-affine-zero-curvature-hollow-body-boundary.r'||file==='arc-length-corrected-frenet-affine-zero-curvature-hollow-body-boundary.r'||file==='corrected-frenet-affine-inflection-hollow-body-boundary.r'||file==='arc-length-corrected-frenet-affine-inflection-hollow-body-boundary.r'||file==='corrected-frenet-inflection-hollow-body-boundary.r'||file==='arc-length-corrected-frenet-inflection-hollow-body-boundary.r'||file==='arc-length-curved-nonaxial-planar-rmf-body-boundary.r'||file==='arc-length-curved-authored-body-boundary.r'||file==='arc-length-curved-fixed-body-boundary.r'||file==='arc-length-curved-fixed-normal-body-boundary.r'||file==='arc-length-curved-frenet-body-boundary.r'||(file==='arc-length-curved-guided-body-boundary.r'||file==='arc-length-curved-contact-body-boundary.r'||file==='arc-length-curved-planar-rmf-body-boundary.r')
    ?source.replace(/^path\s*=\s*bezier_curve\(points:.*$/m,'path = bezier_curve(points: [[0,0,0],[0,0,0],[0,0,0]])')
    :bodyBoundaryMode
    ?source.replace(/^(path\s*=\s*line_curve\(start:\s*)(\[[^\]]+\])(\s*,end:\s*)\[[^\]]+\]/m,(_,prefix,first,separator)=>`${prefix}${first}${separator}${first}`)
      .replace(/^(path\s*=\s*bezier_curve\(points:\s*)\[\[([^\]]+)\],\[[^\]]+\]/m,(_,prefix,first)=>`${prefix}[[${first}],[${first}]`)
      .replace(/^path\s*=\s*circle_curve\(.*$/m,'path = line_curve(start: [0,0,0],end: [0,0,0])')
      .replace(/^path\s*=\s*nurbs_curve\(.*$/m,'path = line_curve(start: [0,0,0],end: [0,0,0])')
    :source.replace(/\bpoints: \[\[([^\]]+)\],\[[^\]]+\]/,(_,first)=>`points: [[${first}],[${first}]`)
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
   const resourceLimited=bodyBoundaryMode
    ?source.replace(/max_deviation:\s*[0-9.]+mm/,'max_deviation: 0.000000000000000000000000000001mm')
    :source.includes('cap_correction_max_work:')
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
   // Preserve the primary failure before fallible diagnostic UI operations.
   await save()
   try{entry.bodyText=(await page.locator('body').innerText()).slice(-10000)}
   catch(diagnostic){entry.bodyTextError=diagnostic.message}
   try{await page.screenshot({path:path.join(directory,'failure.png'),fullPage:true})}
   catch(diagnostic){entry.failureScreenshotError=diagnostic.message}
   throw error
  }finally{await save();await context.close()}
 }
 assert.deepEqual(report.cases.map(c=>caseKey(c.viewport,c.file)),plannedCaseKeys,'Every selected case must run exactly once')
 assert.ok(report.cases.every(c=>c.status==='passed'))
 report.passed=true;await save()
}finally{await browser.close();await new Promise(resolve=>server.close(resolve))}
