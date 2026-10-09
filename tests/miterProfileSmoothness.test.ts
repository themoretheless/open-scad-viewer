import {expect,it} from 'vitest'
import {circleNurbsCurve,type NurbsScaleLaw} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody} from '../src/services/geometry/brep'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
const law=(value:number):NurbsScaleLaw=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
const points:[number,number,number][]=[[0,0,0],[0,0,10]]
const options={normal:[1,0,0] as [number,number,number],maxDeviation:.001,maxSteps:1}
it('reports exact profile G2 with separate C0 stations and caps in synchronous and streamed bodies',async()=>{
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),options)
 expect(body.profileSmoothness,JSON.stringify(body.profileSmoothness)).toMatchObject({extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],scope:'wall-profile-seams',profileG1Certified:true,g1Method:'implied-by-G2',g1Audit:null,profile:{exactG1G2Certified:true,certifiedOrder:2},stationContinuity:'C0',capContinuity:'C0',fullBoundarySmoothnessCertified:false})
 expect(body.profileSmoothness.edgeIds).toHaveLength(8)
 const stream=streamProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),options)
 let next=await stream.next();while(!next.done)next=await stream.next()
 expect(next.value.profileSmoothness).toEqual(body.profileSmoothness)
 const exhausted=inspectMiterProfileSmoothness(body.model,[8,9],body.profileSmoothness.profile.exactWork-1)
 expect(exhausted.profile).toMatchObject({exactG1G2Certified:false,certifiedOrder:null})
})
it('does not hide a missing natural boundary or unpaired profile edge',()=>{
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),options)
 const model=structuredClone(body.model)
 const wire=model.loops[model.faces[0]!.outer]!
 const coedge=wire.coedges.find(c=>c.pcurve.controlPoints[0]![0]===c.pcurve.controlPoints[1]![0])!
 coedge.pcurve.controlPoints[1]![1]=.5
 const before=structuredClone(model)
 const report=inspectMiterProfileSmoothness(model,[8,9])
 expect(report.extractionComplete).toBe(false)
 expect(report.profile).toMatchObject({exactG1G2Certified:false,certifiedOrder:null})
 expect(report.unclassifiedFaces).toContain(0)
 expect(report.unpairedEdges.length).toBeGreaterThan(0)
 expect(model).toEqual(before)
})
it('keeps a complete G1 profile claim when exact G2 fails, within one shared work budget',()=>{
 const base=[[1,0,0],[1,1,0],[.5,1,0],[1,1,0],[0,1,0]]
 const profiles=[0,1,2,3].map(turn=>({degree:4,knots:[0,0,0,0,0,1,1,1,1,1],
  controlPoints:base.map(([a,b,z])=>{let x=a!,y=b!;for(let i=0;i<turn;i++)[x,y]=[-y,x];return [x,y,z!]}),weights:[1,1,1,1,1],periodic:false}))
 const body=createProgressiveMiterBrepProfileBody([profiles],points,law(1),law(0),options)
 expect(body.profileSmoothness).toMatchObject({extractionComplete:true,profile:{exactG1G2Certified:false,certifiedOrder:null},profileG1Certified:true,g1Method:'exact-projective-audit',g1Audit:{exactG1G2Certified:true,certifiedOrder:1},stationContinuity:'C0',fullBoundarySmoothnessCertified:false})
 expect(body.profileSmoothness.exactWork).toBe(body.profileSmoothness.profile.exactWork+body.profileSmoothness.g1Audit!.exactWork)
 expect(body.profileSmoothness.exactWork).toBeLessThanOrEqual(1000000)
 const exhausted=inspectMiterProfileSmoothness(body.model,[body.model.faces.length-2,body.model.faces.length-1],body.profileSmoothness.profile.exactWork)
 expect(exhausted).toMatchObject({profileG1Certified:false,g1Method:'unproved',g1Audit:{exactWork:0}})
})
it('carries independent G1 and unproved G2 through joint-mode Rush, viewport evidence and Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const graph=compileRushFrontend(readFileSync('examples/rush/miter-g1-profile-frame-guide-affine.r','utf8'))
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(built.report.construction?.[node.id]).toMatchObject({authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true,profileSmoothness:{profileG1Certified:true,g1Method:'exact-projective-audit',profile:{exactG1G2Certified:false}},volume:{solidGeometryCertified:true}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG1Certified:true,profileG2Certified:false,continuousBound:true,solidGeometryCertified:true})
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
})

it('certifies every closed joint-mode profile seam within the declared aggregate budget',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const graph=compileRushFrontend(readFileSync('examples/rush/closed-miter-frame-guide-affine-hollow.r','utf8'))
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const report=built.report.construction![node.id] as any
 expect(report.profileSmoothness).toMatchObject({maxWork:2000000,profile:{certifiedOrder:2,exactG1G2Certified:true},profileG1Certified:true,capContinuity:'absent',stationContinuity:'C0'})
 expect(report.profileSmoothness.edgeIds).toHaveLength(128)
 expect(report.profileSmoothness.exactWork).toBeGreaterThan(0)
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry
 const cutoff=inspectMiterProfileSmoothness(model,[],report.profileSmoothness.exactWork-1)
 expect(cutoff.profile.exactG1G2Certified).toBe(false)
 expect(report.profileSmoothness.exactWork).toBeLessThanOrEqual(2000000)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG1Certified:true,profileG2Certified:true,solidGeometryCertified:true})
})

it('refuses periodic and malformed pcurves before certifying profile joins',()=>{
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1),law(0),options)
 for(const periodic of [true,false]){
  const model=structuredClone(body.model),face=model.faces[0]!
  const use=model.loops[face.outer]!.coedges.find(c=>c.pcurve.controlPoints[0]![0]===c.pcurve.controlPoints[1]![0])!
  if(periodic)use.pcurve.periodic=true
  else use.pcurve.knots=[0,1,0,1]
  expect(inspectMiterProfileSmoothness(model,[model.faces.length-2,model.faces.length-1])).toMatchObject({extractionComplete:false,profileG1Certified:false,profile:{exactG1G2Certified:false}})
 }
})
