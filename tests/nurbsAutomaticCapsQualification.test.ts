import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import type {NurbsBrep} from '../src/services/geometry/brep'
// Runs only in the frozen stage with its matching Auto-cap WASM. No mocks,
// authored plane correction, substituted laws, or widened limits.
const cases=[['oblique-rmf',16],['curved-rmf',72],['curved-frenet',72]] as const
it.each(cases)('original %s hollow reaches native bound, viewport and fresh Solid',async(stem,faces)=>{
 const text=readFileSync(new URL(`../examples/rush/${stem}-circle-hollow-body-boundary.r`,import.meta.url),'utf8')
 expect(text).not.toMatch(/cap_correction_|circle_correction_/)
 const graph=compileRushFrontend(text).document,before=structuredClone(graph)
 const built=await buildOwnNurbsAsync(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw new Error('Missing original source body snapshot')
 const data=readNativeGeometryData(built.nativeGeometry),report=data.sweepBodyBoundaryEvidence
 const model=data.geometry as NurbsBrep
 expect(report).toMatchObject({capRepresentation:'radial-annular',boundaryContinuousBound:true,
  boundaryErrorWithinBudget:true,budget:.01,globalEmbeddingCertified:false,
  retainedAnnularFilling:{sourceFillingCovered:true,sourceUvFacesInjective:true,sourceUvPairsClassified:true,solidGeometryCertified:false}})
 expect(report.capSelectionExactWork).toBeGreaterThan(0)
 expect(report.retainedAnnularFilling.exactWork).toBe(report.capSelectionExactWork+
  report.retainedAnnularFilling.constructionExactWork+report.retainedAnnularFilling.fillingExactWork)
 expect(report.retainedAnnularFilling.exactWork).toBeLessThanOrEqual(1000000)
 expect(report.capFaceGroups.flat()).toEqual(report.capFaces)
 expect(report.capFaces).toHaveLength(8)
 expect(data.sweepPlanarCapFaces).toEqual([])
 expect(model.faces).toHaveLength(faces)
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:.01})
 const actual=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)
 expect(actual).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allPairsClassified:true,nextPair:null})
 expect(actual!.boundary!.totalPairs).toBe(faces*(faces-1)/2)
 expect(actual!.boundary!.individualPairs+actual!.boundary!.groupedPairs).toBe(actual!.boundary!.totalPairs)
 expect(graph).toEqual(before)
 const broken=structuredClone(model)
 broken.faces[report.capFaces[0]].surface.controlPoints[0]![0]![0]!+=.0001
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry,broken)).toThrow(/geometry|boundary|pcurve|embedding|topology|snapshot binding/i)
})
it.each(cases)('original %s strict source budget is refused without changing its data',async stem=>{
 const source=readFileSync(new URL(`../examples/rush/${stem}-circle-hollow-body-boundary.r`,import.meta.url),'utf8')
 const strict=source.replace(/max_deviation:\s*0\.01mm/,'max_deviation: 0.000000000000000000000000000001mm')
 expect(strict).not.toBe(source)
 await expect(buildOwnNurbsAsync(compileRushFrontend(strict).document,{action:'build'})).rejects.toThrow(/budget|unproved|refused|continuous|deviation/i)
})
