import {expect,it} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'

it('transports exact rational station G2 through topology-owned BRep audit with one budget',()=>{
 const sections=[0,3,10,21,34].map(z=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)]])
 const model=createRationalBrepSectionLoft(sections),before=structuredClone(model)
 const caps=[model.faces.length-2,model.faces.length-1]
 const proof=inspectMiterProfileSmoothness(model,caps)
 expect(proof).toMatchObject({extractionComplete:true,profileG1Certified:true,stationContinuity:'G2',capContinuity:'C0',fullBoundarySmoothnessCertified:false})
 expect(proof.station).toMatchObject({extractionComplete:true,stationG1Certified:true,stationG2Certified:true,unpairedEdges:[]})
 expect(proof.station.edgeIds).toHaveLength(12)
 expect(proof.totalExactWork).toBe(proof.exactWork+proof.station.exactWork)
 expect(proof.totalExactWork).toBeLessThanOrEqual(2000000)
 const short=inspectMiterProfileSmoothness(model,caps,proof.totalExactWork-1)
 expect(short.station.stationG2Certified).toBe(false)
 expect(short.totalExactWork).toBeLessThanOrEqual(proof.totalExactWork-1)
 expect(inspectMiterProfileSmoothness(model,caps,0)).toMatchObject({profileG1Certified:false,stationContinuity:'C0',totalExactWork:0})
 expect(()=>inspectMiterProfileSmoothness(model,[caps[0]!,caps[0]!])).toThrow()
 expect(model).toEqual(before)
})

it('preserves exact nominal profile G2 on the corrected spatial Rush fixture',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const graph=compileRushFrontend(readFileSync('examples/rush/progressive-miter-spatial-circle-corrected-hollow.r','utf8'))
 const built=await buildOwnNurbsAsync(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG2Certified:true,solidGeometryCertified:true,continuousBound:true,boundaryErrorWithinBudget:true})
})

it('preserves nonbinary retained station G2 through Rush, viewport evidence and Solid',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const graph=compileRushFrontend(readFileSync('examples/rush/nonuniform-station-g2-hollow-miter.r','utf8'))
 const node=graph.document.nodes.find(node=>node.op==='brep_progressive_miter_sweep')!
 expect(node).toBeDefined()
 const built=await buildOwnNurbsAsync(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction?.[node.id]).toMatchObject({continuousBound:true,volume:{solidGeometryCertified:true}})
 const model=JSON.parse(built.nativeGeometry!.geometryJson).geometry
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)).toMatchObject({solidGeometryCertified:true})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({solidGeometryCertified:true,continuousBound:true,boundaryErrorWithinBudget:true,stationContinuity:'G2',stationG2Certified:true})
 expect(readSweepViewportEvidence(built.nativeGeometry)?.stationSeamCount).toBeGreaterThanOrEqual(24)
})
