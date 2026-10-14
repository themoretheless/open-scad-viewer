import {it,expect} from 'vitest'
import {circleNurbsCurve,progressiveMiterNurbsProfiles,type NurbsVectorLaw} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveMiterBrepProfileBody} from '../src/services/geometry/brep'
const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const points:[number,number,number][]=[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]
const frameAxis:NurbsVectorLaw={degree:1,knots:[0,0,.25,.5,.75,1,1],values:[[1,-1,0],[1,1,0],[-1,1,0],[-1,-1,0],[1,-1,0]],weights:[1,1,1,1,1]}
const frameNormal:NurbsVectorLaw={degree:1,knots:[2,2,5,5],values:[[0,0,1],[0,0,1]],weights:[1,1]}
const guide={degree:1,knots:[0,0,.25,.5,.75,1,1],controlPoints:[...points,points[0]!].map(([x,y])=>[x,y,100]),weights:[1,1,1,1,1],periodic:false}
const loops=[[circleNurbsCurve([0,0,0],[1,0,0],.25)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.1))]]
const options={normal:[0,0,1] as [number,number,number],closed:true,maxDeviation:10,maxSteps:8,frameAxis,frameNormal,orientationGuide:guide,
 axisScale:{degree:1,knots:[7,7,9,9],values:[[2,1,1],[2,1,1]] as [number,number,number][],weights:[1,1]},retainedWallMaxInjectivityCells:10000}
it('qualifies the closed combined frame guide affine retained body without promoting miter corners',()=>{
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),options)
 expect(body.boundaryCertificate.continuousBound).toBe(true)
 expect(body.volume,JSON.stringify(body.volume)).toMatchObject({solidGeometryCertified:true,nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{shell:0,outward:true,expectedOutward:true},{shell:1,outward:false,expectedOutward:false}]})
 expect(body.approximation.report).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true})
})
it('refuses endpoint mismatches and insufficient proof budget for the same combined closed family',()=>{
 const axis=structuredClone(frameAxis);axis.values.at(-1)![0]=2
 expect(()=>progressiveMiterNurbsProfiles(loops.flat(),points,law(1),law(0),{...options,frameAxis:axis})).toThrow(/frame endpoints/)
 const rail=structuredClone(guide);rail.controlPoints.at(-1)![2]=101
 expect(()=>progressiveMiterNurbsProfiles(loops.flat(),points,law(1),law(0),{...options,orientationGuide:rail})).toThrow(/guide endpoints/)
 const tight={...options,maxDeviation:.1,maxSteps:16}
 const report=progressiveMiterNurbsProfiles(loops.flat(),points,law(1),law(0),tight).report
 expect(report).toMatchObject({accepted:false,frameTransportCertified:true,profileRegularityCertified:true,seamContinuity:'C0'})
 expect(report.certifiedErrorUpper).toBeGreaterThan(.1)
 expect(()=>createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),tight)).toThrow(/budget/)
})
it('preserves closed combined mode through Rush and actual Solid admission',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const graph=compileRushFrontend(readFileSync('examples/rush/closed-miter-frame-guide-affine-hollow.r','utf8'))
 const node=graph.document.nodes.find(node=>node.op==='brep_progressive_miter_sweep')!
 const built=await buildOwnNurbsAsync(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({closedPath:true,seamContinuity:'C0',continuousBound:true,volume:{solidGeometryCertified:true}})
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({solidGeometryCertified:true,continuousBound:true,boundaryErrorWithinBudget:true})
})
