import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createBrepTube,partialAnnularPreview,tessellateNurbsBrep,transformNurbsBrep} from '../src/services/geometry/brep'
import {solidPartialAnnularPreview} from '../src/services/solidPartialAnnularPreview'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'

const directory=resolve(process.argv[2]);mkdirSync(directory,{recursive:true})
const base=createBrepTube(20,5,6),a=.37,b=-.61
const matrix:[[number,number,number,number],[number,number,number,number],[number,number,number,number],[number,number,number,number]]=[
 [Math.cos(a)*Math.cos(b),-Math.sin(a),Math.cos(a)*Math.sin(b),17],
 [Math.sin(a)*Math.cos(b),Math.cos(a),Math.sin(a)*Math.sin(b),-9],
 [-Math.sin(b),0,Math.cos(b),23],[0,0,0,1]]
const cases=[]
for(const [placement,source] of [base,transformNurbsBrep(base,matrix)].entries()) {
 const snapshot=JSON.stringify(source)
 for(const edge of [2,6,9,11]) {
  const result=partialAnnularPreview(source,edge,1.25)
  assert.equal(result.qualification.status,'preview-only')
  assert.equal(result.qualification.commitAllowed,false)
  assert.equal(result.qualification.boundaryIntersectionProof,'unqualified')
  assert.deepEqual(result.model.topologyIds!.bodies,source.topologyIds!.bodies)
  assert.equal(JSON.stringify(source),snapshot)
  const mesh=tessellateNurbsBrep(result.model,12)
  assert(mesh.positions.length>0 && mesh.indices.length>0)
  assert(Array.from(mesh.positions).every(Number.isFinite))
  assert(Array.from(mesh.indices).every(i=>i<mesh.positions.length/3))
  const body={...cadRoadmapParts()[0].body,brep:source,mesh:tessellateNurbsBrep(source,12)}
  const bodySnapshot=JSON.stringify(body)
  const display=solidPartialAnnularPreview(body,edge,1.25)
  assert.equal(display.body.id,body.id)
  assert.equal(display.evidence.qualification.commitAllowed,false)
  assert.equal(JSON.stringify(body),bodySnapshot)
  assert.equal(display.body.mesh.positions.length,mesh.positions.length)
  assert(mainSolidResult(mainSolidExpectation({kind:'partialAnnularPreview',body,edge,radius:1.25}),display))
  cases.push({placement,edge,vertices:mesh.positions.length/3,triangles:mesh.indices.length/3})
 }
}
assert.throws(()=>partialAnnularPreview(base,999,1.25))
const bytes=readFileSync(resolve('public/wasm/geometry-kernel.wasm'))
const report={schema:'cad-partial-preview-wasm/1',scope:'Actual WASM API and display mesh; UI command and full geometric qualification remain open.',wasmSha256:createHash('sha256').update(bytes).digest('hex'),passed:true,cases}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,wasmSha256:report.wasmSha256}))
