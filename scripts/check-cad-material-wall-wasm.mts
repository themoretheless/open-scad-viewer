import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {wallSearchCandidates} from '../src/services/solidWallSearch'
import {inspectMaterialWall,materialWallExpectation,validMaterialWall} from '../src/services/solidMaterialWall'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const fixtures=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/material-wall/contract.json','utf8')).cases
const cases=[]
for(const c of fixtures){
 const before=JSON.stringify(c.request),start=performance.now(),result=inspectMaterialWall(c.request)
 assert(validMaterialWall(materialWallExpectation(c.request),result),c.name)
 assert.equal(result.converged,c.result.converged,c.name);assert.equal(result.reason,c.result.reason,c.name)
 assert.equal(result.clearance.totalPairs,c.result.clearance.totalPairs,c.name)
 if(result.intervalMm){const d=c.result.intervalMm;assert(result.intervalMm[0]<=d[1]&&result.intervalMm[1]>=d[0],c.name)}
 else assert.equal(c.result.intervalMm,null,c.name)
 assert.equal(JSON.stringify(c.request),before,c.name)
 cases.push({name:c.name,elapsedMs:performance.now()-start,result})
}
const messages:any[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m))
for(const [i,c] of fixtures.entries()){
 const job={kind:'materialWall' as const,options:c.request}
 await handler({version:1,id:i+1,job})
 const reply=messages.at(-1);assert.equal(reply.ok,true,c.name)
 assert(mainSolidResult(mainSolidExpectation(job),reply.result),c.name)
 assert.equal(reply.result.converged,c.result.converged,c.name)
}
const automatic=[]
for(const name of ['cube-wall','annular-wall','placed-annular-wall']){
 const c=fixtures.find(c=>c.name===name),before=JSON.stringify(c.request.model)
 const lines=wallSearchCandidates(c.request.model,c.request.faceGroups,undefined,8)
 assert.ok(lines.length>0&&lines.length<=8,name)
 let qualified=null,attempted=0
 for(const line of lines){
  const options={...c.request,origin:line.origin,direction:line.direction},r=inspectMaterialWall(options);attempted++
  assert(validMaterialWall(materialWallExpectation(options),r),name)
  if(r.converged){qualified=r;break}
 }
 assert.ok(qualified,name+' automatic search must find a qualified wall')
 const expected=name==='cube-wall'?10:15
 assert.ok(qualified.intervalMm[0]<=expected&&qualified.intervalMm[1]>=expected,name)
 assert.equal(JSON.stringify(c.request.model),before)
 automatic.push({name,attempted,candidates:lines.length,result:qualified,coverage:'selected-face-unions; search samples do not certify whole-body coverage'})
}
const wasm=readFileSync('public/wasm/geometry-kernel.wasm')
const report={schema:'cad-material-wall-wasm/1',passed:true,wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,cases,automatic,workerHandlerCases:messages,scope:'Aligned material chord minimum bounds between explicitly selected face unions; Automatic sampled candidates qualified on box and annular fixtures; whole-body coverage remains pending.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,workerHandlerCases:messages.length,wasmSha256:report.wasmSha256}))
