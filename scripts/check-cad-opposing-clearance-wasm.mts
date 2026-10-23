import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {measureFaceDistance} from '../src/services/solidMeasurements'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const native=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/opposing-clearance/native.json','utf8'))
const model=native.sourceModel,before=JSON.stringify(model),cases=[]
for(const c of native.cases){
 const options={a:model,b:model,faceA:c.faces[0],faceB:c.faces[1],toleranceMm:1e-5,toleranceUv:1e-8,maxCells:4096,maxDomainCells:100000}
 const started=performance.now(),result=measureFaceDistance(options)
 assert(mainSolidResult(mainSolidExpectation({kind:'faceDistance',options}),result))
 assert(result.converged&&result.distanceIntervalMm[0]<=c.expectedMm&&result.distanceIntervalMm[1]!>=c.expectedMm)
 assert(result.parameters&&result.points&&result.pointEnclosures)
 for(let side=0;side<2;side++){
  const point=evaluateNurbsSurface(model.faces[c.faces[side]].surface,...result.parameters[side]).point
  point.forEach((x,k)=>{
   assert(Math.abs(x-result.points![side][k])<=1e-10)
   const bounds=result.pointEnclosures![side][k]
   assert(x>=bounds[0]-1e-10&&x<=bounds[1]+1e-10)
  })
 }
 cases.push({name:c.name,faces:c.faces,expectedMm:c.expectedMm,elapsedMs:performance.now()-started,result})
}
const messages:any[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
for(const [id,maxCells,maxDomainCells] of [[1,4096,100000],[2,1,10000],[3,4096,1]]){
 const options={a:model,b:model,faceA:0,faceB:3,toleranceMm:1e-5,toleranceUv:1e-8,maxCells,maxDomainCells}
 const job={kind:'faceDistance' as const,options}
 await handle({version:1,id,job});const reply=messages.at(-1)
 assert.equal(reply.ok,true);assert(mainSolidResult(mainSolidExpectation(job),reply.result))
 assert.equal(reply.result.converged,id===1)
 if(id===3)assert.equal(reply.result.points,null)
}
assert.equal(JSON.stringify(model),before)
const document=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/boundary-embedding-wasm/browser-document.json','utf8'))
for(const c of cases)writeFileSync(resolve(directory,c.name+'-browser-fixture.json'),JSON.stringify({document,bodyAName:'Quotient annular',bodyBId:'quotient-annular',faceA:c.faces[0],faceB:c.faces[1],expectedMm:c.expectedMm})+'\n')
const wasm=readFileSync('public/wasm/geometry-kernel.wasm')
const report={schema:'cad-opposing-face-clearance-wasm/1',passed:true,wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,cases,workerHandlerCases:messages,scope:'Selected opposing face clearances; not global wall thickness.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,workerHandlerCases:messages.length,wasmSha256:report.wasmSha256}))
