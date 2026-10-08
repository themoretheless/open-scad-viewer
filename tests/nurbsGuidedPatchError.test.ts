import {readSweepPatchViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {circleNurbsCurve,bezierNurbsCurve,previewProgressiveNurbsProfiles,progressiveSweepNurbsProfiles,type GuidedProgressiveSweepOptions} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'

const profile=bezierNurbsCurve([[1,0,0],[2,0,0]],[1,2])
const path=bezierNurbsCurve([[0,0,0],[0,0,10]])
const guide=bezierNurbsCurve([[1,0,0],[1,1,10]])
const scale={degree:1,knots:[7,7,9,9],values:[1,2],weights:[1,1]}
const twist={...scale,knots:[17,17,19,19],values:[0,.25*180/Math.PI]}
const options:GuidedProgressiveSweepOptions={normal:[1,0,0],orientationGuide:guide,
 initialSections:3,maxSections:33,maxDeviation:.01,
 axisScale:{degree:1,knots:[23,23,29,29],values:[[1,1,1],[2,1,1]],weights:[1,1]},
 centerLaw:{degree:1,knots:[31,31,41,41],values:[[0,0,0],[.5,0,0]],weights:[1,1]}}

it('carries original guided joint-law bounds through WASM and refinement admission',()=>{
 const coarse=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,3)
 const fine=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,33)
 expect(coarse.report.accepted).toBe(false)
 expect(fine.report.accepted).toBe(true)
 for(const level of [coarse,fine]){
  expect(level.report).toMatchObject({continuousBound:true,roundingCertified:true,errorCertificateReason:null,
   continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
  expect(level.report.knownProfileErrorUpper).toBe(level.report.continuousErrorUpper)
  expect(level.report.errorCertificateCells).toBeLessThanOrEqual(10000)
 }
 expect(fine.report.continuousErrorUpper!).toBeLessThanOrEqual(.01)
 expect(fine.report.continuousErrorUpper!).toBeLessThan(coarse.report.continuousErrorUpper!)
 for(const t of [0,.13,.375,.5,.87,1])for(const u of [0,.375,1]){
  const got=evaluateNurbsSurface(coarse.patches[0]!,u,t).point
  const amplitude=((1+3*u)/(1+u))*(1+t)**2+.5*t
  const angle=Math.atan(t)+.25*t
  const ideal=[amplitude*Math.cos(angle),amplitude*Math.sin(angle),10*t]
  expect(Math.hypot(...got.map((v,k)=>v-ideal[k]!))).toBeLessThanOrEqual(coarse.report.continuousErrorUpper!)
 }
 const result=progressiveSweepNurbsProfiles([profile],path,scale,twist,options)
 expect(result.report).toMatchObject({accepted:true,continuousBound:true})
 expect(result.levels.length).toBeGreaterThan(1)
})

it('keeps arc-length outside the proved original-parameter guided error scope',()=>{
 const level=previewProgressiveNurbsProfiles([profile],path,scale,twist,{...options,spacing:'arc_length'},3)
 expect(level.report).toMatchObject({continuousBound:false,roundingCertified:false,continuousErrorUpper:null,
  errorCertificateReason:'arc-length-correspondence-unproved'})
})

it('refines contact fitted patches using the original anchor and affine laws',()=>{
 const contactOptions:GuidedProgressiveSweepOptions={...options,
  orientationGuide:bezierNurbsCurve([[2,0,0],[2,0,10]]),contactAnchor:{parameter:1}}
 const angular={...twist,values:[0,0]}
 const coarse=previewProgressiveNurbsProfiles([profile],path,scale,angular,contactOptions,3)
 const fine=previewProgressiveNurbsProfiles([profile],path,scale,angular,contactOptions,33)
 expect(coarse.report).toMatchObject({accepted:false,continuousBound:true})
 expect(fine.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true,errorCertificateReason:null})
 expect(fine.report.continuousErrorUpper!).toBeLessThanOrEqual(.01)
 expect(fine.report.continuousErrorUpper!).toBeLessThan(coarse.report.continuousErrorUpper!)
 for(const t of [0,.13,.375,.5,.87,1])for(const u of [0,.375,1]){
  const q=(1+3*u)/(1+u)
  const ideal=[2*(q*(1+t)**2+.5*t)/(2*(1+t)**2+.5*t),0,10*t]
  const got=evaluateNurbsSurface(coarse.patches[0]!,u,t).point
  expect(Math.hypot(...got.map((v,k)=>v-ideal[k]!))).toBeLessThanOrEqual(coarse.report.continuousErrorUpper!)
 }
})

it('keeps a partial contact profile bound for refusal without certifying the union',()=>{
 const simple=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const contactOptions:GuidedProgressiveSweepOptions={normal:[1,0,0],
  orientationGuide:bezierNurbsCurve([[2,0,0],[2,0,10]]),contactAnchor:{parameter:1},
  initialSections:3,maxSections:9,maxDeviation:1e-30}
 const level=previewProgressiveNurbsProfiles(Array.from({length:32},()=>simple),path,
  {...scale,values:[1,1]},{...twist,values:[0,0]},contactOptions,9)
 expect(level.report).toMatchObject({accepted:false,sampledControlDeviation:0,
  continuousBound:false,roundingCertified:false,continuousErrorUpper:null})
 expect(level.report.knownProfileErrorUpper!).toBeGreaterThan(contactOptions.maxDeviation)
 expect(level.report.errorCertificateCells).toBeLessThanOrEqual(10000)
})

it.each(['orientation','contact'] as const)('uses the Rust %s guide bound for Rush refusal and async preview reports',async(mode)=>{
 const source=`// @rush/1
profile = line_curve(start: [1mm,0,0],end: [2mm,0,0])
path = line_curve(start: [0,0,0],end: [0,0,10mm])
rail = line_curve(start: [1mm,0,0],end: [1mm,0,10mm])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 orientation_guide: rail,normal: [1,0,0],initial_sections: 3,max_sections: 3,
 max_deviation: 0.000000000000000000000000000001mm
).nurbs_patches_tessellate(segments: 4)`
 const selected=mode==='contact'?source.replace('orientation_guide: rail,normal:', 'orientation_guide: rail,contact_profile: 0,contact_parameter: 0,normal:'):source
 const {document}=compileRushFrontend(selected)
 expect(()=>buildOwnNurbs(document,{action:'build'})).toThrow(/Progressive sweep continuous retained-patch error/)
 let previews=0
 await expect(buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:(_id,preview)=>{
  previews++
  expect(preview.report).toMatchObject({accepted:false,continuousBound:true,sampledControlDeviation:0})
  expect(preview.report.continuousErrorUpper!).toBeGreaterThan(preview.report.budget)
 }})).rejects.toThrow(/Progressive sweep continuous retained-patch error/)
 expect(previews).toBe(1)
})


it.each([false,true])('certifies a closed guided retained seam (contact=%s)',contact=>{
 const closedPath=circleNurbsCurve([0,0,0],[0,0,1],5)
 const closedGuide=circleNurbsCurve([0,0,1],[0,0,1],5)
 const closedProfile=bezierNurbsCurve([[5,0,1],[5,0,2]])
 const constant={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const level=previewProgressiveNurbsProfiles([closedProfile],closedPath,constant,
  {...constant,values:[0,0]}, {normal:[0,0,1],orientationGuide:closedGuide,
   ...(contact?{contactAnchor:{parameter:0}}:{}),initialSections:33,maxSections:129,maxDeviation:2},129)
 expect(level.report).toMatchObject({closedPath:true,accepted:true,continuousBound:true,roundingCertified:true,errorCertificateReason:null})
 expect(level.report.continuousErrorUpper).toBeGreaterThanOrEqual(0)
 expect(level.report.errorCertificateCells).toBeLessThanOrEqual(10000)
})


it.each(['guided','contact'])('propagates accepted closed %s Rust bounds and preserves strict-budget refusal',async mode=>{
 const source=readFileSync(new URL(`../examples/rush/closed-${mode}-progressive-sweep.r`,import.meta.url),'utf8')
 const {document}=compileRushFrontend(source)
 let previews=0
 let accepted=false
 const acceptance:boolean[]=[]
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:1}},{onSweepPreview:(_id,preview)=>{
  previews++
  expect(preview.report).toMatchObject({closedPath:true,continuousBound:true,roundingCertified:true,
   continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
  accepted=preview.report.accepted
  acceptance.push(accepted)
  expect(preview.report.budget).toBe(2)
  if(accepted)expect(preview.report.continuousErrorUpper!).toBeLessThanOrEqual(preview.report.budget)
 }})
 expect(previews).toBe(2)
 expect(accepted).toBe(true)
 expect(acceptance).toEqual([false,true])
 if(!('nativeGeometry' in built))throw new Error('Missing native patch source')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toBeNull()
 const strict=compileRushFrontend(source.replace('max_deviation: 2mm','max_deviation: 0.000000000000000000000000000001mm')).document
 expect(()=>buildOwnNurbs(strict,{action:'build'})).toThrow(/continuous retained-patch error/)
 await expect(buildOwnNurbsAsync(strict,{action:'build'},{onSweepPreview:(_id,preview)=>{
  expect(preview.report).toMatchObject({closedPath:true,accepted:false,continuousBound:true})
  expect(preview.report.continuousErrorUpper!).toBeGreaterThan(preview.report.budget)
 }})).rejects.toThrow(/continuous retained-patch error/)
})
