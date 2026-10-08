import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,progressiveSweepNurbsPatches} from '../src/services/nurbsConstructors'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepPatchViewportEvidence} from '../src/services/sweepViewportEvidence'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
const profile=bezierNurbsCurve([[0,-1,1],[0,-1,2]])
const path=bezierNurbsCurve([[0,0,0],[.5,0,0],[1,1,0]])
const scale={degree:1,knots:[0,0,1,1],values:[1,2],weights:[1,1]}
const twist={...scale,values:[0,0]}
const options={normal:[0,0,1],orientation:'rmf' as const,initialSections:3,maxSections:33,maxDeviation:.1}
it('composes curved planar RMF wall error with constructor-owned filled caps',()=>{
 const ring=[[0,-.1,-.1],[0,.1,-.1],[0,.1,.1],[0,-.1,.1]]
 const loops=[ring.map((p,i)=>bezierNurbsCurve([p,ring[(i+1)%4]!]))]
 const body=createProgressiveBrepProfileBody(loops,path,{...scale,values:[1,1]},twist,{...options,maxSections:129,maxDeviation:.01})
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
  retainedCaps:{exact:true},retainedWalls:{certified:true},capProjection:{idealCapDomainsCertified:true},globalEmbeddingCertified:false})
 expect(body.boundaryErrorUpper).toBeGreaterThan(0)
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(.01)
})
it('certifies original curved planar RMF transport through packaged WASM',()=>{
 const result=progressiveSweepNurbsPatches(profile,path,scale,twist,options)
 expect(result.report).toMatchObject({accepted:true,continuousBound:true,roundingCertified:true,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(result.report).not.toHaveProperty('globalEmbeddingCertified',true)
 expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
 expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(.1)
 expect(result.patches?.length).toBeGreaterThan(0)
})
it('does not infer RMF planarity from rounded station samples',()=>{
 const changed=structuredClone(path);changed.controlPoints[1]![2]=Number.MIN_VALUE
 const result=progressiveSweepNurbsPatches(profile,changed,scale,twist,options)
 expect(result.report).toMatchObject({continuousBound:false,continuousErrorUpper:null})
 expect(result.report.errorCertificateReason).toMatch(/^source-plane-(coefficients-nonzero|exact-work-unproved)$/)
})
it('preserves joint scale/twist/affine/center laws on original rational planar RMF',()=>{
 const rational=bezierNurbsCurve([[0,0,0],[1/3,0,0],[.5,.5,0]],[1,1.5,2])
 const result=progressiveSweepNurbsPatches(profile,rational,scale,{...twist,values:[0,.25*180/Math.PI]},
  {...options,axisScale:{degree:1,knots:[0,0,1,1],values:[[1,2,1],[2,1,1]],weights:[1,1]},
   centerLaw:{degree:1,knots:[0,0,1,1],values:[[0,0,0],[.125,.25,0]],weights:[1,1]}})
 expect(result.report).toMatchObject({accepted:true,continuousBound:true})
 expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
 expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(.1)
})
it('carries planar RMF evidence through Rust-backed Rush into viewport and refuses a strict budget',async()=>{
 const source=readFileSync(new URL('../examples/rush/planar-rmf-progressive-sweep.r',import.meta.url),'utf8')
 const graph=compileRushFrontend(source).document
 const built=await buildOwnNurbsAsync(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing native patches')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,budget:.1})
 const strict=source.replace('max_deviation: 0.1mm','max_deviation: 0.000000000000000000000000000001mm')
 await expect(buildOwnNurbsAsync(compileRushFrontend(strict).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
})

it('carries exact nonaxial original-plane parameter RMF bounds through Rush into viewport',async()=>{
 const source=readFileSync('examples/rush/nonaxial-planar-rmf-progressive-sweep.r','utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing native patches')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,budget:.05})
})
