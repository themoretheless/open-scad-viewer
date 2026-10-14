import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepPatchViewportEvidence} from '../src/services/sweepViewportEvidence'
import {bezierNurbsCurve,inspectProgressiveRetainedProfileJoin,inspectProgressiveRetainedDecompositionJoins,inspectProgressiveRetainedDecompositionSmoothness} from '../src/services/nurbsConstructors'
const profiles=[bezierNurbsCurve([[0,0,0],[1,0,0]]),bezierNurbsCurve([[1,0,0],[2,0,0]])]
const path=bezierNurbsCurve([[0,0,0],[0,0,4]])
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const options={normal:[1,0,0],orientation:'fixed' as const,initialSections:3,maxSections:5,maxDeviation:.01}
it('transports native retained profile G2 with explicit local scope and budget refusal',()=>{
 const audit=(work:number)=>inspectProgressiveRetainedProfileJoin(profiles,path,scale,twist,options,3,0,1,2,1,work)
 const proof=audit(1000000)
 expect(proof).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true,
  profilePatchRanges:[[0,1],[1,2]],scope:'explicit-retained-profile-join-only',
  allProfileJoinsCertified:false,sourceFrameSmoothnessCertified:false,capJoinsCertified:false,
  continuousBound:false,solidCertified:false})
 expect(audit(proof.exactWork-1).certified).toBe(false)
 expect(()=>inspectProgressiveRetainedProfileJoin(profiles,path,scale,twist,options,3,0,0,2,1,1000000)).toThrow()
 const corner=[profiles[0]!,bezierNurbsCurve([[1,0,0],[2,.25,0]])]
 expect(inspectProgressiveRetainedProfileJoin(corner,path,scale,twist,options,3,0,1,1,1,1000000).certified).toBe(false)
})

it('aggregates all native decomposition joins with shared work and explicit coverage',()=>{
 const dense={...profiles[0]!,degree:1,
  knots:[0,...Array.from({length:33},(_,i)=>i/32),1],
  controlPoints:Array.from({length:33},(_,i)=>[i/32,0,0]),weights:Array(33).fill(1)}
 const opts={...options,initialSections:2,maxSections:65}
 const audit=(work:number)=>inspectProgressiveRetainedDecompositionJoins([dense,dense],path,scale,twist,opts,2,2,1,work)
 const positive=audit(1000000)
 expect(positive).toMatchObject({expectedJoins:62,checkedJoins:62,coverageComplete:true,
  decompositionJoinsCertified:true,scope:'within-source-profile-decomposition-only',
  allProfileJoinsCertified:false,closedProfileSeamsCertified:false,sourceFrameSmoothnessCertified:false,
  capJoinsCertified:false,continuousBound:false,solidCertified:false})
 expect(positive.joins.every(j=>j.certified&&j.regularityCertified)).toBe(true)
 expect(positive.joins.reduce((n,j)=>n+j.exactWork,0)).toBe(positive.exactWork)
 expect(audit(positive.exactWork-1).decompositionJoinsCertified).toBe(false)
 expect(audit(0)).toMatchObject({expectedJoins:62,checkedJoins:0,coverageComplete:false,
  decompositionJoinsCertified:false,exactWork:0})
})

it('carries native decomposition G2 through Rush to scoped viewport evidence',async()=>{
 const document=compileRushFrontend(readFileSync('examples/rush/decomposed-profile-g2-progressive-sweep.r','utf8')).document
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 const result=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 expect(result.report.construction?.[node.id]).toMatchObject({accepted:true,
  retainedDecompositionSmoothness:{expectedJoins:31,checkedJoins:31,coverageComplete:true,
   decompositionJoinsCertified:true,scope:'within-source-profile-decomposition-only',solidCertified:false}})
 expect(readSweepPatchViewportEvidence(result.nativeGeometry)).toMatchObject({decompositionG2Certified:true,decompositionJoinCount:31})
})

it('proves native G1 on dense quadratic curvature jumps while retaining G2 refusal',()=>{
 const controls=[[0,0,0]]
 for(let i=0;i<16;i++){
  const slope=i%2===0?0:.25
  controls.push([i+.5,i*.125+slope*.5,0],[i+1,(i+1)*.125,0])
 }
 const dense={...profiles[0]!,degree:2,knots:[0,0,0,...Array.from({length:15},(_,i)=>[(i+1)/16,(i+1)/16]).flat(),1,1,1],
  controlPoints:controls,weights:Array(33).fill(1)}
 const opts={...options,initialSections:2,maxSections:2}
 const proof=inspectProgressiveRetainedDecompositionSmoothness([dense],path,scale,twist,opts,2,1,1000000)
 expect(proof.g2).toMatchObject({expectedJoins:15,coverageComplete:true,decompositionJoinsCertified:false})
 expect(proof.g1).toMatchObject({expectedJoins:15,requestedOrder:1,coverageComplete:true,decompositionJoinsCertified:true})
 expect(proof.decompositionG1Certified).toBe(true)
 expect(proof.exactWork).toBe(proof.g2.exactWork+proof.g1!.exactWork)
 expect(proof.exactWork).toBeLessThanOrEqual(proof.maxExactWork)
 expect(inspectProgressiveRetainedDecompositionSmoothness([dense],path,scale,twist,opts,2,1,0).decompositionG1Certified).toBe(false)
})

it('carries native G1 curvature-jump fallback through Rush without promoting G2',async()=>{
 const document=compileRushFrontend(readFileSync('examples/rush/decomposed-profile-g1-progressive-sweep.r','utf8')).document
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 const result=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 expect(result.report.construction?.[node.id]).toMatchObject({accepted:true,
  retainedDecompositionG1Fallback:{decompositionG1Certified:true,g1:{requestedOrder:1,decompositionJoinsCertified:true},
   g2:{requestedOrder:2,decompositionJoinsCertified:false}}})
 expect(readSweepPatchViewportEvidence(result.nativeGeometry)).toMatchObject({decompositionG1Certified:true,decompositionG2Certified:false})
})
