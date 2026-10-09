import {expect,it} from 'vitest'
import {bezierNurbsCurve,type ProgressiveGuidedSurfaceSweepOptions} from '../src/services/nurbsConstructors'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {readSweepBodyBoundaryViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
const points=[[0,0,0],[2,0,0],[2,2,0],[0,2,0]]
const loops=[points.map((point,i)=>bezierNurbsCurve([point,points[(i+1)%4]!]))]
const path=bezierNurbsCurve([[0,0,0],[0,0,10]])
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const vector=(value:[number,number,number])=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const base:ProgressiveGuidedSurfaceSweepOptions={orientation:'rmf',normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:.01}
it.each(['corrected-frenet-affine-zero-curvature-hollow-body-boundary.r','arc-length-corrected-frenet-affine-zero-curvature-hollow-body-boundary.r','corrected-frenet-affine-inflection-hollow-body-boundary.r','arc-length-corrected-frenet-affine-inflection-hollow-body-boundary.r','corrected-frenet-rational-straight-affine-hollow-body-boundary.r','arc-length-corrected-frenet-rational-straight-affine-hollow-body-boundary.r','rmf-rational-straight-affine-hollow-body-boundary.r','arc-length-rmf-rational-straight-affine-hollow-body-boundary.r','corrected-frenet-straight-affine-hollow-body-boundary.r','corrected-frenet-inflection-hollow-body-boundary.r','arc-length-corrected-frenet-inflection-hollow-body-boundary.r'])('qualifies corrected source wall/caps/hole and independent Solid admission: %s',async file=>{
 const graph=compileRushFrontend(readFileSync(new URL(`../examples/rush/${file}`,import.meta.url),'utf8')).document
 const before=structuredClone(graph)
 if(file.includes('straight'))expect(graph.nodes.find(n=>n.op==='brep_progressive_sweep')).toMatchObject({
  axis_scale:{values:[[2,3,1],[2,3,1]]},center_law:{values:[[.125,-.25,0],[.125,-.25,0]]}})
 if(/affine-(inflection|zero-curvature)/.test(file))expect(graph.nodes.find(n=>n.op==='brep_progressive_sweep')).toMatchObject({axis_scale:{values:[[1.25,.75,1],[1.25,.75,1]]},center_law:{values:[[.002,-.003,0],[.002,-.003,0]]}})
 const built=await buildOwnNurbsAsync(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw new Error('Missing corrected retained body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,
  budget:file.includes('straight')?.01:.1})
 const retained=JSON.parse(built.nativeGeometry.geometryJson).geometry
 if(file.includes('straight')) {
  const poles=retained.faces.slice(0,-2).flatMap((face:any)=>face.surface.controlPoints.flat())
  expect([0,1,2].map(k=>[Math.min(...poles.map((p:number[])=>p[k]!)),Math.max(...poles.map((p:number[])=>p[k]!))]))
   .toEqual([[.125,4.125],[-.25,5.75],[0,10]])
 }
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,retained)
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allPairsClassified:true,nextPair:null})
 expect(volume!.individualPairs).toBeLessThanOrEqual(10000)
 expect(volume!.individualPairs+volume!.groupedPairs).toBe(retained.faces.length*(retained.faces.length-1)/2)
 expect(volume!.groupCells).toBeGreaterThan(0)
 expect(graph).toEqual(before)
})
it.each(['plain','affine','authored','guide'])('preserves constructor-owned wall/cap bounds through WASM: %s',mode=>{
 const options:ProgressiveGuidedSurfaceSweepOptions={...base}
 if(mode!=='plain'){options.axisScale=vector([2,3,1]);options.centerLaw=vector([0,0,0])}
 if(mode==='authored'){options.orientation='authored';options.frameAxis=vector([0,0,1]);options.frameNormal=vector([1,0,0])}
 if(mode==='guide')options.orientationGuide=bezierNurbsCurve([[1,0,0],[1,0,10]])
 const body=createProgressiveBrepProfileBody(loops,path,scale,twist,options)
 expect(body).toMatchObject({retainedCaps:{exact:true,scope:'constructor-owned-retained-endpoint-regions'},
  retainedWalls:{certified:true,faceCoverageCertified:true,coefficientFamilyCertified:true},
  capProjection:{idealCapDomainsCertified:true},boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
  boundaryErrorScope:'constructor-owned-retained-wall-and-cap-union',globalEmbeddingCertified:false})
 expect(body.boundaryErrorUpper).toBeGreaterThanOrEqual(0)
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(.01)
 expect(body.filledCapErrorUpper).toHaveLength(2)
 expect(body.approximation.report.endpointContourErrorUpper).toHaveLength(2)
})
it('does not promote an unproved original curved RMF frame to a body boundary bound',()=>{
 const curved=bezierNurbsCurve([[0,0,0],[0,0,5],[1,0,10]])
 const body=createProgressiveBrepProfileBody(loops,curved,scale,twist,{...base,maxDeviation:1})
 expect(body).toMatchObject({filledCapErrorUpper:null,boundaryErrorUpper:null,boundaryContinuousBound:false,boundaryErrorWithinBudget:null,globalEmbeddingCertified:false})
})
it.each(['plain','affine','authored','guide'])('bounds actual B-rep extraction of a small unsegmented rational profile: %s',mode=>{
 const extracted=structuredClone(loops)
 extracted[0]![0]={degree:2,knots:[0,0,0,.5,1,1,1],
  controlPoints:[[0,0,0],[.5,-.25,0],[1.5,-.25,0],[2,0,0]],weights:[1,.75,1.25,1],periodic:false}
 const options:ProgressiveGuidedSurfaceSweepOptions={...base}
 if(mode!=='plain'){options.axisScale=vector([2,3,1]);options.centerLaw=vector([0,0,0])}
 if(mode==='authored'){options.orientation='authored';options.frameAxis=vector([0,0,1]);options.frameNormal=vector([1,0,0])}
 if(mode==='guide')options.orientationGuide=bezierNurbsCurve([[1,0,0],[1,0,10]])
 const body=createProgressiveBrepProfileBody(extracted,path,scale,twist,options)
 expect(body).toMatchObject({bodyDecompositionProducts:54,retainedCaps:{exact:true},retainedWalls:{certified:true},
  boundaryContinuousBound:true,boundaryErrorWithinBudget:true,globalEmbeddingCertified:false})
 expect(body.bodyDecompositionErrorUpper).toBeGreaterThan(0)
 expect(body.boundaryErrorUpper).toBeGreaterThanOrEqual(body.bodyDecompositionErrorUpper!)
 expect(body.boundaryErrorUpper).toBeLessThan(.01)
})
it('includes the retained hole and all walls in the complete body boundary',()=>{
 const hole=[[.5,.5,0],[.5,1.5,0],[1.5,1.5,0],[1.5,.5,0]]
 const hollow=[...loops,hole.map((p,i)=>bezierNurbsCurve([p,hole[(i+1)%4]!]))]
 const body=createProgressiveBrepProfileBody(hollow,path,scale,twist,base)
 expect(body).toMatchObject({retainedCaps:{exact:true,inspectedEdges:16},retainedWalls:{certified:true},
  boundaryContinuousBound:true,boundaryErrorWithinBudget:true,globalEmbeddingCertified:false})
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(.01)
})
it.each(['arc-length-curved-nonaxial-planar-rmf-body-boundary.r','arc-length-curved-planar-rmf-body-boundary.r','arc-length-curved-contact-body-boundary.r','arc-length-curved-guided-body-boundary.r','arc-length-curved-frenet-body-boundary.r','arc-length-curved-fixed-normal-body-boundary.r','arc-length-curved-fixed-body-boundary.r','arc-length-curved-authored-body-boundary.r','progressive-hollow-boundary.r','progressive-unsegmented-hollow-boundary.r','planar-rmf-body-boundary.r','closed-planar-rmf-body-boundary.r'])('carries original-source body bounds through Rush into snapshot-local viewport metadata: %s',async file=>{
 const source=readFileSync(new URL(`../examples/rush/${file}`,import.meta.url),'utf8')
 const graph=compileRushFrontend(source).document
 const before=structuredClone(graph)
 const built=await buildOwnNurbsAsync(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw new Error('Missing retained body source')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:file==='arc-length-curved-nonaxial-planar-rmf-body-boundary.r'||file==='arc-length-curved-fixed-normal-body-boundary.r'||file==='arc-length-curved-frenet-body-boundary.r'||file==='arc-length-curved-guided-body-boundary.r'||file==='arc-length-curved-contact-body-boundary.r'||file==='arc-length-curved-planar-rmf-body-boundary.r'?2:file.startsWith('arc-length-curved-')?.2:.01})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toBeNull()
 const retained=JSON.parse(built.nativeGeometry.geometryJson).geometry
 if(file==='closed-planar-rmf-body-boundary.r') {
  expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry,retained))
   .not.toThrow()
  const forged={...built.nativeGeometry,geometryJson:JSON.stringify({geometry:retained,
   sweepBodyBoundaryEvidence:{globalEmbeddingCertified:true,solidGeometryCertified:true}})}
  expect(inspectProgressiveSweepSolidAdmission(forged,retained)).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true})
  const corrupted=structuredClone(retained)
  corrupted.faces[0].surface.controlPoints[0][0][0]+=.01
  expect(()=>inspectProgressiveSweepSolidAdmission(forged,corrupted)).toThrow(/snapshot binding/)
  const wrongCaps={...built.nativeGeometry,geometryJson:JSON.stringify({geometry:retained,
   sweepBodyBoundaryEvidence:{closedPath:false,globalEmbeddingCertified:true,solidGeometryCertified:true}})}
  expect(inspectProgressiveSweepSolidAdmission(wrongCaps,retained)).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true})
  expect(()=>inspectProgressiveSweepSolidAdmission(wrongCaps,corrupted)).toThrow(/snapshot binding/)
 } else {
  expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,retained))
   .toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true})
 }
 expect(graph).toEqual(before)
 const strict=source.replace(/max_deviation:\s*[0-9.]+mm/,'max_deviation: 0.000000000000000000000000000001mm')
 await expect(buildOwnNurbsAsync(compileRushFrontend(strict).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error|refinement.*budget/)
})


it.each(['fixed','rmf'] as const)('composes arc-length walls and filled caps through WASM: %s',orientation=>{
 const rationalPath=bezierNurbsCurve([[0,0,0],[0,0,10]],[1,4])
 const body=createProgressiveBrepProfileBody(loops,rationalPath,scale,twist,
  {...base,orientation,spacing:'arc_length',lengthTolerance:.001,lengthMaxCells:100000})
 expect(body).toMatchObject({retainedCaps:{exact:true},retainedWalls:{certified:true},
  boundaryContinuousBound:true,boundaryErrorWithinBudget:true,globalEmbeddingCertified:false})
 expect(body.approximation.report.continuousBound).toBe(true)
 expect(body.filledCapErrorUpper).toHaveLength(2)
 expect(body.boundaryErrorUpper).toBeLessThanOrEqual(.01)
},60000)

it.each(['fixed-normal','planar-rmf'])('refuses folded %s material despite a proved complete boundary error',async mode=>{
 const source=readFileSync(new URL(`../examples/rush/arc-length-curved-${mode}-folded-body-boundary.r`,import.meta.url),'utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing native body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:2})
 const retained=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry,retained)).toThrow(/Progressive sweep Solid geometry could not be proved/)
},60000)
