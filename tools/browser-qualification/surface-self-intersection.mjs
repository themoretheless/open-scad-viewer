import assert from 'node:assert/strict'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {build,preview} from 'vite'
import {chromium} from 'playwright'
const [wasmPath,outputArg]=process.argv.slice(2)
assert(wasmPath&&outputArg)
const output=resolve(outputArg),root=process.cwd(),dist=resolve('/tmp/osv-surface-worker-dist-20261102')
await mkdir(output,{recursive:true})
const bytes=await readFile(wasmPath)
await build({configFile:false,root,publicDir:false,worker:{format:'es'},build:{outDir:dist,emptyOutDir:true,rollupOptions:{input:resolve(root,'tools/browser-qualification/surface-self-intersection.ts'),output:{entryFileNames:'qualification.js'}}}})
let browser,server
try{
 server=await preview({configFile:false,root,publicDir:false,build:{outDir:dist},preview:{host:'127.0.0.1',port:0}})
 browser=await chromium.launch({headless:true})
 const page=await browser.newPage(),errors=[],workers=[]
 await page.addInitScript(()=>{
  const Native=globalThis.Worker;globalThis.surfaceWorkerAudit={started:0,terminated:0}
  globalThis.Worker=class extends Native{
   constructor(...args){super(...args);globalThis.surfaceWorkerAudit.started++}
   terminate(){globalThis.surfaceWorkerAudit.terminated++;super.terminate()}
  }
 })
 page.on('pageerror',e=>errors.push(e.message));page.on('worker',w=>workers.push(w.url()))
 await page.route('**/__audit.wasm',r=>r.fulfill({contentType:'application/wasm',body:bytes}))
 await page.route('**/__surface_audit',r=>r.fulfill({contentType:'text/html',body:'<!doctype html><script type="module" src="/qualification.js"></script>'}))
 await page.goto(new URL('/__surface_audit',server.resolvedUrls.local[0]).href)
 await page.waitForFunction(()=>typeof globalThis.runSurfaceAuditQualification==='function')
 const result=await page.evaluate(()=>globalThis.runSurfaceAuditQualification())
 assert.equal(result.absence,'absent');assert.equal(result.recovered,'absent')
 assert.equal(result.exact.status,'present');assert.equal(result.exact.witness,null)
 assert.deepEqual(result.exact.exactWitness[0][0],{numerator:'1',denominator:'12'})
 assert.equal(result.rational.status,'present');assert.deepEqual(result.rational.exactWitness[1][0],{numerator:'8',denominator:'9'})
 for(const [key,pattern] of Object.entries({busy:/busy/,cancelled:/cancelled/,deadline:/deadline/,nativeRefusal:/NURBS_INVALID_INPUT/,cloneFailure:/clone|request/i,preAbort:/cancelled/,badBudget:/budget/}))assert.match(result[key],pattern,key)
 assert.equal(result.inputIntact,true);assert.equal(result.noEarlyWorkers,true);assert(result.ticks>0)
 assert.deepEqual(result.ownership,{active:false,workersStarted:76,workersTerminated:76})
 const platformAudit=await page.evaluate(()=>globalThis.surfaceWorkerAudit)
 assert.deepEqual(platformAudit,{started:76,terminated:76});assert.deepEqual(errors,[])
 assert.equal(result.nonlinear.length,5)
 const x=(11-4*Math.sqrt(6))/25,t=(1+Math.sqrt(6))/5
 const expected=[[x,x,1],[0,0,1],[0,0,1],[79/63,79/63,8/7],[79/63,79/63,8/7]]
 const parameters=[[(2-Math.sqrt(2-2*x))/3,(2+t)/3],[.25,.75],[.25,.75],[5/18,5/6],[5/18,5/6]]
 for(let i=0;i<5;i++){
  const r=result.nonlinear[i].report,c=r.certifiedCrossing
  assert.equal(r.status,'present');assert.equal(r.witness,null);assert.equal(r.exactWitness,null);assert.equal(r.allContactBranchesEnumerated,false);assert.equal(c?.method,'original-planar-krawczyk')
  for(let k=0;k<3;k++){const b=c.positionBounds[k];assert(b[0]<=expected[i][k]&&expected[i][k]<=b[1])}
  const axis=i===4?1:0
  for(let k=0;k<2;k++){const b=c.parameterBoxes[k][axis];assert(b[0]<=parameters[i][k]&&parameters[i][k]<=b[1]);assert.deepEqual(c.parameterBoxes[k][1-axis],[.5,.5])}
 }
 for(const {axis,report:r} of result.spatial){
  const c=r.certifiedCrossing;assert.equal(r.status,'present');assert.equal(c.method,'original-curve-surface-krawczyk');assert.equal(r.witness,null);assert.equal(r.exactWitness,null);assert.equal(r.allContactBranchesEnumerated,false)
  for(const [i,t] of [.25,.75].entries())assert(c.parameterBoxes[i][axis][0]<=t&&t<=c.parameterBoxes[i][axis][1])
  const v=c.parameterBoxes[1][1-axis];assert(v[0]<=473/1063&&473/1063<=v[1]);assert(v[1]<.5);assert(r.probeSearch.chargedCells<=10000)
 }
 assert.equal(result.spatial.length,2)
 assert.notEqual(result.separatedReport.status,'present');assert.equal(result.separatedReport.certifiedCrossing,null)
 assert.equal(result.isoTargets.length,2)
 for(const {report:r} of result.isoTargets){assert.equal(r.roots.length,2);assert.equal(r.windowCoverageComplete,true);assert.equal(r.allSurfaceBranchesEnumerated,false);assert.equal(r.bodyAdmission,false);for(const t of [.25,.75]){const root=r.roots.find(x=>x.parameterBox[0][0]<=t&&t<=x.parameterBox[0][1]);assert(root);const z=4/3+t*t*t/4;assert(root.parameterBox[2][0]<=10+2*z&&10+2*z<=root.parameterBox[2][1])}}
 assert.equal(result.isoShort.status,'unresolved');assert.equal(result.isoShort.windowCoverageComplete,false)
 assert.equal(result.isoAbsent.status,'absent-in-window');assert.equal(result.isoAbsent.allSurfaceBranchesEnumerated,false)
 assert.equal(result.offCenterSpatial.status,'present');assert.equal(result.offCenterSpatial.certifiedCrossing.method,'original-curve-surface-krawczyk');assert.notEqual(result.offCenterSpatial.certifiedCrossing.parameterBoxes[0][1][0],.5)
 assert.equal(result.originalPatchReports.length,2)
 for(const {order,report:r} of result.originalPatchReports){assert.equal(r.status,'present');assert.equal(r.curvePatchCoverageComplete,true);assert.equal(r.contacts.length,1);assert.deepEqual(r.contacts[0].multiplicities,[order]);assert(!Object.hasOwn(r.contacts[0],'crossingSign'));assert.equal(r.proofContextSerialized,false);assert.equal(r.bodyAdmission,false)}
 assert.equal(result.originalPatchKnot.contacts.length,1);assert.deepEqual(result.originalPatchKnot.contacts[0].multiplicities,[2,1]);assert.deepEqual(result.originalPatchKnot.contacts[0].parameterBox[0],[1,1])
 assert.equal(result.originalPatchZero.status,'unresolved');assert.equal(result.originalPatchZero.exactWork,0)
 assert.equal(result.originalPatchOutside.status,'absent-on-patch');assert.equal(result.originalPatchBoundary.status,'present');assert.equal(result.originalPatchBoundary.contacts[0].uvMethod,'exact-boundary-projective');assert.equal(result.originalPatchIntervalBoundary.status,'present');assert.equal(result.originalPatchIntervalBoundary.contacts[0].uvMethod,'original-affine-coordinate-bounds')
 assert.equal(result.originalPatchIrrationalBoundary.status,'present');assert.equal(result.originalPatchIrrationalBoundary.curvePatchCoverageComplete,true);assert.deepEqual(result.originalPatchIrrationalBoundary.contacts[0].positionBounds,[[0,0],[0,0],[.5,.5]])
 assert.equal(result.originalPatchCommonRootBoundary.status,'present');assert.equal(result.originalPatchCommonRootBoundary.curvePatchCoverageComplete,true);assert.equal(result.originalPatchCommonRootBoundary.contacts[0].uvMethod,'original-polynomial-common-root')
 assert.equal(result.advancedPatch.length,16);assert.equal(result.commonRootPatch.length,28)
 assert.equal(result.commonRootScalarOnly.status,'absent-on-patch');const budget=result.commonRootAuxiliaryBudget
 assert.equal(budget.status,'unresolved');assert.equal(budget.contacts.length,0);assert(budget.ambiguousParameters.length>0);assert(budget.exactWork<=budget.maxExactWork);assert.equal(budget.exactWork,budget.scalarProofs.reduce((sum,p)=>sum+p.workUsed,0))
 const report={passed:true,browser:browser.version(),wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,installedPackageUpdated:false,result,platformAudit,workers,scope:'Actual Chromium module Workers on an explicit raw candidate; no installed-package or editor/provider qualification'}
 await writeFile(resolve(output,'report.json'),JSON.stringify(report,null,2)+'\n')
 console.log(JSON.stringify(report,null,2))
}finally{await browser?.close();await server?.httpServer.close()}
