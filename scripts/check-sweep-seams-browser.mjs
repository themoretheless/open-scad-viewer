import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {createServer} from 'vite'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const output=path.resolve(process.argv[2]??'docs/qualification/sweep-multispan-seams-2026-10-03')
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const server=await createServer({logLevel:'silent',server:{host:'127.0.0.1',port:0}})
let browser
try {
 await server.listen()
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true})
 const page=await browser.newPage(),errors=[]
 page.on('pageerror',error=>errors.push(error.message))
 const url=server.resolvedUrls.local[0]+'__seam_probe'
 await page.route(url,route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Exact sweep seam probe</title>'}))
 await page.goto(url)
 const cases=await page.evaluate(async()=>{
  const {warmGeometryKernel}=await import('/src/services/geometry/kernel.ts')
  await warmGeometryKernel()
  const {inspectSweepExactStripJets,inspectSweepExactSeams,inspectSweepProjectiveStripJets,inspectSweepProjectiveSeams}=await import('/src/services/nurbsSweepAudit.ts')
  const multi=y=>({degreeU:3,degreeV:1,knotsU:[0,0,0,0,.5,1,1,1,1],knotsV:[0,0,1,1],
   controlPoints:[0,.125,.5,.875,1].map(x=>[[x,y,0],[x,y+1,0]]),weights:[1,2,3,2,1].map(w=>[w,w]),periodicU:false,periodicV:false})
  const a=multi(0),b=multi(1)
  const inspect=(x,y,work=1000000,order=2)=>inspectSweepExactStripJets(x,y,'vMax','vMin',order,1,work)
  const positive=inspect(a,b),cases=[{name:'common-rational-multispan-C2',expected:true,report:positive}]
  const transpose=s=>({degreeU:s.degreeV,degreeV:s.degreeU,knotsU:s.knotsV,knotsV:s.knotsU,
   controlPoints:s.controlPoints[0].map((_,j)=>s.controlPoints.map(row=>row[j])),weights:s.weights[0].map((_,j)=>s.weights.map(row=>row[j])),periodicU:false,periodicV:false})
  cases.push({name:'common-rational-multispan-C2-transposed',expected:true,report:inspectSweepExactStripJets(transpose(a),transpose(b),'uMax','uMin',2,1,1000000)})
  const changed=structuredClone(b);changed.controlPoints[4][1][1]+=2*Number.EPSILON
  cases.push({name:'one-ULP-jet-mismatch',expected:false,report:inspect(a,changed)})
  const basis=structuredClone(b);basis.knotsU[4]=.25
  cases.push({name:'different-along-basis',expected:false,report:inspect(a,basis)})
  cases.push({name:'exact-work-exhaustion',expected:false,report:inspect(a,b,positive.work-1)})
  const c1=y=>({degreeU:2,degreeV:1,knotsU:[0,0,0,.5,1,1,1],knotsV:[0,0,1,1],
   controlPoints:[0,.25,.75,1].map(x=>[[x,y,0],[x,y+1,0]]),weights:Array.from({length:4},()=>[1,1]),periodicU:false,periodicV:false})
  cases.push({name:'common-multispan-C1',expected:true,report:inspect(c1(0),c1(1),1000000,1)})
  cases.push({name:'C1-basis-cannot-claim-C2',expected:false,report:inspect(c1(0),c1(1))})
  const unclamped=structuredClone([a,b]);for(const surface of unclamped)surface.knotsU=[-1,-.5,-.25,0,.5,1,1.25,1.5,2]
  cases.push({name:'unsupported-unclamped-basis',expected:false,report:inspect(...unclamped)})
  const singular=structuredClone([a,b]);for(const surface of singular)for(const row of surface.controlPoints)for(const point of row)point[0]=0
  cases.push({name:'singular-exact-jets',expected:false,report:inspect(...singular)})
  const seams=[0,1].map(i=>({patches:[i,i+1],boundaries:['vMax','vMin'],order:2,normalScale:1,jetTolerance:1e-8}))
  const set=inspectSweepExactSeams([a,b,multi(2)],seams,1000000)
  cases.push({name:'whole-multispan-seam-set-C2',expected:true,report:{...set,certified:set.exactG1G2Certified}})
  const exhausted=inspectSweepExactSeams([a,b,multi(2)],seams,set.seams[0].work)
  cases.push({name:'shared-seam-work-exhaustion',expected:false,report:{...exhausted,certified:exhausted.exactG1G2Certified}})
  const quarter=turn=>({degreeU:1,degreeV:2,knotsU:[0,0,1,1],knotsV:[0,0,0,1,1,1],
   controlPoints:[0,1].map(z=>[[1,0],[1,1],[0,1]].map(([x,y])=>{for(let i=0;i<turn;i++)[x,y]=[-y,x];return [x,y,z]})),
   weights:Array.from({length:2},()=>[1,Math.SQRT1_2,1]),periodicU:false,periodicV:false})
  const arcs=[0,1,2,3].map(quarter)
  for(let i=0;i<4;i++)cases.push({name:`projective-rational-G2-seam-${i}`,expected:true,report:inspectSweepProjectiveStripJets(arcs[i],arcs[(i+1)%4],'vMax','vMin',2,1,1000000)})
  const curvature=structuredClone(arcs[1]);for(const row of curvature.controlPoints)row[2][1]=.125
  cases.push({name:'projective-changed-curvature-G1',expected:true,report:inspectSweepProjectiveStripJets(arcs[0],curvature,'vMax','vMin',1,1,1000000)})
  cases.push({name:'projective-changed-curvature-G2',expected:false,report:inspectSweepProjectiveStripJets(arcs[0],curvature,'vMax','vMin',2,1,1000000)})
  cases.push({name:'projective-zero-budget',expected:false,report:inspectSweepProjectiveStripJets(arcs[0],arcs[1],'vMax','vMin',2,1,0)})
  const declarations=arcs.map((_,i)=>({patches:[i,(i+1)%4],boundaries:['vMax','vMin'],order:2,normalScale:1,jetTolerance:0}))
  const cyclic=inspectSweepProjectiveSeams(arcs,declarations,1000000)
  cases.push({name:'whole-projective-cyclic-G2',expected:true,report:{...cyclic,certified:cyclic.exactG1G2Certified}})
  const closingBudget=cyclic.seams.slice(0,3).reduce((sum,s)=>sum+s.work,0)
  const refusedClosure=inspectSweepProjectiveSeams(arcs,declarations,closingBudget)
  cases.push({name:'projective-closing-seam-budget-exhausted',expected:false,report:{...refusedClosure,certified:refusedClosure.exactG1G2Certified}})
  const {progressiveSweepNurbsProfiles}=await import('/src/services/nurbsConstructors.ts')
  const profiles=arcs.map(s=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:s.controlPoints[0],weights:[1,Math.SQRT1_2,1],periodic:false}))
  const law=value=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
  const built=progressiveSweepNurbsProfiles(profiles,{degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[0,0,8]],weights:[1,1],periodic:false},law(1),law(0),
   {orientation:'fixed',normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:1e-8,
    axisScale:{degree:1,knots:[0,0,1,1],values:[[2,1,1],[2,1,1]],weights:[1,1]},
    centerLaw:{degree:1,knots:[0,0,1,1],values:[[0,0,0],[.5,0,0]],weights:[1,1]}})
  const joins=built.profilePatchRanges.map((range,i)=>({patches:[range[0],built.profilePatchRanges[(i+1)%4][0]],boundaries:['uMax','uMin'],order:2,normalScale:1,jetTolerance:0}))
  const actual=inspectSweepProjectiveSeams(built.patches,joins,1000000)
  cases.push({name:'actual-affine-progressive-sweep-G2-profile-joins',expected:true,report:{...actual,certified:actual.exactG1G2Certified}})
  const kinked=structuredClone(built.patches);for(const surface of kinked)for(const row of surface.controlPoints)row[1][2]+=.5
  const kink=inspectSweepProjectiveSeams(kinked,joins,1000000)
  cases.push({name:'actual-progressive-sweep-internal-kink-refused',expected:false,report:{...kink,certified:kink.exactG1G2Certified}})
  return cases
 })
 for(const entry of cases)assert.equal(entry.report.certified,entry.expected,entry.name)
 assert.deepEqual(errors,[])
 const report={schema:'sweep-multispan-browser-seams/1',recordedAt:new Date().toISOString(),browser:browser.version(),
  scope:'Finite exact represented seam fixtures through TypeScript, binary transport and native WASM. Common clamped along-seam basis, Bezier cross direction, at most 33 controls; not arbitrary moving-frame G1/G2 qualification.',
  geometryWasmSha256:sha(await readFile('public/wasm/geometry-kernel.wasm')),cases,passed:true}
 await mkdir(output,{recursive:true})
 await writeFile(path.join(output,'browser.json'),JSON.stringify(report,null,2)+'\n')
 console.log(`${cases.length} browser seam checks passed`)
}finally{await browser?.close();await server.close()}
