import {exportDirectStepV5,importDirectStepV5} from '../src/services/cadNurbsStep'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reconstructCertifiedMiterStations} from '../src/services/geometry/brep'
import {it,expect} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {createBrepSectionLoftSurfaces,createRationalBrepSectionLoft,inspectNurbsBrep,proposeSmoothStationWalls,createProgressiveMiterBrepProfileBody,smoothCertifiedMiterBody} from '../src/services/geometry/brep'
import {inspectMiterStationSmoothness} from '../src/services/miterStationSmoothness'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {inspectSweepRetainedWallCharts} from '../src/services/nurbsSweepRetainedCharts'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
const signs=[[1,0],[1,1],[0,1],[-1,1],[-1,0],[-1,-1],[0,-1],[1,-1],[1,0]]
const section=(x:number,z:number):NurbsCurve[][]=>[[{
 degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],
 controlPoints:signs.map(([a,b])=>[x+a!,b!,z]),
 weights:signs.map((_,i)=>i%2===0?1:Math.SQRT1_2),periodic:false,
}]]
it('reconstructs privately owned final corrected sections rather than exposed source sections',()=>{
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const source=createProgressiveMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.2)]],[[0,0,0],[0,0,10]],law(1),law(0),{
  normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:.1,
  circleCorrection:{quantum:2**-8,tolerance:.01,maxWork:10000},
 })
 const options={quantum:2**-8,maxWork:10000,wallTolerance:.01,maxDeviation:.1}
 // Approximation samples are public diagnostics, not the retained corrected body.
 source.approximation.sections[1]![0]!.controlPoints[0]![0]!+=100
 const result=reconstructCertifiedMiterStations(source,options)
 expect(result.volume.solidGeometryCertified).toBe(true)
 expect(result.profileSmoothness.station.stationG2Certified).toBe(true)
 expect(()=>reconstructCertifiedMiterStations(structuredClone(source),options)).toThrow(/constructor-owned/)
})
it('retains original sharp polyline stations as C0 during owned reconstruction',()=>{
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const source=createProgressiveMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],1)]],[[0,0,0],[0,0,10],[10,0,10]],law(1),law(0),{
  normal:[1,0,0],initialSteps:1,maxSteps:1,maxDeviation:.1,
  circleCorrection:{quantum:.125,tolerance:1e-9,maxWork:10000},
 })
 const result=reconstructCertifiedMiterStations(source,{quantum:1/32,maxWork:10000,wallTolerance:.1,maxDeviation:.5})
 expect(result.sharpStationIndices).toEqual([1])
 expect(result.profileSmoothness.stationContinuity).toBe('C0')
 expect(result.profileSmoothness.station.stationG1Certified).toBe(false)
 expect(result.volume.solidGeometryCertified).toBe(true)
})
it('retains bounded quintic walls and certifies actual soft station jets through WASM',()=>{
 const sections=[section(0,0),section(1,5),section(0,10)]
 const r=proposeSmoothStationWalls(sections,[],false,.125,10,10000)
 expect(r.reason).toBe('bounded-shared-quintic-station-jets')
 expect(r.sides![0]).toHaveLength(8)
 const model=createBrepSectionLoftSurfaces(sections,r.sides!,false)
 expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 expect(inspectMiterStationSmoothness(model,[model.faces.length-2,model.faces.length-1])).toMatchObject({
  extractionComplete:true,stationG1Certified:true,stationG2Certified:true,
 })
 const caps=[model.faces.length-2,model.faces.length-1]
 const charts=inspectSweepRetainedWallCharts(model,caps,100000)
 const volume=inspectSweepVolume(model,caps,DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(charts.allChartsCertified).toBe(true)
 // Material validity requires the synchronized shared-boundary proof too.
 expect(volume.solidGeometryCertified).toBe(true)
 expect(volume.allPairsClassified).toBe(true)
 const base=createRationalBrepSectionLoft(sections)
 for(let face=0;face<8;face++)for(let ui=0;ui<=8;ui++)for(let vi=0;vi<=8;vi++){
  const a=evaluateNurbsSurface(model.faces[face]!.surface,ui/8,vi/8).point
  const b=evaluateNurbsSurface(base.faces[face]!.surface,ui/8,vi/8).point
  expect(Math.hypot(...a.map((x,k)=>x-b[k]!))).toBeLessThanOrEqual(r.wallDisplacementUpper!+1e-12)
 }
 expect(proposeSmoothStationWalls(sections,[],false,.125,0,10000)).toMatchObject({sides:null,reason:'displacement-budget'})
 expect(proposeSmoothStationWalls(sections,[],false,.125,10,82)).toMatchObject({sides:null,work:82,reason:'work-limit'})
 const sharp=proposeSmoothStationWalls(sections,[1],false,.125,10,10000)
 const sharpModel=createBrepSectionLoftSurfaces(sections,sharp.sides!,false)
 expect(inspectMiterStationSmoothness(sharpModel,[sharpModel.faces.length-2,sharpModel.faces.length-1])).toMatchObject({stationG1Certified:false,stationG2Certified:false})
})
it('requires intact full-bound ownership and exact reproduction of the source stations',()=>{
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const source=createProgressiveMiterBrepProfileBody(section(0,0),[[0,0,0],[0,0,10]],law(1),law(0),{
  normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:.01,
 })
 const sections=source.approximation.sections.map(station=>station.map(curve=>[curve]))
 const options={quantum:.125,maxWork:10000,wallTolerance:.01,maxDeviation:.01}
 expect(()=>smoothCertifiedMiterBody(structuredClone(source),sections,[],options)).toThrow(/constructor-owned/)
 const wrong=structuredClone(sections);for(const pole of wrong[1]![0]![0]!.controlPoints)pole[0]!+=.125
 expect(()=>smoothCertifiedMiterBody(source,wrong,[],options)).toThrow(/reproduce|correspondence|closed/i)
 const result=smoothCertifiedMiterBody(source,sections,[],options)
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.volume.solidGeometryCertified).toBe(true)
 expect(result.profileSmoothness.station.stationG2Certified).toBe(true)
 const imported=importDirectStepV5(exportDirectStepV5(result.model).text).model
 expect(inspectSweepVolume(imported,[imported.faces.length-2,imported.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
 expect(inspectMiterStationSmoothness(imported,[imported.faces.length-2,imported.faces.length-1]).stationG2Certified).toBe(true)
})

it('composes the full bound and proves Solid after genuinely curved owned-source reconstruction',()=>{
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const source=createProgressiveMiterBrepProfileBody(section(0,0),[[0,0,0],[0,0,10]],law(1),law(0),{
  normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:1,
  centerLaw:{degree:1,knots:[0,0,.5,1,1],values:[[0,0,0],[1,0,0],[0,0,0]],weights:[1,1,1]},
 })
 const sections=source.approximation.sections.map(station=>station.map(curve=>[curve]))
 const options={quantum:.125,maxWork:10000,wallTolerance:.5,maxDeviation:2}
 expect(()=>smoothCertifiedMiterBody(source,sections,[],{...options,maxDeviation:1})).toThrow(/complete boundary error/)
 const result=smoothCertifiedMiterBody(source,sections,[],options)
 expect(result.candidate.wallDisplacementUpper).toBeGreaterThan(.39)
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.volume.solidGeometryCertified).toBe(true)
 expect(result.profileSmoothness.station.stationG2Certified).toBe(true)
 const imported=importDirectStepV5(exportDirectStepV5(result.model).text).model
 expect(inspectSweepVolume(imported,[imported.faces.length-2,imported.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
 expect(inspectMiterStationSmoothness(imported,[imported.faces.length-2,imported.faces.length-1]).stationG2Certified).toBe(true)
})
