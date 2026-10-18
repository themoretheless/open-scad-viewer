// Browser ABI transport and certificate assertions; every geometry stage is Rust.
import assert from 'node:assert/strict'
import {createServer} from 'vite'
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs'
import {join} from 'node:path'
import {tmpdir} from 'node:os'
import {createHash} from 'node:crypto'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const [requestPath,output]=process.argv.slice(2);assert(requestPath&&output)
const bytes=readFileSync(requestPath),cacheDir=mkdtempSync(join(tmpdir(),'osv-interpolated-browser-'))
const server=await createServer({configFile:false,root:process.cwd(),appType:'custom',logLevel:'warn',cacheDir,
 optimizeDeps:{noDiscovery:true},server:{host:'127.0.0.1',port:0},plugins:[{name:'interpolated-carrier-probe',configureServer(server){
  server.middlewares.use((req,res,next)=>{
   if(req.url==='/carrier-input'){res.setHeader('Content-Type','application/json');res.end(bytes);return}
   if(req.url==='/carrier-probe'){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Original carrier certificate</title>');return}
   next()
  })
 }}]})
let browser
try{
 await server.listen();const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,channel:'chromium'})
 const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message))
 await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/carrier-probe`)
 await page.evaluate(()=>{void(async()=>{try{
  const {callGeometryRust,warmGeometryKernel}=await import('/src/services/geometry/kernel.ts')
  await warmGeometryKernel();const input=await(await fetch('/carrier-input')).json()
  const before=JSON.stringify(input)
  const call=({op,...args})=>callGeometryRust(op,args)
  const require=(yes,label)=>{if(!yes)throw Error(label)}
  const full=(r,label)=>{
   for(const key of ['certified','solidGeometryCertified','continuousBound','idealBodyEmbeddingCertified'])require(r[key]===true,`${label}/${key}: ${r.reason}`)
   for(const key of ['embeddingCertified','volumeRegularityCertified','parameterTopologyPreserved'])require(r.idealBody[key]===true,`${label}/${key}`)
   require(r.seamContinuity==='C0'&&r.smoothnessCertified===false,`${label}/explicit C0`)
   require(r.model.faces.length===6,`${label}/faces`)
  }
  const built=call(input);full(built,'construct');require(JSON.stringify(input)===before,'source changed')
  const exported=call({op:'brep_nurbs_export_step_v9',model:built.model});require(exported.certificate.complete,'STEP export')
  const restored=call({op:'brep_nurbs_import_step_v9',text:exported.text});require(restored.certificate.complete,'STEP import')
  for(const key of ['vertices','edges','loops','faces','shells','bodies'])require(restored.model[key].length===built.model[key].length,`STEP/${key}`)
  const tight=structuredClone(input);tight.idealBody.maxCells=built.idealBody.cells;tight.idealBody.maxExactWork=built.idealBody.exactWork
  full(call(tight),'exact ledger')
  for(const [field,value] of [['maxCells',0],['maxCells',built.idealBody.cells-1],['maxExactWork',0],['maxExactWork',built.idealBody.exactWork-1]]){
   const short=structuredClone(tight);short.idealBody[field]=value;short.model=built.model;short.certified=true;short.idealBodyEmbeddingCertified=true
   const r=call(short);require(r.certified===false&&r.idealBodyEmbeddingCertified===false&&r.model===null,`refusal/${field}/${value}`)
  }
  window.carrierReport={passed:true,faces:built.model.faces.length,idealBody:built.idealBody,seamContinuity:built.seamContinuity,smoothnessCertified:built.smoothnessCertified,sourcePreserved:true,stepRoundTripPassed:true,exactBudgetPassed:true,zeroAndOneShortRefusalsPassed:true,forgedFlagsRefused:true}
 }catch(error){window.carrierFailure=String(error?.stack??error)}})()})
 await page.waitForFunction(()=>window.carrierReport||window.carrierFailure,null,{timeout:120000})
 const {report,failure}=await page.evaluate(()=>({report:window.carrierReport,failure:window.carrierFailure}))
 assert(!failure,failure);assert.deepEqual(errors,[])
 writeFileSync(output,JSON.stringify({...report,schema:'original-interpolated-carrier-chromium/1',browser:browser.version(),
  inputSha256:createHash('sha256').update(bytes).digest('hex'),artifact:sweepStepArtifactProvenance(),
  scope:'One specified original carrier case through the current browser Rust/WASM bridge. No Rush editor, UI lifecycle or full semantic G1 qualification is inferred.',
  semanticG1MatrixQualified:false,fullGoalComplete:false},null,2)+'\n')
 console.log('Browser original interpolated carrier passed')
}finally{await browser?.close();await server.close();rmSync(cacheDir,{recursive:true,force:true})}
