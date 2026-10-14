import {expect,it} from 'vitest'
import {bezierNurbsCurve,previewProgressiveNurbsProfiles} from '../src/services/nurbsConstructors'
import type {NurbsSurface} from '../src/services/nurbsSurface'
import {inspectRetainedPatchRegularity} from '../src/services/nurbsRetainedPatchRegularity'

const patch=(fold=false):NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
 controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[fold?-1:1,1,0]]],weights:[[1,1],[1,1]]})

it('checks a whole retained set with shared work without promoting other obligations',()=>{
 const r=inspectRetainedPatchRegularity([patch(),patch()],1000)
 expect(r).toMatchObject({surfaceRegularityCertified:true,continuousBound:false,globalEmbeddingCertified:false,solidCertified:false,
  unresolvedPatches:[],method:'actual-retained-patch-shared-jacobian-regularity'})
 expect(r.cells).toBeGreaterThan(0)
 const short=inspectRetainedPatchRegularity([patch(),patch()],r.cells-1)
 expect(short.surfaceRegularityCertified).toBe(false)
 expect(short.cells).toBeLessThanOrEqual(r.cells-1)
 expect(short.unresolvedPatches).not.toEqual([])
 expect(inspectRetainedPatchRegularity([patch(),patch()],0)).toMatchObject({surfaceRegularityCertified:false,cells:0,unresolvedPatches:[0,1]})
})

it('refuses an interior fold and invalid input even at zero work',()=>{
 expect(inspectRetainedPatchRegularity([patch(true)],1000)).toMatchObject({surfaceRegularityCertified:false,unresolvedPatches:[0]})
 expect(()=>inspectRetainedPatchRegularity([],1000)).toThrow()
 const invalid=patch();invalid.weights[0]![0]=-1
 expect(()=>inspectRetainedPatchRegularity([invalid],0)).toThrow()
})


it.each(['fixed','fixed_normal','frenet','corrected_frenet','rmf'] as const)('checks actual %s previews without changing refused error admission',orientation=>{
 const path=bezierNurbsCurve([[0,0,0],[.5,0,0],[1,1,0]])
 const profile=bezierNurbsCurve([[0,.1,.1],[0,.2,.1]])
 const scalar=(x:number)=>({degree:1,knots:[0,0,1,1],values:[x,x],weights:[1,1]})
 const level=previewProgressiveNurbsProfiles([profile,profile],path,scalar(1),scalar(0),{
  orientation,normal:[0,0,1],initialSections:3,maxSections:17,maxDeviation:1e-30,
 },17)
 expect(level.report.accepted).toBe(false)
 const r=inspectRetainedPatchRegularity(level.patches,10000)
 expect(r.surfaceRegularityCertified).toBe(true)
 expect(r.globalEmbeddingCertified).toBe(false)
 expect(level.report.accepted).toBe(false)
 const published=previewProgressiveNurbsProfiles([profile],path,scalar(1),scalar(0),{
  orientation,normal:[0,0,1],initialSections:3,maxSections:17,maxDeviation:1,
 },17)
 expect(published.report.accepted).toBe(true)
 expect(published.report.retainedPatchRegularity).toMatchObject({surfaceRegularityCertified:true,globalEmbeddingCertified:false,solidCertified:false})
 if(orientation==='frenet'||orientation==='fixed_normal'){
  expect(published.report.sourceFrameRegularity).toMatchObject({regularityCertified:true,continuousBound:false,surfaceRegularityCertified:false,globalEmbeddingCertified:false})
 }else expect(published.report.sourceFrameRegularity).toBeUndefined()

 // The coincident profiles demonstrate that regularity is not intersection proof.
 expect(inspectRetainedPatchRegularity(level.patches,0).unresolvedPatches).toHaveLength(level.patches.length)
})

it('does not substitute a path Frenet certificate for an authored guide frame',()=>{
 const profile=bezierNurbsCurve([[0,.1,.1],[0,.2,.1]])
 const path=bezierNurbsCurve([[0,0,0],[.5,0,0],[1,1,0]])
 const guide=bezierNurbsCurve([[0,0,1],[.5,0,1],[1,1,1]])
 const scalar=(x:number)=>({degree:1,knots:[0,0,1,1],values:[x,x],weights:[1,1]})
 const level=previewProgressiveNurbsProfiles([profile],path,scalar(1),scalar(0),{
  orientation:'frenet',orientationGuide:guide,normal:[0,0,1],initialSections:3,maxSections:17,maxDeviation:1,
 },17)
 expect(level.report.accepted).toBe(true)
 expect(level.report.sourceFrameRegularity).toBeUndefined()
})
