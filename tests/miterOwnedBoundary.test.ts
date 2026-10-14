import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody} from '../src/services/geometry/brep'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {constructMiterOwnedBoundary,reconstructMiterOwnedBoundary,type MiterOwnedSourceRequest} from '../src/services/nurbsMiterOwnedBoundary'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
const scalar=(value:number)=>({degree:1,knots:[0,0,1,1],controlPoints:[[value,0,0],[value,0,0]],weights:[1,1],periodic:false})
const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
it('replays original source for native station bounds and refuses injected proofs and exhausted work',()=>{
 const source:MiterOwnedSourceRequest={loops:[[circleNurbsCurve([0,0,0],[0,0,1],.5)]],
  points:[[0,0,0],[0,0,10]],scale:scalar(1),twist:scalar(0),normal:[1,0,0],closed:false,
  initialSteps:2,maxSteps:2,miterLimit:4,maxDeviation:.01}
 const before=structuredClone(source)
 const options={quantum:2**-40,tolerance:1,maxWork:100000,budget:1}
 const rebuilt=reconstructMiterOwnedBoundary(source,options)
 const root=constructMiterOwnedBoundary(source)
 const binding={...options,expectedSourceModel:root.model,expectedSourceCertificate:root.boundaryCertificate,expectedSections:root.sections,expectedSharp:root.sharpStationIndices}
 expect(reconstructMiterOwnedBoundary(source,binding).model).not.toBeNull()
 const wrong=structuredClone(root.model)
 wrong.faces[0]!.surface.controlPoints[0]![0]![0]!+=.125
 expect(()=>reconstructMiterOwnedBoundary(source,{...binding,expectedSourceModel:wrong})).toThrow(/differs from original replay/)
 expect(()=>reconstructMiterOwnedBoundary(source,{...binding,expectedSections:[]})).toThrow(/do not reproduce/)
 expect(()=>reconstructMiterOwnedBoundary(source,{...binding,expectedSharp:[0]})).toThrow(/sharp stations differ/)
 const wrongCertificate=structuredClone(root.boundaryCertificate)
 wrongCertificate.wallErrorUpper=0
 expect(()=>reconstructMiterOwnedBoundary(source,{...binding,expectedSourceCertificate:wrongCertificate})).toThrow(/certificate differs/)
 expect(rebuilt.model).not.toBeNull()
 expect(rebuilt.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(rebuilt.solidGeometryCertified).toBe(false)
 const material={...options,requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS}
 const admitted=reconstructMiterOwnedBoundary(source,material)
 expect(admitted.model).not.toBeNull()
 expect(admitted.solidGeometryCertified).toBe(true)
 expect(admitted.volume?.solidGeometryCertified).toBe(true)
 const refused=reconstructMiterOwnedBoundary(source,{...material,wallCells:0})
 expect(refused.model).toBeNull()
 expect(refused.solidGeometryCertified).toBe(false)
 expect(refused.boundaryCertificate?.continuousBound).toBe(true)
 const affine={source,matrix:[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]],
  quantum:options.quantum,maxWork:100000,budget:1,
  requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS}
 const placed=callGeometryRust<any>('brep_miter_owned_place',affine)
 expect(placed.placement.model).not.toBeNull()
 expect(placed.solidGeometryCertified).toBe(true)
 expect(callGeometryRust<any>('brep_miter_owned_place',{...affine,wallCells:0}).placement.model).toBeNull()
 for(const key of ['model','sections','sourceCertificate','boundaryCertificate']){
  expect(()=>callGeometryRust('brep_miter_owned_place',{...affine,[key]:rebuilt.model})).toThrow(/original source only/)
 }
 expect(reconstructMiterOwnedBoundary(source,{...options,maxWork:0}).model).toBeNull()
 expect(reconstructMiterOwnedBoundary(source,{...options,budget:0}).model).toBeNull()
 for(const key of ['model','sections','sharp','sourceCertificate','boundaryCertificate']){
  expect(()=>callGeometryRust('brep_miter_owned_reconstruct',{source,...options,[key]:rebuilt.model})).toThrow(/original source only/)
 }
 expect(source).toEqual(before)
})
it('preserves the pre-adoption hollow geometry and full native reports through the product constructor',()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const points:[number,number,number][]=[[0,0,0],[0,0,10]]
 const correction={quantum:2**-40,tolerance:1e-9,maxWork:100000}
 const request:MiterOwnedSourceRequest={loops,points,scale:scalar(1),twist:scalar(0),normal:[1,0,0],closed:false,
  initialSteps:1,maxSteps:1,miterLimit:4,maxDeviation:.01,circleCorrection:correction}
 const before=structuredClone(request)
 const native=constructMiterOwnedBoundary(request)
 const integrated=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),{
  normal:[1,0,0],initialSteps:1,maxSteps:1,miterLimit:4,maxDeviation:.01,circleCorrection:correction,
 })
 expect(native.model).toEqual(integrated.model)
 expect(native.boundaryCertificate).toEqual(integrated.boundaryCertificate)
 expect(native.levels).toEqual(integrated.approximation.levels)
 const baseline=JSON.parse(readFileSync(new URL('./fixtures/miter-owned-pre-adoption.json',import.meta.url),'utf8'))
 expect(createHash('sha256').update(JSON.stringify(native.model)).digest('hex')).toEqual(baseline.modelSha256)
 expect(native.boundaryCertificate).toEqual(baseline.boundary)
 expect(native.levels).toEqual(baseline.levels)
 expect(integrated.approximation.sections).toEqual(native.sourceSections)
 expect(integrated.sectionCorrection).toEqual(native.sectionCorrection)
 expect(integrated.retainedWallCharts).toEqual(native.retainedWallCharts)
 expect(native.solidGeometryCertified).toBe(false)
 expect(request).toEqual(before)
 for(const key of ['model','sections','boundaryCertificate','sourceCertificate','sourceSections','sectionCorrection','retainedWallCharts','levels','sharpStationIndices','edges']){
  expect(()=>callGeometryRust('brep_miter_owned_construct',{...request,[key]:{continuousBound:true}})).toThrow(/original source only/)
 }
 expect(()=>callGeometryRust('brep_miter_owned_construct',{
  ...request,circleCorrection:{...correction,maxWork:1},
 })).toThrow(/correction unproved/)
})

const vector=(value:[number,number,number])=>({degree:1,knots:[0,0,1,1],controlPoints:[value,value],weights:[1,1],periodic:false})
const limits={maxProducts:100000,maxFaces:1024,exactWork:1000000,wallCells:10000,domainTolerance:.001,domainPairs:1000,domainCells:10000,projectionCells:10000,
 capRegions:{maxWalls:1024,maxExactWork:1000000,maxChartCells:1000,maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000}}
it('preserves independent cap phase budgets and full diagnostics for affine authored guided construction',()=>{
 const request:MiterOwnedSourceRequest={loops:[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]],
  points:[[0,0,0],[0,0,10]],scale:scalar(1),twist:scalar(0),normal:[1,0,0],closed:false,miterLimit:4,initialSteps:1,maxSteps:1,maxDeviation:.01,
  axisScale:vector([2,1,1]),centerLaw:vector([.125,.25,0]),frameAxis:vector([0,0,2]),frameNormal:vector([3,0,0]),
  orientationGuide:{degree:1,knots:[0,0,1,1],controlPoints:[[1,0,0],[1,0,10]],weights:[1,1],periodic:false},
  circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000},limits}
 const body=constructMiterOwnedBoundary(request),level=body.levels.at(-1)!
 expect(body.boundaryCertificate.continuousBound).toBe(true)
 expect(level).toMatchObject({authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true,frameTransportCertified:true,
  profileRegularityCertified:true,continuousBound:false,roundingCertified:false,continuousErrorMethod:'interval-authored-axis-guide-frame-interpolation'})
 for(const key of ['domainExactWork','projectionExactWork'] as const){
  const denied=constructMiterOwnedBoundary({...request,limits:{...limits,[key]:1}})
  expect(denied.boundaryCertificate.continuousBound).toBe(false)
  expect(denied.boundaryCertificate.wallErrorUpper).toEqual(body.boundaryCertificate.wallErrorUpper)
 }
})
it('keeps closed sharp miter stations C0 without filled cap obligations',()=>{
 const request:MiterOwnedSourceRequest={loops:[[circleNurbsCurve([0,0,0],[0,0,1],.5)]],points:[[0,0,0],[0,0,10],[10,0,10],[10,0,0]],
  scale:scalar(1),twist:scalar(0),normal:[1,0,0],closed:true,miterLimit:4,initialSteps:1,maxSteps:1,maxDeviation:.01}
 const body=constructMiterOwnedBoundary(request)
 expect(body.sharpStationIndices).toEqual([0,1,2,3])
 expect(body.boundaryCertificate.continuousBound).toBe(true)
 expect(body.boundaryCertificate.filledCapErrorUpper).toBeNull()
 expect(body.levels.at(-1)).toMatchObject({closedPath:true,seamContinuity:'C0',method:'progressive-miter-fourfold-section-refinement'})
 expect(()=>constructMiterOwnedBoundary({...request,capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}})).toThrow()
})

it('repairs nonplanar moving-axis periodic caps with a charged native displacement',()=>{
 const outer={degree:2,knots:[0,1,2,3,4,5,6,7,8],controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:[1,1,1,1,1,1],periodic:true}
 const hole={...outer,controlPoints:outer.controlPoints.toReversed().map(p=>p.map(x=>x*.25))}
 const moving=(a:[number,number,number],b:[number,number,number])=>({...vector(a),controlPoints:[a,b]})
 const request:MiterOwnedSourceRequest={loops:[[outer],[hole]],points:[[0,0,0],[0,0,10]],scale:scalar(1),twist:moving([0,0,0],[.25,0,0]),
  normal:[1,0,0],closed:false,miterLimit:4,initialSteps:1,maxSteps:32,maxDeviation:.01,
  axisScale:moving([1,1,1],[2,1,1]),centerLaw:moving([0,0,0],[0,.125,.25]),frameAxis:moving([0,0,1],[0,.5,1]),frameNormal:vector([3,0,0]),
  orientationGuide:moving([1,0,0],[1,0,10]),limits:{...limits,capRegions:{...limits.capRegions,maxChartCells:10000,maxTrimPairs:10000}}}
 const original=structuredClone(request),raw=constructMiterOwnedBoundary(request)
 expect(raw.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(raw.sectionCorrection?.reason).toBe('automatic-bounded-cap-planarity')
 expect(raw.boundaryCertificate.errorUpper).toBeLessThan(.01)
 const exhausted=constructMiterOwnedBoundary({...request,limits:{...request.limits!,capRegions:{...request.limits!.capRegions,maxExactWork:0}}})
 expect(exhausted.boundaryCertificate.continuousBound).toBe(false)
 const repaired=constructMiterOwnedBoundary({...request,capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000,authoredFrame:true}})
 expect(repaired.levels.at(-1)?.steps).toBe(16)
 expect(repaired.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(repaired.boundaryCertificate.errorUpper).toBeLessThan(.01)
 expect(repaired.boundaryCertificate.filledCapErrorUpper).toHaveLength(2)
 expect(repaired.solidGeometryCertified).toBe(false)
 expect(request).toEqual(original)
},60000)

it('keeps streamed source and preview mutations out of native final construction and cancels before it',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const points:[number,number,number][]=[[0,0,0],[0,0,10]],scale=law(1),twist=law(0)
 const options={normal:[1,0,0] as [number,number,number],initialSteps:1,maxSteps:1,miterLimit:4,maxDeviation:.01,
  circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}}
 const expected=createProgressiveMiterBrepProfileBody(loops,points,scale,twist,options)
 const source=structuredClone({loops,points,scale,twist,options})
 const stream=streamProgressiveMiterBrepProfileBody(source.loops,source.points,source.scale,source.twist,source.options)
 const first=await stream.next()
 expect(first.done).toBe(false)
 if(first.done)throw new Error('Expected retained preview')
 first.value.patches[0]!.controlPoints[0]![0]![0]=999
 first.value.report.certifiedErrorUpper=0
 source.loops[0]![0]!.controlPoints[0]![0]=999
 source.points[1]![2]=999
 source.scale.values[0]=999
 source.options.maxDeviation=0
 let result=await stream.next();while(!result.done)result=await stream.next()
 expect(result.value).toEqual(expected)
 const abort=new AbortController()
 const cancelled=streamProgressiveMiterBrepProfileBody(loops,points,scale,twist,options,{signal:abort.signal})
 expect((await cancelled.next()).done).toBe(false)
 abort.abort()
 await expect(cancelled.next()).rejects.toMatchObject({name:'AbortError'})
})
