import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {inspectMaterialChord,inspectMaterialSegment,materialExpectation,validMaterial} from '../src/services/solidMaterialVolume'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const fixtures=JSON.parse(readFileSync(process.argv[3]??'docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/material-segment/contract.json','utf8')).cases
const cases=[]
for(const c of fixtures){
 const before=JSON.stringify(c.request),mode=c.request.op==='cad_material_chord'?'chord':'segment'
 const start=performance.now(),result=mode==='chord'?inspectMaterialChord(c.request):inspectMaterialSegment(c.request)
 assert(validMaterial(materialExpectation(c.request,mode),result),c.name)
 assert.equal(result.proven,c.result.proven,c.name);assert.equal(result.reason,c.result.reason,c.name)
 if(result.method==='continuous-material-chord'){
  assert.equal(result.normalAlignment,c.result.normalAlignment,c.name)
  assert.deepEqual(result.normalEvidence?.aligned??null,c.result.normalEvidence?.aligned??null,c.name)
  if(result.normalEvidence){
   assert.equal(result.normalEvidence.spans,c.result.normalEvidence.spans,c.name)
   for(const [i,a] of result.normalEvidence.endpoints.entries()){
    const native=c.result.normalEvidence.endpoints[i]
    if(a===null){assert.equal(native,null,c.name);continue}
    assert.equal(a.reason,native.reason,c.name);assert.equal(a.face,native.face,c.name)
    assert.deepEqual(a.uv,native.uv,c.name)
    for(const [actual,reference] of [[a.sineSquaredInterval,native.sineSquaredInterval],...(a.normalComponents??[]).map((x,k)=>[x,native.normalComponents[k]])]){
     if(actual===null){assert.equal(reference,null,c.name);continue}
     assert(actual[0]<=reference[1]&&actual[1]>=reference[0],c.name)
    }
   }
  }
 }
 if(result.method==='continuous-material-chord'&&result.proven){
  const d=result.lengthIntervalMm!,native=c.result.lengthIntervalMm
  assert(d[0]<=native[1]&&d[1]>=native[0]);assert(d[1]-d[0]<1e-5)
  assert(result.pointEnclosures&&result.boundary.contacts.length===2)
  for(let side=0;side<2;side++){
   const t=result.boundary.contacts[side].parameter
   for(let k=0;k<3;k++){
    const values=t.map(x=>c.request.origin[k]+x*c.request.direction[k])
    const interval=result.pointEnclosures[side][k],slack=Number.EPSILON*Math.max(1,...values.map(Math.abs),...interval.map(Math.abs))*32
    assert(interval[0]<=Math.max(...values)+slack&&interval[1]>=Math.min(...values)-slack)
   }
  }
 }
 assert.equal(JSON.stringify(c.request),before)
 cases.push({name:c.name,elapsedMs:performance.now()-start,result})
}
const messages:any[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m))
for(const [i,c] of fixtures.entries()){
 const job={kind:c.request.op==='cad_material_chord'?'materialChord' as const:'materialSegment' as const,options:c.request}
 await handler({version:1,id:i+1,job})
 const reply=messages.at(-1);assert.equal(reply.ok,true,c.name)
 assert(mainSolidResult(mainSolidExpectation(job),reply.result),c.name)
 assert.equal(reply.result.proven,c.result.proven,c.name)
}
const wasm=readFileSync('public/wasm/geometry-kernel.wasm')
const report={schema:'cad-continuous-material-wasm/1',passed:true,wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,cases,workerHandlerCases:messages,scope:'Authored interior segments, original-root material chords and requested endpoint normal alignment; global minimum thickness remains unqualified.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,workerHandlerCases:messages.length,wasmSha256:report.wasmSha256}))
