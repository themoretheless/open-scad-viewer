import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,circleNurbsCurve,progressiveSweepNurbsPatches} from '../src/services/nurbsConstructors'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepPatchViewportEvidence} from '../src/services/sweepViewportEvidence'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

it('certifies closed planar RMF with joint original rational scale/affine/center and full-turn twist',()=>{
 const profile=bezierNurbsCurve([[6,0,1],[6,0,2]])
 const path=circleNurbsCurve([0,0,0],[0,0,1],5)
 const scale={degree:2,knots:[0,0,0,1,1,1],values:[1,1.1,1],weights:[1,1.5,1]}
 const twist={degree:1,knots:[0,0,1,1],values:[0,360],weights:[1,1]}
 const result=progressiveSweepNurbsPatches(profile,path,scale,twist,{
  normal:[0,0,1],orientation:'rmf',initialSections:5,maxSections:129,maxDeviation:.05,
  axisScale:{...scale,values:[[1,1,1],[1.2,.9,1],[1,1,1]]},
  centerLaw:{...scale,values:[[0,0,0],[.1,-.05,0],[0,0,0]]},
 })
 expect(result.report).toMatchObject({closedPath:true,accepted:true,continuousBound:true,seamContinuity:'C0'})
 expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
 expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(.05)
})

it('carries constructor-owned closed RMF evidence through Rush into viewport and refuses a strict budget',async()=>{
 const source=readFileSync('examples/rush/closed-planar-rmf-full-turn-progressive-sweep.r','utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,
  {action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing native patches')
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,budget:.01})
 const strict=source.replace('max_deviation: 0.01mm','max_deviation: 0.000000000000000000000000000001mm')
 await expect(buildOwnNurbsAsync(compileRushFrontend(strict).document,{action:'build'}))
  .rejects.toThrow(/Progressive sweep (?:continuous retained-patch error|sampled refinement).* exceeds 1e-30mm/)
})


it('composes a closed periodic body boundary from owned walls without cap premises',()=>{
 const corners=[[4.9,0,-.1],[5.1,0,-.1],[5.1,0,.1],[4.9,0,.1]]
 const loops=[corners.map((p,i)=>bezierNurbsCurve([p,corners[(i+1)%4]!]))]
 const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
 const body=createProgressiveBrepProfileBody(loops,circleNurbsCurve([0,0,0],[0,0,1],5),
  scalar([1,1]),scalar([0,360]),{normal:[0,0,1],orientation:'rmf',
   initialSections:5,maxSections:129,maxDeviation:.01})
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
  bodyDecompositionErrorUpper:0,retainedWalls:{certified:true},retainedCaps:null,
  capProjection:null,filledCapErrorUpper:null,globalEmbeddingCertified:false})
 expect(body.boundaryErrorUpper).toBeGreaterThan(0)
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(.01)
 expect(body.approximation.report).toMatchObject({closedPath:true,accepted:true,continuousBound:true})
 expect(body.model.faces.length).toBe(4*(body.approximation.report.sections-1))
 const exhausted=inspectSweepVolume(body.model,[],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1})
 expect(exhausted.solidGeometryCertified).toBe(false)
 expect(exhausted.allPairsClassified).toBe(false)
 const volume=inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,
  allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume.groupedPairs).toBeGreaterThan(0)
 expect(volume.individualPairs+volume.groupedPairs).toBe(body.model.faces.length*(body.model.faces.length-1)/2)
 expect(volume.individualPairs).toBeLessThanOrEqual(DEFAULT_SWEEP_VOLUME_BUDGETS.maxPairs)
 expect(volume.nesting?.rolesConsistent).toBe(true)
 expect(volume.orientations).toEqual([{shell:0,expectedOutward:true,outward:true,
  attempts:expect.any(Number)}])
},60000)

it('bounds closed planar arc-length RMF including conic source and joint affine/full-turn laws',()=>{
 const source=circleNurbsCurve([0,0,0],[0,0,1],1)
 const conic=structuredClone(source);conic.weights=conic.weights.map(w=>w===1?1:.5)
 const section=bezierNurbsCurve([[1,0,.1],[1,0,.2]])
 const scalar={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 for(const path of [source,conic])for(const joint of [false,true]){
  const before=structuredClone(path)
  const result=progressiveSweepNurbsPatches(section,path,scalar,{...scalar,values:joint?[0,360]:[0,0]},
   {normal:[0,0,1],orientation:'rmf',spacing:'arc_length',lengthTolerance:.001,lengthMaxCells:100000,
    initialSections:5,maxSections:33,maxDeviation:2,...(joint?{
     axisScale:{...scalar,values:[[2,3,1],[2,3,1]] as [number,number,number][]},
     centerLaw:{...scalar,values:[[.125,-.25,0],[.125,-.25,0]] as [number,number,number][]}}:{})})
  expect(result.report).toMatchObject({closedPath:true,accepted:true,continuousBound:true})
  expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
  expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(2)
  expect(result.report.errorCertificateCells).toBeLessThanOrEqual(10000)
  expect(path).toEqual(before)
 }
})

it.each([{conic:false,orientation:'rmf' as const},{conic:true,orientation:'rmf' as const},
 {conic:false,orientation:'fixed_normal' as const},{conic:true,orientation:'fixed_normal' as const},
 {conic:false,orientation:'corrected_frenet' as const},{conic:true,orientation:'corrected_frenet' as const}])('certifies closed arc-length frame tube material under native budgets: %j',({conic,orientation})=>{
 const path=circleNurbsCurve([0,0,0],[0,0,1],4)
 if(conic)path.weights=path.weights.map(w=>w===1?1:.5)
 const profile=circleNurbsCurve([4,0,0],[0,1,0],.2)
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const body=createProgressiveBrepProfileBody([[profile]],path,law,{...law,values:[0,0]},
  {normal:[0,0,1],orientation,spacing:'arc_length',initialSections:17,maxSections:65,maxDeviation:2})
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,retainedWalls:{certified:true},retainedCaps:null})
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(2)
 const volume=inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume.individualPairs+volume.groupedPairs).toBe(body.model.faces.length*(body.model.faces.length-1)/2)
 expect(volume.nesting?.rolesConsistent).toBe(true)
})


it('owns closed fixed-normal arc bounds with conic and joint laws and refuses a strict tolerance',()=>{
 const circle=circleNurbsCurve([0,0,0],[0,0,1],1)
 const conic=structuredClone(circle);conic.weights=conic.weights.map(w=>w===1?1:.5)
 const profile=bezierNurbsCurve([[1,0,.1],[1,0,.2]])
 const scalar={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 for(const path of [circle,conic])for(const joint of [false,true]){
  const before=structuredClone(path)
  const options={normal:[0,0,1] as [number,number,number],orientation:'fixed_normal' as const,
   spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000,
   initialSections:5,maxSections:33,maxDeviation:2,...(joint?{
    axisScale:{...scalar,values:[[2,3,1],[2,3,1]] as [number,number,number][]},
    centerLaw:{...scalar,values:[[.125,-.25,0],[.125,-.25,0]] as [number,number,number][]}}:{})}
  const twist={...scalar,values:joint?[0,360]:[0,0]}
  const result=progressiveSweepNurbsPatches(profile,path,scalar,twist,options)
  expect(result.report).toMatchObject({closedPath:true,accepted:true,continuousBound:true})
  expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
  expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(2)
  expect(result.report.errorCertificateCells).toBeLessThanOrEqual(10000)
  const strict=progressiveSweepNurbsPatches(profile,path,scalar,twist,{...options,maxDeviation:1e-30})
  expect(strict.report).toMatchObject({accepted:false,continuousBound:true})
  expect(path).toEqual(before)
 }
})


it('carries closed fixed-normal arc boundary evidence through Rush and refuses source tolerance',async()=>{
 const source=readFileSync('examples/rush/closed-arc-length-fixed-normal-tube-body-boundary.r','utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,
  {action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing native body')
 const strict=source.replace('max_deviation: 2mm','max_deviation: 0.000000000000000000000000000001mm')
 await expect(buildOwnNurbsAsync(compileRushFrontend(strict).document,{action:'build'}))
  .rejects.toThrow(/continuous retained-patch error|refinement.*budget|sampled refinement/i)
})

it('owns closed corrected-planar arc bounds with conic and joint laws and refuses a strict tolerance',()=>{
 const circle=circleNurbsCurve([0,0,0],[0,0,1],1)
 const conic=structuredClone(circle);conic.weights=conic.weights.map(w=>w===1?1:.5)
 const profile=bezierNurbsCurve([[1,0,.1],[1,0,.2]])
 const scalar={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 for(const path of [circle,conic])for(const joint of [false,true]){
  const before=structuredClone(path)
  const options={normal:[0,0,1] as [number,number,number],orientation:'corrected_frenet' as const,
   spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000,
   initialSections:5,maxSections:33,maxDeviation:2,...(joint?{
    axisScale:{...scalar,values:[[2,3,1],[2,3,1]] as [number,number,number][]},
    centerLaw:{...scalar,values:[[.125,-.25,0],[.125,-.25,0]] as [number,number,number][]}}:{})}
  const twist={...scalar,values:joint?[0,360]:[0,0]}
  const result=progressiveSweepNurbsPatches(profile,path,scalar,twist,options)
  expect(result.report).toMatchObject({closedPath:true,accepted:true,continuousBound:true})
  expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
  expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(2)
  expect(result.report.errorCertificateCells).toBeLessThanOrEqual(10000)
  const strict=progressiveSweepNurbsPatches(profile,path,scalar,twist,{...options,maxDeviation:1e-30})
  expect(strict.report).toMatchObject({accepted:false,continuousBound:true})
  expect(path).toEqual(before)
 }
})



it('certifies original nonaxial closed corrected arc planes and refuses off-plane coefficients',()=>{
 const source=circleNurbsCurve([0,0,0],[0,0,1],1)
 source.controlPoints=source.controlPoints.map(([x,y])=>[x,-x,y])
 const profile=bezierNurbsCurve([[1,-1,.1],[1,-1,.2]])
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const options={normal:[1,1,0] as [number,number,number],orientation:'corrected_frenet' as const,
  spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000,initialSections:5,maxSections:33,maxDeviation:4,
  axisScale:{...law,values:[[2,3,1],[2,3,1]] as [number,number,number][]},
  centerLaw:{...law,values:[[.125,-.25,0],[.125,-.25,0]] as [number,number,number][]}}
 for(const conic of [false,true]){
  const path=structuredClone(source);if(conic)path.weights=path.weights.map(w=>w===1?1:.5)
  const before=structuredClone(path)
  const result=progressiveSweepNurbsPatches(profile,path,law,{...law,values:[0,360]},options)
  expect(result.report).toMatchObject({continuousBound:true,accepted:true,closedPath:true})
  expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
  expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(4)
  expect(result.report.errorCertificateCells).toBeLessThanOrEqual(10000)
  const damaged=structuredClone(path);damaged.controlPoints[1]![0]=damaged.controlPoints[1]![0]!+Number.EPSILON
  const refused=progressiveSweepNurbsPatches(profile,damaged,law,{...law,values:[0,360]},options)
  expect(refused.report.continuousBound).toBe(false)
  expect(path).toEqual(before)
 }
})


it('builds original nonaxial corrected closed tube through Rush',async()=>{
 const source=readFileSync('examples/rush/closed-nonaxial-arc-length-corrected-tube-body-boundary.r','utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect('nativeGeometry' in built).toBe(true)
})


it.each([{conic:false,orientation:'corrected_frenet' as const},{conic:true,orientation:'corrected_frenet' as const},
 {conic:false,orientation:'rmf' as const},{conic:true,orientation:'rmf' as const}])('qualifies closed nonaxial frame tube material: %j',({conic,orientation})=>{
 const path=circleNurbsCurve([0,0,0],[0,0,1],4)
 path.controlPoints=path.controlPoints.map(([x,y])=>[x,-x,y])
 if(conic)path.weights=path.weights.map(w=>w===1?1:.5)
 const profile=circleNurbsCurve([4,-4,0],[0,0,1],.2)
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const body=createProgressiveBrepProfileBody([[profile]],path,law,{...law,values:[0,0]},
  {normal:[1,1,0],orientation,spacing:'arc_length',initialSections:17,maxSections:65,maxDeviation:2})
 expect(body).toMatchObject({boundaryContinuousBound:true,boundaryErrorWithinBudget:true,retainedWalls:{certified:true},retainedCaps:null})
 const volume=inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume.individualPairs+volume.groupedPairs).toBe(body.model.faces.length*(body.model.faces.length-1)/2)
 expect(volume.nesting?.rolesConsistent).toBe(true)
 const exhausted=inspectSweepVolume(body.model,[],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxCells:1,maxPairs:1})
 expect(exhausted.solidGeometryCertified).toBe(false)
},120000)

it('certifies original nonaxial closed RMF arc planes and refuses off-plane coefficients',()=>{
 const source=circleNurbsCurve([0,0,0],[0,0,1],1)
 source.controlPoints=source.controlPoints.map(([x,y])=>[x,-x,y])
 const profile=bezierNurbsCurve([[1,-1,.1],[1,-1,.2]])
 const law={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
 const options={normal:[1,1,0] as [number,number,number],orientation:'rmf' as const,
  spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000,initialSections:5,maxSections:33,maxDeviation:4,
  axisScale:{...law,values:[[2,3,1],[2,3,1]] as [number,number,number][]},
  centerLaw:{...law,values:[[.125,-.25,0],[.125,-.25,0]] as [number,number,number][]}}
 for(const conic of [false,true]){
  const path=structuredClone(source);if(conic)path.weights=path.weights.map(w=>w===1?1:.5)
  const before=structuredClone(path)
  const result=progressiveSweepNurbsPatches(profile,path,law,{...law,values:[0,360]},options)
  expect(result.report).toMatchObject({continuousBound:true,accepted:true,closedPath:true})
  expect(result.report.continuousErrorUpper).toBeGreaterThan(0)
  expect(result.report.continuousErrorUpper).toBeLessThanOrEqual(4)
  expect(result.report.errorCertificateCells).toBeLessThanOrEqual(10000)
  const damaged=structuredClone(path);damaged.controlPoints[1]![0]=damaged.controlPoints[1]![0]!+Number.EPSILON
  const refused=progressiveSweepNurbsPatches(profile,damaged,law,{...law,values:[0,360]},options)
  expect(refused.report.continuousBound).toBe(false)
  expect(path).toEqual(before)
 }
})


