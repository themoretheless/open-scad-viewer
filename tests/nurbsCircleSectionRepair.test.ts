import {expect,it} from 'vitest'
import {circleNurbsCurve,type NurbsScaleLaw} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {repairNurbsCircleSweepSections} from '../src/services/nurbsCircleSectionRepair'
import {createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody} from '../src/services/geometry/brep'
const law=(a:number,b:number):NurbsScaleLaw=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
it('shares the correction budget across stations and never publishes partial results',()=>{
 const curve=circleNurbsCurve([0,0,0],[0,0,1],.5)
 const stations=[[curve],[structuredClone(curve)]]
 expect(repairNurbsCircleSweepSections(stations,{quantum:2**-40,tolerance:1e-10,maxWork:1000})).toMatchObject({reason:'bounded-circle-section-interpolation',work:72})
 expect(repairNurbsCircleSweepSections(stations,{quantum:2**-40,tolerance:1e-10,maxWork:71})).toMatchObject({sections:null,wallDisplacementUpper:null,work:71,reason:'work-limit'})
 expect(stations[0]![0]).toEqual(curve)
})
it('rebuilds scale/twist walls and caps with exact profile G2 and a composed boundary bound',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const body=createProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],law(1,1.5),law(0,30),{normal:[1,0,0],maxDeviation:.02,maxSteps:16,retainedWallMaxInjectivityCells:100000,circleCorrection:{quantum:2**-40,tolerance:1e-10,maxWork:100000}})
 expect(body.profileSmoothness).toMatchObject({profileG1Certified:true,profile:{exactG1G2Certified:true,certifiedOrder:2}})
 expect(body.sectionCorrection?.wallDisplacementUpper).toBeGreaterThan(0)
 expect(body.sectionCorrection?.wallDisplacementUpper).toBeLessThan(1e-10)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(body.volume.solidGeometryCertified).toBe(true)
 expect(body.retainedWallCharts.allChartsCertified).toBe(true)
 const stream=streamProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],law(1,1.5),law(0,30),{normal:[1,0,0],maxDeviation:.02,maxSteps:16,retainedWallMaxInjectivityCells:100000,circleCorrection:{quantum:2**-40,tolerance:1e-10,maxWork:100000}})
 let next=await stream.next();while(!next.done)next=await stream.next()
 expect(next.value.sectionCorrection).toEqual(body.sectionCorrection)
 expect(next.value.model).toEqual(body.model)
 expect(next.value.profileSmoothness).toEqual(body.profileSmoothness)
 expect(next.value.boundaryCertificate).toEqual(body.boundaryCertificate)
})
it('carries explicit circle correction through Rush units, viewport evidence and Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/progressive-miter-circle-corrected-hollow.r','utf8')
 const graph=compileRushFrontend(source)
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 expect(node).toMatchObject({circle_correction_tolerance:1e-10,circle_correction_max_work:100000,retained_wall_max_injectivity_cells:100000})
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG1Certified:true,profileG2Certified:true,continuousBound:true,solidGeometryCertified:true,boundaryErrorWithinBudget:true})
 expect(()=>compileRushFrontend(source.replace('0.0000000001mm','0.0000000001deg'))).toThrow()
 expect(()=>buildOwnNurbs(compileRushFrontend(source.replace('circle_correction_max_work: 100000','circle_correction_max_work: 1')).document,{action:'build'})).toThrow(/circle section correction unproved/)
})
it('composes circle repair before spatial cap projection and revalidates the final body',async()=>{
 const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
 const points:[number,number,number][]=[[0,0,0],[0,0,10],[10,0,10],[10,10,15]]
 const options={normal:[1,0,0] as [number,number,number],miterLimit:2,maxDeviation:.01,maxSteps:1,retainedWallMaxInjectivityCells:100000,circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}}
 expect(()=>createProgressiveMiterBrepProfileBody(loops,points,law(1,1),law(0,0),options)).toThrow(/filled retained cap regions unproved/)
 const body=createProgressiveMiterBrepProfileBody(loops,points,law(1,1),law(0,0),{...options,capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}})
 expect(body.profileSmoothness.profile).toMatchObject({exactG1G2Certified:true,certifiedOrder:2})
 expect(body.retainedCaps).toMatchObject({exact:true,reason:'exact-planar-regions'})
 expect(body.sectionCorrection?.exactPlanarSections).toEqual([0,3])
 const stream=streamProgressiveMiterBrepProfileBody(loops,points,law(1,1),law(0,0),{...options,capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}})
 let next=await stream.next();while(!next.done)next=await stream.next()
 expect(next.value.model).toEqual(body.model)
 expect(next.value.sectionCorrection).toEqual(body.sectionCorrection)
 expect(next.value.boundaryCertificate).toEqual(body.boundaryCertificate)
 expect(next.value.profileSmoothness).toEqual(body.profileSmoothness)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(body.boundaryCertificate.errorUpper).toBeGreaterThanOrEqual(body.sectionCorrection!.wallDisplacementUpper!)
 expect(body.volume.solidGeometryCertified).toBe(true)
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const graph=compileRushFrontend(readFileSync('examples/rush/progressive-miter-spatial-circle-corrected-hollow.r','utf8'))
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG1Certified:true,profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
})
it('revalidates authored affine laws after circle correction through Rush',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const graph=compileRushFrontend(readFileSync('examples/rush/progressive-miter-affine-circle-corrected-hollow.r','utf8'))
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect((built.report.construction![node.id] as any).affineLawsApplied).toBe(true)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG1Certified:true,profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
})
