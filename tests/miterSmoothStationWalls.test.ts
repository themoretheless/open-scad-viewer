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
 expect(()=>smoothCertifiedMiterBody(source,sections,[],{...options,maxDeviation:.8})).toThrow(/complete boundary error/)
 const result=smoothCertifiedMiterBody(source,sections,[],options)
 expect(result.boundaryCertificate.errorUpper).toBeGreaterThan(.8)
 expect(result.boundaryCertificate.errorUpper).toBeLessThanOrEqual(1)
 expect(result.candidate.wallDisplacementUpper).toBeGreaterThan(.39)
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.volume.solidGeometryCertified).toBe(true)
 expect(result.profileSmoothness.station.stationG2Certified).toBe(true)
 const imported=importDirectStepV5(exportDirectStepV5(result.model).text).model
 expect(inspectSweepVolume(imported,[imported.faces.length-2,imported.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
 expect(inspectMiterStationSmoothness(imported,[imported.faces.length-2,imported.faces.length-1]).stationG2Certified).toBe(true)
})

it('certifies periodic polynomial profiles after owned curved-center station reconstruction',()=>{
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const profile:NurbsCurve={degree:2,knots:[0,1,2,3,4,5,6,7,8],
  controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],
  weights:[1,1,1,1,1,1],periodic:true}
 const source=createProgressiveMiterBrepProfileBody([[profile]],[[0,0,0],[0,0,10]],law(1),law(0),{
  normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:1,
  centerLaw:{degree:1,knots:[0,0,.5,1,1],values:[[0,0,0],[1,0,0],[0,0,0]],weights:[1,1,1]},
 })
 const before=structuredClone(source.model)
 const options={quantum:.125,maxWork:10000,wallTolerance:.5,maxDeviation:2}
 const result=reconstructCertifiedMiterStations(source,options)
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.volume.solidGeometryCertified).toBe(true)
 expect(result.profileSmoothness.profile).toMatchObject({exactG1G2Certified:true,certifiedOrder:2})
 expect(result.profileSmoothness.station.stationG2Certified).toBe(true)
 expect(source.model).toEqual(before)
 expect(()=>reconstructCertifiedMiterStations(source,{...options,maxWork:0})).toThrow()
 const caps=[result.model.faces.length-2,result.model.faces.length-1]
 for(const face of caps)expect(result.model.faces[face]).toEqual(source.model.faces[face])
 const imported=importDirectStepV5(exportDirectStepV5(result.model).text).model
 expect(inspectMiterStationSmoothness(imported,caps).stationG2Certified).toBe(true)
 expect(inspectSweepVolume(imported,caps,DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
})

it.each(['hollow polygon','nonuniform periodic rational'] as const)('qualifies owned general reconstruction of %s',mode=>{
 const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const ring=(r:number,reverse:boolean):NurbsCurve[]=>{
  const p=[[-r,-r,0],[r,-r,0],[r,r,0],[-r,r,0],[-r,-r,0]]
  if(reverse)p.reverse()
  return p.slice(1).map((b,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[p[i]!,b],weights:[1,1],periodic:false}))
 }
 const rational:NurbsCurve={degree:2,knots:[0,1,2,3,4,5,6,7,8],
  controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],
  weights:[1,.5,1,1,1,.5],periodic:true}
 const hollow=mode==='hollow polygon'
 const source=createProgressiveMiterBrepProfileBody(hollow?[ring(2,false),ring(.5,true)]:[[rational]],
  [[0,0,0],[0,0,10]],law(1),law(0),{normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:1,
   centerLaw:{degree:1,knots:[0,0,.5,1,1],values:[[0,0,0],[hollow?.5:0,0,0],[0,0,0]],weights:[1,1,1]}})
 const before=structuredClone(source.model)
 const options={quantum:.125,maxWork:100000,wallTolerance:.5,maxDeviation:2}
 const result=reconstructCertifiedMiterStations(source,options)
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.volume.solidGeometryCertified).toBe(true)
 expect(result.profileSmoothness.station.stationG2Certified).toBe(true)
 if(hollow)expect(result.profileSmoothness.profileG1Certified).toBe(false)
 else expect(result.profileSmoothness.profileG1Certified).toBe(true)
 expect(source.model).toEqual(before)
 expect(()=>reconstructCertifiedMiterStations(source,{...options,maxWork:0})).toThrow()
 const caps=[result.model.faces.length-2,result.model.faces.length-1]
 for(const face of caps){
  expect(result.model.faces[face]).toEqual(source.model.faces[face])
  expect(result.model.faces[face]!.holes).toHaveLength(hollow?1:0)
 }
 const imported=importDirectStepV5(exportDirectStepV5(result.model).text).model
 expect(inspectMiterStationSmoothness(imported,caps).stationG2Certified).toBe(true)
 expect(inspectSweepVolume(imported,caps,DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
})

it('carries nonuniform rational station certificates through the Rush geometry graph',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const source=readFileSync('examples/rush/progressive-miter-reconstructed-rational-profile.r','utf8')
 const built=buildOwnNurbs(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const construction=built.report.construction![built.nativeGeometry!.nodeId] as {
  continuousBound:boolean;boundaryCertificate:{withinBudget:boolean};
  profileSmoothness:{profileG1Certified:boolean;station:{stationG2Certified:boolean}}
 }
 expect(construction.continuousBound).toBe(true)
 expect(construction.boundaryCertificate.withinBudget).toBe(true)
 expect(construction.profileSmoothness.station.stationG2Certified).toBe(true)
 expect(construction.profileSmoothness.profileG1Certified).toBe(true)
 expect(built.nativeGeometry!.geometryJson).toContain('sweepMiterReplay')
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({
  profileG1Certified:true,stationG2Certified:true,solidGeometryCertified:true,continuousBound:true,capContinuity:'C0',
 })
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const {kind,...model}=built.report.definitions[built.nativeGeometry!.nodeId] as unknown as import('../src/services/geometry/brep').NurbsBrep & {kind:string}
 expect(kind).toBe('brep')
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)?.solidGeometryCertified).toBe(true)
 const tampered=structuredClone(model)
 tampered.faces[0]!.surface.controlPoints[0]![0]![0]!+=.125
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,tampered)).toThrow(/final model differs from original replay/)
})


it('admits the closed rational hollow frame/guide/affine case through native Rush replay',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/closed-rational-frame-guide-affine-hollow-placed.r','utf8')
 const built=buildOwnNurbs(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG1Certified:true,solidGeometryCertified:true,continuousBound:true})
 const {kind,...model}=built.report.definitions[built.nativeGeometry!.nodeId] as unknown as import('../src/services/geometry/brep').NurbsBrep & {kind:string}
 expect(kind).toBe('brep')
 expect(model.shells).toHaveLength(2)
 expect(model.shells.every(shell=>shell.closed)).toBe(true)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)?.solidGeometryCertified).toBe(true)
 const tampered=structuredClone(model)
 tampered.faces[0]!.surface.controlPoints[0]![0]![0]!+=.125
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,tampered)).toThrow(/final model differs from original replay/)
})
