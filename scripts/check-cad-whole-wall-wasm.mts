import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {tessellateNurbsBrep} from '../src/services/geometry/brep'
import {emptyDirectDocument} from '../src/services/directModeling'
import {stringifyMeshJson} from '../src/services/meshJson'
import {wholeWallSearchCandidates} from '../src/services/solidWallSearch'
import {inspectWholeWall,wholeWallExpectation,validWholeWall} from '../src/services/solidWholeWall'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const fixtures=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/whole-wall/contract.json','utf8')).cases
const cases=[]
for(const c of fixtures){
 const before=JSON.stringify(c.request),start=performance.now(),result=inspectWholeWall(c.request)
 assert(validWholeWall(wholeWallExpectation(c.request),result),c.name)
 assert.equal(result.converged,c.result.converged,c.name);assert.equal(result.reason,c.result.reason,c.name)
 assert.equal(result.coverage.totalPairs,c.result.coverage.totalPairs,c.name)
 if(result.intervalMm){const d=c.result.intervalMm;assert(result.intervalMm[0]<=d[1]&&result.intervalMm[1]>=d[0],c.name)}
 else assert.equal(c.result.intervalMm,null,c.name)
 assert.equal(JSON.stringify(c.request),before,c.name)
 cases.push({name:c.name,elapsedMs:performance.now()-start,result})
}
const messages:any[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m))
for(const [i,c] of fixtures.entries()){
 const job={kind:'wholeWall' as const,options:c.request}
 await handler({version:1,id:i+1,job})
 const reply=messages.at(-1);assert.equal(reply.ok,true,c.name)
 assert(mainSolidResult(mainSolidExpectation(job),reply.result),c.name)
 assert.equal(reply.result.converged,c.result.converged,c.name)
}
const automatic=[]
for(const name of ['box-minimum','enclosure-minimum']){
 const c=fixtures.find(c=>c.name===name),before=JSON.stringify(c.request.model)
 const lines=wholeWallSearchCandidates(c.request.model,undefined,64)
 assert.ok(lines.length>0&&lines.length<=64,name)
 let qualified=null,attempted=0
 for(const line of lines){
  const options={...c.request,origin:line.origin,direction:line.direction},r=inspectWholeWall(options);attempted++
  assert(validWholeWall(wholeWallExpectation(options),r),name)
  if(r.converged){qualified=r;break}
 }
 assert.ok(qualified,name+' automatic search must find a qualified wall')
 const expected=name==='box-minimum'?10:1.4
 assert.ok(qualified.intervalMm[0]<=expected&&qualified.intervalMm[1]>=expected,name)
 assert.equal(JSON.stringify(c.request.model),before)
 automatic.push({name,attempted,candidates:lines.length,result:qualified,coverage:'all original face pairs; candidate sampling supplies certified upper witnesses only'})
}
const wasm=readFileSync('public/wasm/geometry-kernel.wasm')
const report={schema:'cad-whole-wall-wasm/1',passed:true,wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,cases,automatic,workerHandlerCases:messages,scope:'Aligned material chord minimum over all original face pairs: box and planar enclosure qualified; curved same-face domains conservatively unresolved.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,workerHandlerCases:messages.length,wasmSha256:report.wasmSha256}))
const browserFixtures=fixtures.filter(c=>['box-minimum','enclosure-minimum','curved-self-unresolved'].includes(c.name)).map(c=>{
 const document=emptyDirectDocument();document.bodies=[{id:'whole-wall',name:'Whole wall specimen',brep:c.request.model,mesh:tessellateNurbsBrep(c.request.model,8)}]
 return {name:c.name,origin:c.request.origin,direction:c.request.direction,minimum:c.name==='box-minimum'?10:1.4,document:JSON.parse(stringifyMeshJson(document))}
})
writeFileSync(resolve(directory,'browser-fixtures.json'),JSON.stringify(browserFixtures)+'\n')
