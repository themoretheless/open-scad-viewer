// Browser qualification of the packaged Rust kernel; no UI/renderer claim.
import {createServer} from 'vite'
import {readFile,writeFile,mkdir} from 'node:fs/promises'
import {gunzipSync} from 'node:zlib'
import {createHash} from 'node:crypto'
import {resolve,dirname} from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const fixturePath=resolve(process.argv[2]??'docs/qualification/global-geometry-2026-10-14/nonplanar-fixtures.json.gz')
const output=resolve(process.argv[3]??'/tmp/osv-global-browser-qualification.json')
const bytes=await readFile(fixturePath),json=fixturePath.endsWith('.gz')?gunzipSync(bytes):bytes
const serveFixtures=(req,res,next)=>{
 if(req.url==='/global-geometry-fixtures.json'){res.setHeader('Content-Type','application/json');res.end(json);return}
 if(req.url==='/global-geometry-qualification'){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Native geometry qualification</title>');return}
 next()
}
const server=await createServer({configFile:false,root:process.cwd(),appType:'custom',
 plugins:[{name:'native-geometry-fixtures',configureServer(server){server.middlewares.use(serveFixtures)}}],
 server:{host:'127.0.0.1',port:0},logLevel:'warn'})
await server.listen()
const {playwright}=await loadQualificationPlaywrightPackage()
const browser=await playwright.chromium.launch({headless:true,channel:'chromium'})
try{
 const page=await browser.newPage()
 page.on('console',message=>console.log(message.text()))
 const errors=[];page.on('pageerror',error=>errors.push(error.message))
 await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/global-geometry-qualification`)
 await page.evaluate(()=>{void(async()=>{try{
  const {warmGeometryKernel}=await import('/src/services/geometry/kernel.ts');await warmGeometryKernel()
  const {runGlobalGeometryChecks}=await import('/scripts/globalGeometryChecks.mts')
  const fixtures=await(await fetch('/global-geometry-fixtures.json')).json()
  window.globalGeometryReport=runGlobalGeometryChecks(fixtures,console.log)
 }catch(error){window.globalGeometryFailure=String(error?.stack??error)}})()})
 await page.waitForFunction(()=>window.globalGeometryReport||window.globalGeometryFailure,null,{timeout:1200000})
 const {report,error}=await page.evaluate(()=>({report:window.globalGeometryReport,error:window.globalGeometryFailure}))
 if(error)throw Error(error)
 if(errors.length)throw Error(errors.join('\n'))
 await mkdir(dirname(output),{recursive:true})
 await writeFile(output,JSON.stringify({...report,browser:browser.version(),fixturesSha256:createHash('sha256').update(bytes).digest('hex'),artifact:sweepStepArtifactProvenance()},null,2)+'\n')
 console.log(`Browser passed ${report.passed} cases and ${report.refused} typed refusals`)
}finally{await browser.close();await server.close()}
