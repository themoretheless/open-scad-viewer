// Browser integration only; construction, tessellation and topology are Rust-owned.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {createServer} from 'vite'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const [catalog,output]=process.argv.slice(2)
assert(catalog&&output)
const fixtures=JSON.parse(readFileSync(catalog,'utf8'))
assert.equal(fixtures.length,9)
const cache=mkdtempSync(join(tmpdir(),'source-strata-mesh-'))
const server=await createServer({configFile:false,root:process.cwd(),appType:'custom',logLevel:'warn',cacheDir:cache,
 optimizeDeps:{noDiscovery:true},server:{host:'127.0.0.1',port:0},plugins:[{name:'source-strata-probe',configureServer(s){
  s.middlewares.use((req,res,next)=>{if(req.url!=='/probe')return next();res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Rust source mesh</title>')})
 }}]})
let browser
try{
 await server.listen()
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,channel:'chromium'})
 const page=await browser.newPage(),errors=[]
 page.on('pageerror',e=>errors.push(e.message))
 await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/probe`)
 const cases=[]
 for(const fixture of fixtures){
  const result=await page.evaluate(async({name,request})=>{
   const {warmGeometryKernel,callGeometryRust}=await import('/src/services/geometry/kernel.ts')
   await warmGeometryKernel()
   const before=JSON.stringify(request),{op,...args}=request
   const built=callGeometryRust(op,args)
   if(!built.certified)throw Error(`${name}: uncertified source`)
   const model=JSON.stringify(built.model),levels=[]
   const check=(actual,segments)=>{
    const mesh=callGeometryRust('brep_nurbs_tessellate',{model:actual,segments}),r=mesh.report
    if(!r.closed||['boundaryEdges','nonManifoldEdges','orientationConflicts','degenerateTriangles'].some(k=>r[k]!==0))throw Error(`${name}/${segments}: ${JSON.stringify(r)}`)
    return {segments,triangles:r.triangleCount,closed:r.closed,boundaryEdges:r.boundaryEdges,nonManifoldEdges:r.nonManifoldEdges,orientationConflicts:r.orientationConflicts,degenerateTriangles:r.degenerateTriangles}
   }
   for(const segments of [2,4,8])levels.push(check(built.model,segments))
   const exported=callGeometryRust('brep_nurbs_export_step_v9',{model:built.model})
   const imported=callGeometryRust('brep_nurbs_import_step_v9',{text:exported.text})
   if(!exported.certificate.complete||!imported.certificate.complete)throw Error(`${name}: STEP`)
   const afterStep=check(imported.model,4)
   if(JSON.stringify(built.model)!==model||JSON.stringify(request)!==before)throw Error(`${name}: mutation`)
   return {name,passed:true,levels,afterStep,sourceUnchanged:true,modelUnchanged:true}
  },fixture)
  cases.push(result);console.log(fixture.name,'passed')
 }
 assert.deepEqual(errors,[])
 const wasm=readFileSync('public/wasm/geometry-kernel.wasm')
 assert.deepEqual(wasm,readFileSync('src/generated/geometry-kernels/kernel_bg.wasm'))
 writeFileSync(output,JSON.stringify({schema:'source-strata-tessellation-browser/1',passed:true,browser:browser.version(),
  geometryArtifact:{bytes:wasm.length,sha256:createHash('sha256').update(wasm).digest('hex')},cases,
  scope:'Native construction and manifold tessellation in Chromium WASM, including STEP import. No analytic mesh deviation or complete UI lifecycle qualification.'},null,2)+'\n')
}finally{await browser?.close();await server.close();rmSync(cache,{recursive:true,force:true})}
