import {expect,it} from 'vitest'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {previewProgressiveNurbsProfiles,type AuthoredProgressiveSweepOptions} from '../src/services/nurbsConstructors'

const profile:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[1,0,0],[2,0,0]],weights:[1,2],periodic:false}
const path:NurbsCurve={...profile,knots:[2,2,5,5],controlPoints:[[0,0,0],[0,0,10]],weights:[1,1]}
const scale={degree:1,knots:[7,7,9,9],values:[1,2],weights:[1,1]}
const twist={...scale,knots:[17,17,19,19],values:[0,14.32394487827058]}
const options:AuthoredProgressiveSweepOptions={orientation:'authored',normal:[1,0,0],maxDeviation:100,initialSections:3,maxSections:9,
 frameAxis:{...scale,knots:[23,23,29,29],values:[[0,0,1],[0,0,1]]},
 frameNormal:{...scale,knots:[31,31,41,41],values:[[1,0,0],[1,1,0]]},
 axisScale:{...scale,knots:[43,43,47,47],values:[[1,1,1],[2,1,1]]},
 centerLaw:{...scale,knots:[59,59,61,61],values:[[0,0,0],[.5,0,0]]}}

it('transports an outward original-law patch bound and improves it with refinement',()=>{
 const before=structuredClone({profile,path,scale,twist,options})
 const coarse=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,3)
 const fine=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,9)
 for(const level of [coarse,fine]){
  expect(level.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true,errorCertificateReason:null,
   continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
  expect(level.report.continuousErrorUpper).toBeGreaterThan(0)
  expect(level.report.knownProfileErrorUpper).toBe(level.report.continuousErrorUpper)
  expect(level.report.errorCertificateCells).toBeLessThanOrEqual(10000)
 }
 expect(fine.report.continuousErrorUpper!).toBeLessThan(coarse.report.continuousErrorUpper!)
 expect({profile,path,scale,twist,options}).toEqual(before)
})
it('covers a genuine path knot transition with original value bounds instead of a false smooth remainder',()=>{
 const bent:NurbsCurve={...path,knots:[2,2,3.2,5,5],controlPoints:[[0,0,0],[0,0,3],[0,0,10]],weights:[1,1,1]}
 const level=previewProgressiveNurbsProfiles([profile],bent,scale,twist,options,3)
 expect(level.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true,errorCertificateReason:null})
 const fine=previewProgressiveNurbsProfiles([profile],bent,scale,twist,options,9)
 expect(fine.report.continuousBound).toBe(true)
 expect(fine.report.continuousErrorUpper!).toBeLessThan(level.report.continuousErrorUpper!)
 for(const t of [.375,.4,.425,.5])for(const u of [0,.375,1]){
  const patch=level.patches[0]!
  const got=evaluateNurbsSurface(patch,u,t).point
  const z=evaluateNurbsCurve(bent,2+3*t).point[2]
  const amplitude=((1+3*u)/(1+u))*(1+t)**2+.5*t
  const angle=Math.atan(t)+.25*t
  const ideal=[amplitude*Math.cos(angle),amplitude*Math.sin(angle),z]
  expect(Math.hypot(...got.map((v,k)=>v-ideal[k]!))).toBeLessThanOrEqual(level.report.continuousErrorUpper!)
 }
})
it('retains partial profile error for refusal without certifying the unresolved union',()=>{
 const fixed:AuthoredProgressiveSweepOptions={...options,maxDeviation:1e-30,
  frameNormal:{...options.frameNormal,values:[[1,0,0],[1,0,0]]},
  axisScale:undefined,centerLaw:undefined}
 const constant={...scale,values:[1,1]}
 const zero={...twist,values:[0,0]}
 const simple={...profile,weights:[1,1]}
 const level=previewProgressiveNurbsProfiles(Array.from({length:32},()=>simple),path,constant,zero,fixed,9)
 expect(level.report).toMatchObject({accepted:false,sampledControlDeviation:0,
  continuousBound:false,roundingCertified:false,continuousErrorUpper:null})
 expect(level.report.knownProfileErrorUpper!).toBeGreaterThan(fixed.maxDeviation)
 expect(level.report.errorCertificateCells).toBeLessThanOrEqual(10000)
})
it('composes simultaneous rational multispan laws with independent domains',()=>{
 const vector=(values:number[][],domain:number[])=>({degree:1,knots:[domain[0]!,domain[0]!,domain[0]!+.4*(domain[1]!-domain[0]!),domain[1]!,domain[1]!],values,weights:[1,.5,2]})
 const joint:AuthoredProgressiveSweepOptions={...options,
  frameAxis:vector([[0,0,1],[0,.2,1],[0,.5,1]],[17,19]),
  frameNormal:vector([[1,0,0],[1,.2,0],[1,1,0]],[23,29]),
  axisScale:vector([[1,1,1],[1.3,1,1],[2,1,1]],[31,41]),
  centerLaw:vector([[0,0,0],[.1,0,0],[.5,0,0]],[43,47])}
 const uniform={...vector([[1,0,0],[1.3,0,0],[2,0,0]],[2,5]),values:[1,1.3,2]}
 const angular={...vector([[0,0,0],[.1,0,0],[.3,0,0]],[7,9]),values:[0,.1*180/Math.PI,.3*180/Math.PI]}
 const coarse=previewProgressiveNurbsProfiles([profile],path,uniform,angular,joint,3)
 const fine=previewProgressiveNurbsProfiles([profile],path,uniform,angular,joint,9)
 for(const level of [coarse,fine]){
  expect(level.report).toMatchObject({continuousBound:true,roundingCertified:true,errorCertificateReason:null})
  expect(level.report.knownProfileErrorUpper).toBe(level.report.continuousErrorUpper)
  expect(level.report.errorCertificateCells).toBeLessThanOrEqual(10000)
 }
 expect(fine.report.continuousErrorUpper!).toBeLessThan(coarse.report.continuousErrorUpper!)
})
