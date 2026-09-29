import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {createHash} from 'node:crypto'
import ts from 'typescript'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const output=process.argv[2]??'/tmp/solid-material-check'
await mkdir(output,{recursive:true})
const source=await readFile('src/services/solidGpuView.ts','utf8')
const module=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText
const server=createServer((req,res)=>{
 res.setHeader('Content-Type',req.url==='/renderer.js'?'text/javascript':'text/html')
 res.end(req.url==='/renderer.js'?module:'<!doctype html><canvas width="256" height="256" style="width:256px;height:256px;background:white"></canvas>')
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
   if(!window.layer.ready)throw Error('WebGPU device lost')
  },{metallic,roughness})
  const image=await page.locator('canvas').screenshot({path:`${output}/${name}.png`})
  hashes.push(createHash('sha256').update(image).digest('hex'))
 }
 assert.equal(new Set(hashes).size,3,'Each material must change rendered pixels')
 assert.deepEqual(errors,[])
 const report={browser:browser.version(),webgpu:true,distinctRenderedMaterials:3,errors,hashes}
 await writeFile(`${output}/report.json`,JSON.stringify(report,null,2)+'\n')
 console.log(report)
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
