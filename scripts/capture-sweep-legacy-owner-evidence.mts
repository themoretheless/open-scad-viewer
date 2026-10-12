// Native original-source recapture. JS performs transport and byte/scope checks.
import {readFileSync,writeFileSync,createReadStream} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {constructMiterOwnedBoundary,type DirectMiterOwnedSourceRequest} from '../src/services/nurbsMiterOwnedBoundary'
import {transformNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
import {callGeometryRust} from '../src/services/geometry/kernel'
import type {SweepBoundaryCertificate} from '../src/services/sweepBoundaryCertificate'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'

const directory=resolve(process.argv[2]),output=resolve(process.argv[3])
const sha=(b:Uint8Array|string)=>createHash('sha256').update(b).digest('hex')
const artifactProvenance=sweepStepArtifactProvenance()
const oraclePath=resolve(directory,'opencascade-sweep.json')
const oracle=JSON.parse(readFileSync(oraclePath,'utf8'))
assert.equal(oracle.passed,true,'Reference independent import did not pass')
assert.equal(oracle.cases.length,157,'Incomplete reference corpus')
const imported=new Map<string,{stepSha256:string}>(oracle.cases.map((item:{file:string;stepSha256:string})=>[item.file,item]))
assert.equal(imported.size,157,'Duplicate imported STEP identities')
const manifestHash=createHash('sha256')
for await(const chunk of createReadStream(resolve(directory,'manifest.json')))manifestHash.update(chunk)
const referenceManifestSha256=manifestHash.digest('hex')
assert.equal(referenceManifestSha256,oracle.manifestSha256,'Reference manifest differs from qualified import')
const rings=(axis:[number,number,number]=[0,0,1])=>[[circleNurbsCurve([0,0,0],axis,.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],axis,.2))]]
const straight:[number,number,number][]=[[0,0,0],[0,0,10]]
const spatial:[number,number,number][]=[...straight,[10,0,10],[10,10,15]]
const capCorrection={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
const source=(loops=rings(),points=straight):DirectMiterOwnedSourceRequest=>({direct:true,loops,points,normal:[1,0,0],closed:false,miterLimit:2,maxDeviation:.01})
const quarters=(r:number):NurbsCurve[]=>[
 [[r,0],[r,r],[0,r]],[[0,r],[-r,r],[-r,0]],
 [[-r,0],[-r,-r],[0,-r]],[[0,-r],[r,-r],[r,0]],
].map(p=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:p.map(([x,y])=>[x!,y!,0]),weights:[1,Math.SQRT1_2,1],periodic:false}))
const matrix=[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]
const requests:[string,DirectMiterOwnedSourceRequest,boolean?][]=[
 ['open-straight-hollow-miter.step',source()],
 ['open-spatial-hollow-miter.step',source(rings(),spatial)],
 ['open-spatial-bounded-correction.step',{...source(rings(),spatial),capCorrection,capEndpoints:[false,true]}],
 ['open-spatial-constructor-correction.step',{...source(rings(),spatial),capCorrection}],
 ['closed-planar-hollow-miter.step',{...source(rings([1,0,0]),[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]),normal:[0,0,1],closed:true}],
 ['affine-oblique-hollow-miter.step',source(),true],
 ['affine-oblique-dyadic-hollow.step',source([quarters(.5),quarters(.25).reverse().map(reverseNurbsCurve)]),true],
]
const cases=[]
for(const [file,request,affine] of requests){
 const before=JSON.stringify(request),owned=constructMiterOwnedBoundary(request)
 assert.equal(JSON.stringify(request),before,'Native request mutated')
 let model=owned.model,boundaryCertificate=owned.boundaryCertificate
 if(affine){
  const expected=transformNurbsBrep(model,matrix)
  const placed=callGeometryRust<{placement:{model:NurbsBrep}|null;boundaryCertificate:SweepBoundaryCertificate;resultModelBound:boolean}>('brep_miter_owned_place',{
   source:request,matrix,quantum:2**-40,maxWork:100000,budget:.01,
   expectedSourceModel:model,expectedSourceCertificate:boundaryCertificate,expectedResultModel:expected,
  })
  assert.equal(placed.resultModelBound,true)
  assert.ok(placed.placement?.model)
  model=placed.placement.model;boundaryCertificate=placed.boundaryCertificate
 }
 const stepSha256=sha(readFileSync(resolve(directory,file)))
 assert.equal(imported.get(file)?.stepSha256,stepSha256,'Reference file differs from independently imported STEP')
 assert.equal(sha(exportDirectStepV5(model).text),stepSha256,'Fresh actual native owner model differs from qualified STEP: '+file)
 const item={file,capFaces:request.closed?[]:[model.faces.length-2,model.faces.length-1],construction:{boundaryCertificate}}
 const result=checkCompleteBoundaryContract(item)
 assert.ok(result,'Actual native owner missing: '+file)
 assert.equal(result.status,file==='open-spatial-hollow-miter.step'?'unproved':'certified')
 cases.push({...item,sourceRequest:request,affineMatrix:affine?matrix:null,result,stepSha256})
}
assert.equal(cases.length,7)
writeFileSync(output,JSON.stringify({scope:'Seven fresh legacy original-source native owners for byte-identical qualified STEP geometry; not a new independent import or old-artifact certificate transfer.',artifactProvenance,
 referenceArtifactProvenance:oracle.artifactProvenance,referenceManifestSha256,referenceOracleSha256:sha(readFileSync(oraclePath)),
 captureSourceSha256:sha(readFileSync(new URL(import.meta.url))),validatorSha256:sha(readFileSync(new URL('./sweep-complete-boundary-contracts.mjs',import.meta.url))),
 certified:6,unproved:1,cases,passed:true},null,2)+'\n')
console.log(cases.map(c=>`${c.file}: ${c.result.status}`).join('\n'))
