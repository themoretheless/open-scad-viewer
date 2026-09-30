import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import ts from 'typescript'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const output=process.argv[2]??'/tmp/solid-material-check'
await mkdir(output,{recursive:true})
const modules=new Map()
for(const name of ['solidGpuView','transparentBsp','transparentTriangleSplit']){
 const source=await readFile(`src/services/${name}.ts`,'utf8')
 modules.set('/'+(name==='solidGpuView'?'renderer.js':name),ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText)
}
const server=createServer((req,res)=>{
 const module=modules.get(req.url)
 res.setHeader('Content-Type',module?'text/javascript':'text/html')
 res.end(module??'<!doctype html><canvas width="256" height="256" style="width:256px;height:256px;background:white"></canvas>')
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:process.env.SOLID_GPU_HEADED!=='1',args:['--enable-unsafe-webgpu']})
 const page=await browser.newPage(),errors=[]
 page.on('pageerror',e=>errors.push(String(e)))
 page.on('console',m=>{if(m.type()==='error')errors.push(m.text())})
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.evaluate(async()=>{
  window.gpuErrors=[]
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const device=adapter.requestDevice.bind(adapter);adapter.requestDevice=async(...args)=>{const result=await device(...args);result.addEventListener('uncapturederror',e=>window.gpuErrors.push(e.error.message));
    const write=result.queue.writeBuffer.bind(result.queue);window.vertexWrites=0;
    result.queue.writeBuffer=(buffer,...args)=>{if(buffer.usage&GPUBufferUsage.VERTEX)window.vertexWrites++;return write(buffer,...args)};
    return result}}return adapter}
  const {SolidGpuLayer}=await import('/renderer.js')
  const layer=new SolidGpuLayer(document.querySelector('canvas'))
  if(!await layer.init())throw Error('WebGPU initialization failed')
  window.layer=layer
  layer.setView({camera:{yaw:0,pitch:Math.PI/2},viewBox:[-1.5,-1.5,3]})
 })
 const hashes=[]
 for(const [name,metallic,roughness] of [['plastic',0,.2],['metal',1,.2],['rough-metal',1,.9]]){
  await page.evaluate(async({metallic,roughness})=>{
   window.layer.setBodies([{id:'triangle',positions:new Float32Array([-1,-1,0,1,-1,0,0,1,0]),normals:new Float32Array([0,0,1,0,0,1,0,0,1]),hues:new Float32Array([-1]),color:[.7,.3,.1],metallic,roughness}])
   await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
   if(!window.layer.ready)throw Error('WebGPU device lost: '+window.gpuErrors.join('; '))
  },{metallic,roughness})
  const image=await page.locator('canvas').screenshot({path:`${output}/${name}.png`})
  hashes.push(createHash('sha256').update(image).digest('hex'))
 }
 assert.equal(new Set(hashes).size,3,'Each material must change rendered pixels')

 async function render(name,bodies,drag){
  await page.evaluate(async({bodies,drag})=>{
   window.layer.setBodies(bodies.map(b=>({id:b.id,positions:new Float32Array(b.positions??[-1,-1,b.z,1,-1,b.z,0,1,b.z]),normals:new Float32Array([0,0,1,0,0,1,0,0,1]),hues:new Float32Array([-1]),color:b.color,opacity:b.opacity})))
   if(drag)window.layer.setDragOffset(drag.ids,drag.delta)
   await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
   if(!window.layer.ready)throw Error('WebGPU device lost: '+window.gpuErrors.join('; '))
  },{bodies,drag})
  const image=await page.locator('canvas').screenshot({path:`${output}/${name}.png`})
  return createHash('sha256').update(image).digest('hex')
 }
 const near={id:'near',z:.5,color:[1,0,0],opacity:.5},far={id:'far',z:0,color:[0,0,1],opacity:.5}
 const ordered=await render('transparent-near-far',[near,far])
 const viewUploads=await page.evaluate(async()=>{
  const before=window.vertexWrites
  window.layer.setView({camera:{yaw:0,pitch:Math.PI/2},viewBox:[-1.4,-1.5,3]})
  window.layer.resize(256,256,1)
  await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
  const afterPan=window.vertexWrites
  window.layer.setView({camera:{yaw:.1,pitch:Math.PI/2},viewBox:[-1.5,-1.5,3]})
  await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
  const afterRotate=window.vertexWrites
  window.layer.setView({camera:{yaw:0,pitch:Math.PI/2},viewBox:[-1.5,-1.5,3]})
  await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
  return {pan:afterPan-before,rotate:afterRotate-afterPan}
 })
 assert.equal(viewUploads.pan,0,'pan/resize must reuse transparent vertex ordering')
 assert.equal(viewUploads.rotate,1,'camera rotation must upload the new ordering')
 assert.equal(await render('transparent-reversed-input',[far,near]),ordered,'input order must not change separated transparent layers')
 assert.notEqual(await render('opaque-near',[{...near,opacity:1}]),ordered)
 const opaque=await render('opaque-far',[{...far,opacity:1}])
 assert.equal(await render('zero-opacity',[{...near,opacity:0},{...far,opacity:1}]),opaque,'zero opacity must not occlude')
 const front=await render('opaque-front',[{...near,opacity:1}])
 assert.equal(await render('transparent-behind-opaque',[far,{...near,opacity:1}]),front,'opaque depth must hide transparent geometry behind it')
 const dragged=await render('transparent-drag',[near,far],{ids:['near'],delta:[0,0,-1]})
 assert.notEqual(dragged,ordered,'drag must change transparent depth order')
 assert.equal(await render('transparent-moved',[{...near,z:-.5},far]),dragged)
 await page.evaluate(()=>window.layer.setView({camera:{yaw:0,pitch:-Math.PI/2},viewBox:[-1.5,-1.5,3]}))
 const rotated=await render('transparent-rotated',[near,far])
 assert.equal(await render('transparent-rotated-reversed',[far,near]),rotated)
 await page.evaluate(()=>window.layer.setView({camera:{yaw:0,pitch:Math.PI/2},viewBox:[-1.5,-1.5,3]}))
 // Equal centroids, but opposite depth on the two sides of the intersection.
 // This records an unmet requirement; it must not be counted as an acceptance pass.
 const crossingA={id:'crossing-red',positions:[-1,-1,-.6,1,-1,.6,0,1,0],color:[1,0,0],opacity:.5}
 const crossingB={id:'crossing-blue',positions:[-1,-1,.6,1,-1,-.6,0,1,0],color:[0,0,1],opacity:.5}
 const crossingHash=await render('crossing-red-blue',[crossingA,crossingB])
 const crossingReversedHash=await render('crossing-blue-red',[crossingB,crossingA])
 // Split exactly at x=0: each projected half now has one consistent depth order.
 // Same surfaces, colors, normals and alpha; only triangulation differs.
 const splitCrossing=[crossingA,crossingB].flatMap(body=>[
  {...body,id:body.id+'-left',positions:[...body.positions.slice(0,3),0,-1,0,0,1,0]},
  {...body,id:body.id+'-right',positions:[0,-1,0,...body.positions.slice(3,6),0,1,0]},
 ])
 const referenceHash=await render('crossing-split-reference',splitCrossing)
 assert.equal(await render('crossing-split-reference-reversed',[...splitCrossing].reverse()),referenceHash,'split reference must have unambiguous depth order')
 const crossingTransparency={inputOrderInvariant:crossingHash===crossingReversedHash,matchesSplitReference:crossingHash===referenceHash&&crossingReversedHash===referenceHash,referenceHash,hashes:[crossingHash,crossingReversedHash],scope:'Exact x=0 split reference for two planar triangles; no general transparency acceptance.'}
 assert.equal(crossingTransparency.inputOrderInvariant,true)
 assert.equal(crossingTransparency.matchesSplitReference,true)
 let layeredScene
 if(process.argv.includes('--layers')){
  const layers=Array.from({length:1000},(_,i)=>({id:'layer-'+i,z:-.5+i/1000,color:i%2?[1,0,0]:[0,0,1],opacity:.005}))
  const start=performance.now(),first=await render('1000-layers',layers)
  const loadAndCaptureMs=performance.now()-start
  assert.equal(await render('1000-layers-reversed',[...layers].reverse()),first)
  const frames=await page.evaluate(async()=>{
   const values=[]
   for(let i=0;i<30;i++){
    const start=performance.now();window.layer.setView({camera:{yaw:i*.02,pitch:1.2},viewBox:[-1.5,-1.5,3]})
    await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
    values.push(performance.now()-start)
    if(!window.layer.ready)throw Error('GPU unavailable during layered scene')
   }
   window.layer.setView({camera:{yaw:0,pitch:Math.PI/2},viewBox:[-1.5,-1.5,3]})
   await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))
   return values
  })
  const restored=createHash('sha256').update(await page.locator('canvas').screenshot({path:`${output}/1000-layers-restored.png`})).digest('hex')
  assert.equal(restored,first,'restoring the camera must restore the layered image')
  const sorted=[...frames].sort((a,b)=>a-b)
  layeredScene={layers:1000,loadAndCaptureMs,twoRafFrameMs:{p50:sorted[14],p95:sorted[28],max:sorted[29]},cameraRestored:true,inputOrderInvariant:true,scope:'1000 single-triangle layers; timings include RAF waits, not GPU-only latency or large CAD acceptance.'}
 }
 const disposed=await page.evaluate(()=>{
  window.layer.destroy();window.layer.destroy()
  // Inspect ownership explicitly; GC timing cannot prove that references were released.
  const layer=window.layer
  return {ready:layer.ready,packed:layer.packed,tree:layer.transparentTree,data:layer.transparentData,triangles:layer.transparentTriangles.length,ranges:layer.ranges.size,drag:layer.dragOffsets.size,vertices:layer.vertexCount}
 })
 assert.deepEqual(disposed,{ready:false,packed:null,tree:null,data:null,triangles:0,ranges:0,drag:0,vertices:0})
 assert.deepEqual(await page.evaluate(()=>window.gpuErrors),[])
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),webgpu:true,distinctRenderedMaterials:3,transparentOrdering:true,opaqueOcclusion:true,zeroOpacity:true,dragOrdering:true,cameraOrdering:true,viewUploads,crossingTransparency,layeredScene,disposed,errors,hashes}
 await writeFile(`${output}/report.json`,JSON.stringify(report,null,2)+'\n')
 console.log(report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
