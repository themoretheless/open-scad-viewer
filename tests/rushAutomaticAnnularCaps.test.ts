import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

// Exercise the ordinary authored Rush command against the real WASM kernel.
// There is no annular option or caller-supplied certificate in the source.
const source=readFileSync('examples/rush/automatic-annular-rmf-body.r','utf8')

it.each(['sync','async'] as const)('selects native ring caps through ordinary Rush (%s) and admits the actual body',async mode=>{
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const options={action:'build' as const,display:{segments:2,subdivisionLevels:0}}
 const built=mode==='sync'?buildOwnNurbs(document,options):await buildOwnNurbsAsync(document,options)
 expect(built.nativeGeometry).toBeDefined()
 const data=readNativeGeometryData(built.nativeGeometry!),evidence=data.sweepBodyBoundaryEvidence
 expect(evidence.capRepresentation).toBe('radial-annular')
 expect(evidence.planarCapFaces).toEqual([])
 expect(evidence.capFaceGroups.map((g:number[])=>g.length)).toEqual([4,4])
 expect(evidence.capFaces).toEqual(evidence.capFaceGroups.flat())
 expect(evidence.capSelectionExactWork).toBeGreaterThan(0)
 const filling=evidence.retainedAnnularFilling
 expect(filling).toMatchObject({sourceFillingCovered:true,withinBudget:true})
 expect(filling.exactWork).toBe(filling.selectionExactWork+filling.constructionExactWork+filling.fillingExactWork)
 expect(filling.exactWork).toBeLessThanOrEqual(1000000)
 expect(evidence.retainedSmoothness.capInternalSeams).toHaveLength(2)
 expect(evidence.retainedSmoothness.geometricMaxCells).toBe(128)
 expect(evidence.retainedSmoothness.geometricCells).toBeLessThanOrEqual(128)
 for(const cap of evidence.retainedSmoothness.capInternalSeams){
  expect(cap.geometricG2).toMatchObject({complete:true,withinTolerance:true,positionTolerance:0,normalTolerance:1e-7,curvatureTolerance:1e-5,exactIdentityClaim:false,solidGeometryCertified:false})
  expect(cap.geometricG2.seams).toHaveLength(cap.edgeIds.length)
 }
 expect(evidence.retainedSmoothness.totalExactWork).toBeLessThanOrEqual(2000000)
 expect(evidence.retainedSmoothness).toMatchObject({capWallContinuity:'C0',fullBoundarySmoothnessCertified:false})
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,data.geometry)
 expect(volume).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume!.individualPairs+volume!.groupedPairs).toBe(data.geometry.faces.length*(data.geometry.faces.length-1)/2)
 expect(JSON.stringify(document)).toBe(before)
},120000)

it('reports absent endpoint caps for a closed Rush sweep',async()=>{
 const closed=`// @rush/1
outer = circle_curve(center: [2mm,0,0],normal: [0,1,0],radius: 0.5mm)
hole = circle_curve(center: [2mm,0,0],normal: [0,-1,0],radius: 0.25mm)
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 2mm)
show brep_progressive_sweep([[outer],[hole]],path,
 orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 9,max_sections: 9,max_deviation: 2mm
).brep_tessellate(2)`
 const built=await buildOwnNurbsAsync(compileRushFrontend(closed).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 const data=readNativeGeometryData(built.nativeGeometry!),evidence=data.sweepBodyBoundaryEvidence
 expect(evidence).toMatchObject({capRepresentation:'closed',closedPath:true,capFaces:[],capFaceGroups:[[],[]],planarCapFaces:[],capSelectionExactWork:0})
 expect(evidence.retainedSmoothness.capWallContinuity).toBe('absent')
 expect(evidence.retainedAnnularFilling).toBeNull()
},120000)

it('keeps exact straight ring caps in their legacy representation through Rush',async()=>{
 const straight=source.replace('normal: [3,4,0]','normal: [0,0,1]')
  .replace('normal: [-3,-4,0]','normal: [0,0,-1]')
  .replace('end: [3mm,4mm,0]','end: [0,0,10mm]')
  .replace('normal: [0,0,1],\n scale:','normal: [1,0,0],\n scale:')
  .replace('values: [1,2]','values: [1,1]')
 const built=await buildOwnNurbsAsync(compileRushFrontend(straight).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 const data=readNativeGeometryData(built.nativeGeometry!),evidence=data.sweepBodyBoundaryEvidence
 expect(evidence.capRepresentation).toBe('legacy')
 expect(evidence.capFaceGroups.map((g:number[])=>g.length)).toEqual([1,1])
 expect(evidence.planarCapFaces).toEqual(evidence.capFaces)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,data.geometry)).toMatchObject({solidGeometryCertified:true,allPairsClassified:true,nextPair:null})
},120000)
