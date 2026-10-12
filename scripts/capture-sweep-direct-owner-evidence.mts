import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {circleNurbsCurve,bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody,createProgressiveMiterBrepProfileBody} from '../src/services/geometry/brep'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'

const directory=resolve(process.argv[2]),output=resolve(process.argv[3])
const law=(a:number,b:number)=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
const sha=(b:Uint8Array|string)=>createHash('sha256').update(b).digest('hex')
const cases=[]
const capture=(file:string,body:ReturnType<typeof createProgressiveBrepProfileBody>|ReturnType<typeof createProgressiveMiterBrepProfileBody>,miter=false)=>{
 const existing=readFileSync(resolve(directory,file))
 assert.equal(sha(exportDirectStepV5(body.model).text),sha(existing),'Fresh native model differs from qualified STEP: '+file)
 const construction=miter?{boundaryCertificate:(body as ReturnType<typeof createProgressiveMiterBrepProfileBody>).boundaryCertificate}:
  {bodyBoundaryEvidence:{...Object.fromEntries(Object.entries(body).filter(([key])=>key!=='model'&&key!=='approximation')),
   budget:body.approximation.report.budget,closedPath:body.approximation.report.closedPath}}
 const closed=body.approximation.report.closedPath
 const item={file,capFaces:closed?[]:[body.model.faces.length-2,body.model.faces.length-1],construction}
 const result=checkCompleteBoundaryContract(item)
 assert.ok(result,'Missing actual native boundary owner')
 cases.push({...item,result,stepSha256:sha(existing)})
}
for(const orientation of ['fixed','fixed_normal','rmf'] as const){
 const rings=[[circleNurbsCurve([0,0,0],[0,0,1],3)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],1))]]
 const body=createProgressiveBrepProfileBody(rings,bezierNurbsCurve([[0,0,0],[0,0,10]]),law(1,2),law(0,0),
  {normal:[1,0,0],orientation,initialSections:2,maxSections:2,maxDeviation:.01})
 capture(orientation==='rmf'?'rmf-straight-linear-scale-hollow.step':orientation==='fixed'?'fixed-frame-linear-scale-hollow.step':'fixed-normal-linear-scale-hollow.step',body)
}
const closedLoops=[[circleNurbsCurve([0,0,0],[1,0,0],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.2))]]
capture('closed-authored-affine-hollow-miter.step',createProgressiveMiterBrepProfileBody(closedLoops,[[0,0,0],[10,0,0],[10,10,0],[0,10,0]],law(1,1),law(0,0),{
 normal:[0,0,1],closed:true,miterLimit:2,maxDeviation:.001,maxSteps:1,
 axisScale:{degree:1,knots:[2,2,5,5],values:[[2,1,1],[2,1,1]],weights:[1,1]},
 centerLaw:{degree:1,knots:[7,7,9,9],values:[[.125,0,0],[.125,0,0]],weights:[1,1]},
}),true)
const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
capture('progressive-scale-twist-hollow-miter.step',createProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],law(1,1.5),law(0,30),{normal:[1,0,0],maxDeviation:.02,maxSteps:16}),true)
writeFileSync(output,JSON.stringify({scope:'Fresh actual native owner recapture for byte-identical qualified STEP models; not a new independent import.',artifactProvenance:sweepStepArtifactProvenance(),qualifiedManifestSha256:sha(readFileSync(resolve(directory,'manifest.json'))),captureSourceSha256:sha(readFileSync(new URL(import.meta.url))),verifierSha256:sha(readFileSync(new URL('./sweep-complete-boundary-contracts.mjs',import.meta.url))),cases,passed:true},null,2)+'\n')
console.log(cases.map(c=>`${c.file}: ${c.result.status}`).join('\n'))
