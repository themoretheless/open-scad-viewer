import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {bezierNurbsCurve,previewProgressiveNurbsProfiles} from '../src/services/nurbsConstructors'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepPatchViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'

it.each(['oblique-rmf-progressive-sweep.r','progressive-sweep.r','affine-progressive-sweep.r','fixed-progressive-sweep.r','closed-fixed-progressive-sweep.r','fixed-normal-progressive-sweep.r','closed-fixed-normal-progressive-sweep.r','frenet-progressive-sweep.r','spatial-frenet-progressive-sweep.r','spatial-frenet-affine-progressive-sweep.r','closed-frenet-progressive-sweep.r'])('propagates original transport retained bounds and strict refusal through Rush: %s',async file=>{
 const source=readFileSync(new URL(`../examples/rush/${file}`,import.meta.url),'utf8')
 const previews:boolean[]=[]
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:1}},{onSweepPreview:(_id,p)=>{
  expect(p.report).toMatchObject({continuousBound:true,roundingCertified:true,continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
  previews.push(p.report.accepted)
 }})
 expect(previews.length).toBeGreaterThan(0)
 if(file==='fixed-progressive-sweep.r')expect(previews[0]).toBe(false)
 expect(previews.at(-1)).toBe(true)
 if(!('nativeGeometry' in built))throw new Error('Missing native fixed patch source')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toBeNull()
 const strict=source.replace(/max_deviation:\s*[0-9.]+mm/,'max_deviation: 0.000000000000000000000000000001mm')
 expect(strict).not.toBe(source)
 await expect(buildOwnNurbsAsync(compileRushFrontend(strict).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
})


it.each(['fixed','fixed_normal','frenet'] as const)('certifies original %s arc-length bounds independently of tolerance admission',orientation=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path=orientation==='frenet'?bezierNurbsCurve([[0,0,0],[0.5,0,0],[1,1,0]]):bezierNurbsCurve([[0,0,0],[0,0,10]])
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const level=previewProgressiveNurbsProfiles([profile],path,law,{...law,values:[0,0]},
  {normal:[1,0,0],orientation,spacing:'arc_length',initialSections:3,maxSections:3,maxDeviation:1},3)
 expect(level.report).toMatchObject({continuousBound:true,roundingCertified:true,errorCertificateReason:null})
 expect(Number.isFinite(level.report.continuousErrorUpper)).toBe(true)
 expect(level.report.accepted).toBe(level.report.continuousErrorUpper!<=level.report.budget)
})

it.each(['rmf'] as const)('keeps curved %s transport without a straight-source certificate',orientation=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path=bezierNurbsCurve([[0,0,0],[0,0,5],[1,0,10]])
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const level=previewProgressiveNurbsProfiles([profile],path,law,{...law,values:[0,0]},
  {normal:[1,0,0],orientation,initialSections:3,maxSections:3,maxDeviation:1},3)
 expect(level.report).toMatchObject({continuousBound:false,roundingCertified:false,continuousErrorUpper:null,errorCertificateReason:'source-plane-coefficients-nonzero'})
})

it.each(['fixed_normal','frenet'] as const)('does not promote partial %s profile certificates to the whole union',orientation=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path=orientation==='frenet'?bezierNurbsCurve([[0,0,0],[1/3,0,0],[2/3,1/3,0],[1,1,1]]):bezierNurbsCurve([[0,0,0],[0,0,10]])
 const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const twist={...scale,values:[0,0]}
 const options={normal:[1,0,0] as [number,number,number],orientation,initialSections:3,maxSections:33,maxDeviation:1}
 const small=previewProgressiveNurbsProfiles([profile,profile],path,scale,twist,options,33)
 expect(small.report).toMatchObject({continuousBound:true,accepted:true})
 const large=previewProgressiveNurbsProfiles(Array.from({length:64},()=>profile),path,scale,twist,options,33)
 expect(large.report).toMatchObject({continuousBound:false,continuousErrorUpper:null})
 expect(large.report.errorCertificateCells).toBeLessThanOrEqual(10000)
 expect(large.report.knownProfileErrorUpper).not.toBeNull()
})


it('certifies original rational straight RMF affine transport and rejects a curved source',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,1]],[1,2])
 const path=bezierNurbsCurve([[0,0,0],[0,0,10]],[1,2])
 const scale={degree:1,knots:[0,0,1,1],values:[1,2],weights:[1,1]}
 const twist={...scale,values:[0,14.32394487827058]}
 const options={orientation:'rmf' as const,normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:129,maxDeviation:.01,
  axisScale:{degree:1,knots:[7,7,9,9],values:[[1,2,1],[2,1,1]] as [number,number,number][],weights:[1,1]},
  centerLaw:{degree:1,knots:[11,11,13,13],values:[[0,0,0],[.125,.25,0]] as [number,number,number][],weights:[1,1]}}
 const level=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,129)
 expect(level.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true})
 expect(level.report.continuousErrorUpper).toBeLessThanOrEqual(.01)
 const near=bezierNurbsCurve([[0,0,0],[2**-1022,0,5],[0,0,10]])
 const refused=previewProgressiveNurbsProfiles([profile],near,scale,twist,options,129)
 expect(refused.report).toMatchObject({continuousBound:false,continuousErrorUpper:null,errorCertificateReason:'source-plane-coefficients-nonzero'})
 const arc=previewProgressiveNurbsProfiles([profile],path,scale,twist,{...options,spacing:'arc_length',maxSections:3},3)
 expect(arc.report).toMatchObject({continuousBound:true,roundingCertified:true,errorCertificateReason:null,accepted:false})
 expect(arc.report.continuousErrorUpper!).toBeGreaterThan(options.maxDeviation)
})

it.each([0,1,2])('preserves translated rational RMF proof on axis %s in both directions',axis=>{
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 for(const direction of [-1,1]){
  const start:[number,number,number]=[3,-2,7]
  const end:[number,number,number]=[...start];end[axis]+=direction*10
  const normal:[number,number,number]=[0,0,0];normal[(axis+1)%3]=1
  const profile=bezierNurbsCurve([start.map((v,k)=>v+normal[k]),start.map((v,k)=>v+2*normal[k])])
  const path=bezierNurbsCurve([start,end],[2,3])
  const level=previewProgressiveNurbsProfiles([profile],path,law,{...law,values:[0,0]},
   {orientation:'rmf',normal,initialSections:3,maxSections:129,maxDeviation:.01},129)
  expect(level.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true,continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
  expect(level.report.continuousErrorUpper).toBeLessThanOrEqual(.01)
 }
})

it('preserves an original oblique rational RMF source certificate',()=>{
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const profile=bezierNurbsCurve([[3,-2,8],[3,-2,9]])
 for(const direction of [-1,1]){
  const path=bezierNurbsCurve([[3,-2,7],[3+3*direction,-2+4*direction,7]],[2,3])
  const level=previewProgressiveNurbsProfiles([profile],path,law,{...law,values:[0,0]},
   {orientation:'rmf',normal:[0,0,1],initialSections:3,maxSections:129,maxDeviation:.01},129)
  expect(level.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true})
  expect(level.report.continuousErrorUpper).toBeLessThanOrEqual(.01)
 }
})
