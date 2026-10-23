import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {transformNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {selfIntersectionExpectation,validSelfIntersection,inspectSelfIntersection} from '../src/services/solidSelfIntersection'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'

const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const fixture=JSON.parse(readFileSync(resolve('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/pole-quotient/combined-native.json'),'utf8'))
const {model:legacyBase,toleranceUv,maxSpans:_budget,op:_op,...limits}=fixture.request
const sourceFixture=process.argv[3] ? resolve(process.argv[3]) : null
const base=sourceFixture ? JSON.parse(readFileSync(sourceFixture,'utf8')).sourceModel : legacyBase
assert(base && Array.isArray(base.faces), 'Qualification fixture must contain a source model')
const a=.37,b=-.61
const matrix:[[number,number,number,number],[number,number,number,number],[number,number,number,number],[number,number,number,number]]=[
 [Math.cos(a)*Math.cos(b),-Math.sin(a),Math.cos(a)*Math.sin(b),17],
 [Math.sin(a)*Math.cos(b),Math.cos(a),Math.sin(a)*Math.sin(b),-9],
 [-Math.sin(b),0,Math.cos(b),23],[0,0,0,1]]
const cases=[]
for(const [placement,model] of [base,transformNurbsBrep(base,matrix)].entries()){
 const before=JSON.stringify(model)
 for(const maxSpans of [4096,600,448]){
  const result=inspectSelfIntersection(model,toleranceUv,limits,maxSpans)
  assert(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),result))
  assert.equal(result.absenceProven,false)
  assert.equal(result.allFacesInjective,maxSpans===4096)
  const proven=result.faces.filter(f=>f.result?.proven).length
  if(maxSpans===4096){
  for(const i of [0,10])assert.equal(result.faces[i].result?.reason,'collapsed-boundary-requires-quotient-proof')

   assert.equal(proven,25)
   assert.equal(result.faces.filter(f=>f.quotientProof?.proven).length,2)
   for(const i of [0,10])assert.equal(result.faces[i].quotientProof?.cells,256)
   assert.equal(result.faces[5].result?.reason,'global-polar-projection-contraction')
   assert(result.faces[5].result!.contractionUpper!<1)
  }else assert(proven<25)
  assert.equal(JSON.stringify(model),before)
  cases.push({placement,maxSpans,spans:result.spans,proven,quotients:result.faces.filter(f=>f.quotientProof).map(f=>({face:f.face,proof:f.quotientProof})),torus:result.faces[5].result})
 }
 const messages:any[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 await handle({version:1,id:1,job:{kind:'selfIntersection',model,toleranceUv,limits,maxSpans:4096}})
 assert.equal(messages.length,1);assert.equal(messages[0].ok,true)
 assert(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,4096),messages[0].result))
 assert.equal(messages[0].result.allFacesInjective,true)
 assert.equal(messages[0].result.faces.filter((f:any)=>f.quotientProof?.proven).length,2)
 assert.equal(JSON.stringify(model),before)
}
const wasm=readFileSync(resolve('public/wasm/geometry-kernel.wasm'))
const mesh=tessellateNurbsBrep(base,12)
writeFileSync(resolve(directory,'browser-document.json'),JSON.stringify({version:1,sketches:[],bodies:[{id:'quotient-annular',name:'Quotient annular',brep:base,mesh:{positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)}}]})+'\n')
const report={schema:'cad-quotient-injectivity-wasm/1',sourceFixture,passed:true,wasmSha256:createHash('sha256').update(wasm).digest('hex'),wasmBytes:wasm.length,cases,workerHandlerCases:2,scope:'Actual WASM and worker handler; within-face quotient absence proven; endpoint G1 and distinct-face contacts remain unqualified.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,workerHandlerCases:2,wasmSha256:report.wasmSha256}))
