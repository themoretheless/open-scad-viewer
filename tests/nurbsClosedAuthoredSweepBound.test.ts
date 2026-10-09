import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve,previewProgressiveNurbsProfiles} from '../src/services/nurbsConstructors'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
const profile=bezierNurbsCurve([[4.1,0,0],[4.2,0,0]])
const path=circleNurbsCurve([0,0,0],[0,0,1],4)
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const frameAxis={degree:3,knots:[0,0,0,0,.25,.5,.75,1,1,1,1],
 values:[[0,0,1],[.125,0,1],[.375,.125,1],[0,.25,1],[-.375,.125,1],[-.125,0,1],[0,0,1]] as [number,number,number][],weights:[1,1,1,1,1,1,1]}
const frameNormal={degree:1,knots:[0,0,1,1],values:[[1,0,0],[1,0,0]] as [number,number,number][],weights:[1,1]}
const options={orientation:'authored' as const,normal:[1,0,0],frameAxis,frameNormal,
 initialSections:5,maxSections:33,maxDeviation:2}
it('bounds the entire closed authored sweep in original normalized arc length',()=>{
 const axis={degree:1,knots:[0,0,1,1],weights:[1,1],
  values:[[0,0,1],[0,0,1]] as [number,number,number][]}
 const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,
  {...options,frameAxis:axis,spacing:'arc_length',lengthTolerance:.001,lengthMaxCells:100000},33)
 expect(preview.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(preview.report.continuousErrorUpper).toBeLessThanOrEqual(2)
 expect(preview.report.errorCertificateCells).toBeLessThanOrEqual(10000)
})
it.each(['parameter','arc_length'] as const)('certifies the actual closed authored tube boundary and material under default Solid budgets: %s',spacing=>{
 const rotatingAxis={degree:path.degree,knots:path.knots,weights:path.weights,
  values:path.controlPoints.map(p=>[-p[1]!/4,p[0]!/4,0] as [number,number,number])}
 const normal={...frameNormal,values:[[0,0,1],[0,0,1]] as [number,number,number][]}
 const body=createProgressiveBrepProfileBody([[circleNurbsCurve([4,0,0],[0,1,0],.2)]],path,scale,twist,
  {...options,normal:[0,0,1],frameAxis:rotatingAxis,frameNormal:normal,initialSections:17,maxSections:65,spacing})
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
  retainedWalls:{certified:true,faceCoverageCertified:true},retainedCaps:null})
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(2)
 const volume=inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,
  allFacesInjective:true,allPairsClassified:true,nextPair:null,nesting:{rolesConsistent:true}})
 expect(volume.orientations).toEqual([{shell:0,expectedOutward:true,outward:true,attempts:expect.any(Number)}])
},60000)
it('certifies a closed circular profile with rotating original authored axes within the shared budget',()=>{
 const rotatingAxis={degree:path.degree,knots:path.knots,weights:path.weights,
  values:path.controlPoints.map(p=>[-p[1]!/4,p[0]!/4,0] as [number,number,number])}
 const normal={...frameNormal,values:[[0,0,1],[0,0,1]] as [number,number,number][]}
 const tube=previewProgressiveNurbsProfiles([circleNurbsCurve([4,0,0],[0,1,0],.2)],path,scale,twist,
  {...options,normal:[0,0,1],frameAxis:rotatingAxis,frameNormal:normal,initialSections:17,maxSections:65},33)
 expect(tube.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(tube.report.continuousErrorUpper).toBeLessThanOrEqual(2)
 expect(tube.report.errorCertificateCells).toBeLessThanOrEqual(10000)
})
it('includes the copied closing section in the original authored retained-patch bound',()=>{
 const coarse=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,17)
 expect(coarse.report).toMatchObject({closedPath:true,continuousBound:true,accepted:false,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(coarse.report.continuousErrorUpper).toBeGreaterThan(2)
 const fine=previewProgressiveNurbsProfiles([profile],path,scale,twist,options,33)
 expect(fine.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(fine.report.continuousErrorUpper).toBeGreaterThan(0)
 expect(fine.report.continuousErrorUpper).toBeLessThanOrEqual(2)
 expect(fine.report.closedSourceFrameSmoothness?.closedSourceFrameSmoothnessCertified).toBe(true)
})

it('requires one complete shared proof across every closed authored profile',()=>{
 const two=previewProgressiveNurbsProfiles([profile,profile],path,scale,twist,options,33)
 expect(two.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true})
 const limited=previewProgressiveNurbsProfiles(Array(64).fill(profile),path,scale,twist,options,33)
 expect(limited.report.continuousBound).toBe(false)
 expect(limited.report.continuousErrorUpper).not.toEqual(expect.any(Number))
 expect(limited.report.errorCertificateCells).toBeLessThanOrEqual(10000)
})

it.each(['parameter','arc_length'] as const)('composes closed authored affine, center, scale and twist without promoting frame C2: %s',spacing=>{
 const scalar=(values:number[])=>({degree:2,knots:[0,0,0,1,1,1],weights:[1,1,1],values})
 const vector=(values:[number,number,number][])=>({degree:2,knots:[0,0,0,1,1,1],weights:[1,1,1],values})
 const joint=previewProgressiveNurbsProfiles([profile],path,scalar([1,1.02,1]),scalar([0,.03,0]),
  {...options,spacing,axisScale:vector([[1,1,1],[1.05,.95,1],[1,1,1]]),
   centerLaw:vector([[0,0,0],[.01,.02,0],[0,0,0]])},33)
 expect(joint.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true})
 expect(joint.report.continuousErrorUpper).toBeLessThanOrEqual(2)
 expect(joint.report.closedSourceFrameSmoothness?.closedSourceFrameSmoothnessCertified).toBe(false)
})

it('charges a near-closing frame mismatch without treating tolerance as an exact seam',()=>{
 const near={...frameAxis,values:frameAxis.values.map(v=>[...v] as [number,number,number])}
 near.values[near.values.length-1]![0]=1e-12
 const preview=previewProgressiveNurbsProfiles([profile],path,scale,twist,{...options,frameAxis:near},33)
 expect(preview.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true})
 expect(preview.report.originalSectionEndpointErrorUpper).toBeGreaterThan(1e-13)
 expect(preview.report.closedSourceFrameSmoothness?.closedSourceFrameSmoothnessCertified).toBe(false)
})

it('bounds closed guided arc-length transport including the retained copied seam',()=>{
 const rail=circleNurbsCurve([0,0,1],[0,0,1],4)
 const sourceProfile=bezierNurbsCurve([[4,0,.1],[4,0,.2]])
 const before=structuredClone({path,rail,sourceProfile})
 const preview=previewProgressiveNurbsProfiles([sourceProfile],path,scale,twist,
  {normal:[0,0,1],orientation:'rmf',orientationGuide:rail,spacing:'arc_length',lengthTolerance:.001,lengthMaxCells:100000,
   initialSections:5,maxSections:33,maxDeviation:2},33)
 expect(preview.report).toMatchObject({closedPath:true,continuousBound:true,accepted:true,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'})
 expect(preview.report.continuousErrorUpper).toBeGreaterThan(0)
 expect(preview.report.continuousErrorUpper).toBeLessThanOrEqual(2)
 expect(preview.report.errorCertificateCells).toBeGreaterThan(0)
 expect(preview.report.errorCertificateCells).toBeLessThanOrEqual(10000)
 expect({path,rail,sourceProfile}).toEqual(before)
})

it.each(['parameter','arc_length'] as const)('qualifies closed guided tube material within default native Solid budgets: %s',spacing=>{
 const rail=circleNurbsCurve([0,0,1],[0,0,1],4)
 const contour=circleNurbsCurve([4,0,0],[0,1,0],.2)
 const body=createProgressiveBrepProfileBody([[contour]],path,scale,twist,
  {orientation:'rmf',normal:[0,0,1],orientationGuide:rail,spacing,initialSections:17,maxSections:65,maxDeviation:2})
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
  retainedWalls:{certified:true},retainedCaps:null,filledCapErrorUpper:null})
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(2)
 const proof=inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(proof).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,
  allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(proof.individualPairs+proof.groupedPairs).toBe(body.model.faces.length*(body.model.faces.length-1)/2)
 expect(proof.nesting?.rolesConsistent).toBe(true)
 const short=inspectSweepVolume(body.model,[],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1})
 expect(short.solidGeometryCertified).toBe(false)
})
