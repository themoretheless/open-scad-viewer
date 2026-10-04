import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/modelGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
const source=()=>readFileSync('examples/rush/progressive-miter-reconstructed-stations.r','utf8')
const request={action:'build' as const,display:{segments:4,subdivisionLevels:0}}
it('carries bounded curved station reconstruction through actual Rush, async construction and viewport evidence',async()=>{
 const graph=compileModelGraphText(source()).document
 const node=graph.nodes.find(n=>n.op==='brep_smooth_miter_stations')!
 expect(node).toMatchObject({wall_tolerance:.5,quantum:.125,max_work:10000,max_deviation:2})
 const built=buildOwnNurbs(graph,request)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({
  nodeId:node.id,continuousBound:true,boundaryErrorWithinBudget:true,
  solidGeometryCertified:true,profileG2Certified:true,stationG2Certified:true,
  stationContinuity:'G2',capContinuity:'C0',
 })
 const report=(built.report.construction![node.id] as any)
 expect(report.method).toBe('bounded-miter-station-reconstruction')
 expect(report.candidate.wallDisplacementUpper).toBeGreaterThan(.39)
 expect(report.sharpStationIndices).toEqual([])
 expect(report.boundaryErrorUpper).toBeGreaterThan(1)
 const asyncBuilt=await buildOwnNurbsAsync(graph,request)
 expect(asyncBuilt.nativeGeometry).toEqual(built.nativeGeometry)
 expect(asyncBuilt.report.construction).toEqual(built.report.construction)
})
it('refuses exhausted construction and complete-bound budgets through Rush',()=>{
 for(const [text,message]of [
  [source().replace('quantum: 0.125mm, max_work: 10000','quantum: 0.125mm, max_work: 1'),/work-limit/],
  [source().replace('max_deviation: 2mm','max_deviation: 1mm'),/complete boundary error/],
 ] as const)expect(()=>buildOwnNurbs(compileModelGraphText(text).document,request)).toThrow(message)
})
it('checks reconstruction dimensions in graph compilation before geometry runs',()=>{
 expect(()=>compileModelGraphText(source().replace('wall_tolerance: 0.5mm','wall_tolerance: 0.5deg'))).toThrow(/dimension|length|unit/i)
 expect(()=>compileModelGraphText(source().replace('quantum: 0.125mm, max_work: 10000','quantum: 0.125deg, max_work: 10000'))).toThrow(/dimension|length|unit/i)
})
it('keeps a sharp miter C0 through actual Rush and final viewport evidence',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/progressive-miter-reconstructed-sharp.r','utf8')).document
 const node=graph.nodes.find(n=>n.op==='brep_smooth_miter_stations')!
 const built=buildOwnNurbs(graph,request)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({solidGeometryCertified:true,continuousBound:true,stationContinuity:'C0',stationG1Certified:false,stationG2Certified:false})
 expect((built.report.construction![node.id] as any).sharpStationIndices).toEqual([1])
})
it('admits closed circle correction without inventing cap obligations in sync and async paths',async()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/closed-miter-frame-guide-affine-hollow-corrected.r','utf8')).document
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph,request)
 const report=built.report.construction![node.id] as any
 expect(report.boundaryCertificate).toMatchObject({closed:true,continuousBound:true,withinBudget:true,filledCapErrorUpper:null})
 expect(report.retainedCaps).toBeNull();expect(report.retainedCapDecomposition).toBeNull()
 expect(report.retainedWallCharts.allChartsCertified).toBe(true)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({solidGeometryCertified:true,continuousBound:true,capContinuity:'absent',stationContinuity:'C0'})
 const asyncBuilt=await buildOwnNurbsAsync(graph,request)
 expect(asyncBuilt.report.construction).toEqual(built.report.construction)
 expect(asyncBuilt.nativeGeometry).toEqual(built.nativeGeometry)
 expect(()=>buildOwnNurbs(compileModelGraphText(readFileSync('examples/rush/closed-miter-frame-guide-affine-hollow-corrected.r','utf8').replace('circle_correction_max_work:1000000','circle_correction_max_work:1')).document,request)).toThrow(/circle section correction unproved/)
},120000)
it('reconstructs a closed corrected frame/guide/affine hollow Solid with profile G2 and original C0 corners',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r','utf8')).document
 const node=graph.nodes.find(n=>n.op==='brep_smooth_miter_stations')!
 const built=buildOwnNurbs(graph,request)
 const report=built.report.construction![node.id] as any
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG2Certified:true,solidGeometryCertified:true,continuousBound:true,boundaryErrorWithinBudget:true,capContinuity:'absent',stationContinuity:'C0'})
 expect(report.boundaryCertificate).toMatchObject({closed:true,filledCapErrorUpper:null})
 expect(report.profileSmoothness.station.capEdges).toEqual([])
 expect(report.profileSmoothness.station.extractionComplete).toBe(true)
 expect(report.profileSmoothness.station.edgeIds).toHaveLength(128)
 const joins=report.profileSmoothness.station.g2.seams as {certified:boolean;reason:string}[]
 expect(joins.filter(join=>join.certified)).toHaveLength(96)
 expect(joins.filter(join=>!join.certified)).toHaveLength(32)
 expect(joins.filter(join=>!join.certified).every(join=>join.reason==='constant-projective-jet-relation-different')).toBe(true)
 expect(report.sharpStationIndices).toEqual([0,4,8,12])
 expect(report.candidate.wallDisplacementUpper).toBeGreaterThan(0)
 expect(report.boundaryErrorUpper).toBeGreaterThan(report.candidate.wallDisplacementUpper)
},120000)

it.each(['half','one','two'])('carries noncircular conic %s weights through Rush, G2 stations and Solid',async label=>{
 const text=readFileSync(`examples/rush/progressive-miter-reconstructed-conic-${label}.r`,'utf8')
 const graph=compileModelGraphText(text).document
 const node=graph.nodes.find(n=>n.op==='brep_smooth_miter_stations')!
 const built=buildOwnNurbs(graph,request)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({
  nodeId:node.id,continuousBound:true,boundaryErrorWithinBudget:true,
  solidGeometryCertified:true,profileG2Certified:true,stationG2Certified:true,
  stationContinuity:'G2',capContinuity:'C0',
 })
 const asyncBuilt=await buildOwnNurbsAsync(graph,request)
 expect(asyncBuilt.report.construction).toEqual(built.report.construction)
 expect(asyncBuilt.nativeGeometry).toEqual(built.nativeGeometry)
 expect(()=>buildOwnNurbs(compileModelGraphText(text.replace('quantum: 0.125mm, max_work: 10000','quantum: 0.125mm, max_work: 1')).document,request)).toThrow(/work-limit/)
},120000)

it('retains G2 of canonical circular profile joins through a closed authored frame, guide and affine Rush body',()=>{
 const source=readFileSync('examples/rush/closed-miter-frame-guide-affine-hollow.r','utf8')
 const graph=compileModelGraphText(source).document,before=structuredClone(graph)
 const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph,request)
 const report=built.report.construction![node.id] as any
 expect(report.profileSmoothness).toMatchObject({extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],profileG1Certified:true,
  profile:{exactG1G2Certified:true,certifiedOrder:2,unresolvedSeams:[]},fullBoundarySmoothnessCertified:false})
 expect(report.profileSmoothness.profile.seams.length).toBeGreaterThan(0)
 expect(report.profileSmoothness.totalExactWork).toBeLessThanOrEqual(report.profileSmoothness.maxWork)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({nodeId:node.id,profileG1Certified:true,profileG2Certified:true,
  stationContinuity:'C0',capContinuity:'absent'})
 expect(graph).toEqual(before)
})
