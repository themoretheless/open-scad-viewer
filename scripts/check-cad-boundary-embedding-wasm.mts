import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {tessellateNurbsBrep} from '../src/services/geometry/brep'
import {inspectSelfIntersection,selfIntersectionExpectation,validSelfIntersection} from '../src/services/solidSelfIntersection'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const source=resolve(process.argv[3]??'docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/transition-face-pairs/embedding-joined-charts.json')
const model=JSON.parse(readFileSync(source,'utf8')).sourceModel
const toleranceUv=1e-8,maxSpans=4096
const limits={maxPairs:400,maxCells:150000,maxDomainCells:1500000,cellsPerPair:1024,domainCellsPerPair:100000,maxBoxes:64}
const boundaryAudit={exactWork:1000000,trimPairs:1000,trimCells:10000,trimDomainCells:100000}
const before=JSON.stringify(model),cases=[]
for(const name of ['complete','exact-work-exhausted','face-work-exhausted']){
 const audit={...boundaryAudit,exactWork:name==='exact-work-exhausted'?1:boundaryAudit.exactWork}
 const contactLimits=name==='complete'?limits:{...limits,maxPairs:1}
 const spans=name==='face-work-exhausted'?448:maxSpans
 const started=performance.now(),result=inspectSelfIntersection(model,toleranceUv,contactLimits,spans,audit)
 assert(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,contactLimits,spans,audit),result),name+' protocol admission')
 assert.equal(result.boundaryEmbedding?.proven,name==='complete')
 assert.equal(result.absenceProven,name==='complete')
 if(name==='complete'){
  assert.equal(result.visitedPairs,351);assert.equal(result.unresolvedPairCount,0)
  assert.equal(result.pairs.filter(p=>p.sharedBoundary?.kind==='exact-hull'&&p.sharedBoundary.joinedProof?.proven).length,2)
  const expected=selfIntersectionExpectation(model,toleranceUv,contactLimits,spans,audit)
  const changed=structuredClone(model);changed.vertices[0].point[0]+=0.001
  assert.equal(validSelfIntersection(selfIntersectionExpectation(changed,toleranceUv,contactLimits,spans,audit),result),false,'stale geometry')
  assert.equal(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,contactLimits,spans,{...audit,exactWork:1}),result),false,'stale audit budget')
  for(const patch of [{cells:511},{dominanceMarginLower:1e9},{projection:[[1,0,0],[0,1,0]]},{collapsedEnd:1}]){
   const altered=structuredClone(result)
   const certificate=altered.pairs.find(p=>p.sharedBoundary?.kind==='exact-hull'&&p.sharedBoundary.joinedProof)?.sharedBoundary
   assert(certificate?.kind==='exact-hull'&&certificate.joinedProof)
   Object.assign(certificate.joinedProof,patch)
   assert.equal(validSelfIntersection(expected,altered),false,'altered joined proof '+JSON.stringify(patch))
  }
 }else assert(result.pairs.every(p=>p.sharedBoundary?.kind!=='exact-hull'))
 assert.equal(JSON.stringify(model),before)
 const request={op:'cad_self_intersection',model,toleranceUv,...contactLimits,maxSpans:spans,boundaryAudit:audit}
 writeFileSync(resolve(directory,name+'.json'),JSON.stringify({request,result})+'\n')
 cases.push({name,elapsedMs:performance.now()-started,proven:result.boundaryEmbedding?.proven,visitedPairs:result.visitedPairs})
}
const job={kind:'selfIntersection' as const,model,toleranceUv,limits,maxSpans,boundaryAudit}
const messages:any[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
await handle({version:1,id:1,job})
assert.equal(messages.length,1);assert.equal(messages[0].ok,true)
assert(mainSolidResult(mainSolidExpectation(job),messages[0].result))
assert.equal(messages[0].result.boundaryEmbedding.proven,true)
assert.equal(JSON.stringify(model),before)
const mesh=tessellateNurbsBrep(model,12)
writeFileSync(resolve(directory,'browser-document.json'),JSON.stringify({version:1,sketches:[],bodies:[{id:'quotient-annular',name:'Quotient annular',brep:model,mesh:{positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)}}]})+'\n')
const wasm=readFileSync(resolve('public/wasm/geometry-kernel.wasm'))
const report={schema:'cad-boundary-embedding-wasm/1',passed:true,source,wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,cases,workerHandlerCases:1,scope:'Canonical source only. Boundary embedding and self-intersection absence; endpoint G1, thickness and material volume remain unqualified.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
