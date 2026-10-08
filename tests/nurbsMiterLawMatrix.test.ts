import {miterLawMatrixModes,miterLawMatrixSource} from '../scripts/miter-law-matrix-sources'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each(miterLawMatrixModes)('owns all miter law combinations through Rush/viewport/Solid: %j',async mode=>{
 const source=miterLawMatrixSource(mode)
 const document=compileRushFrontend(source).document
 const node=document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const report=built.report.construction![node.id]
 expect(report).toMatchObject({accepted:true,continuousBound:true,closedPath:mode.closed,
  authoredFramesApplied:mode.authored,orientationGuideApplied:mode.guided,affineLawsApplied:mode.affine,
  volume:{solidGeometryCertified:true}})
 if(mode.closed)expect(report).toMatchObject({seamContinuity:'C0'})
 if(!built.nativeGeometry)throw Error('Missing native owned miter body')
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({solidGeometryCertified:true,continuousBound:true,boundaryErrorWithinBudget:true})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)!
 expect(volume).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(volume.orientations.every(o=>o.outward===o.expectedOutward)).toBe(true)
 expect(JSON.stringify(document)).toBe(before)
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace(/max_deviation: (2|10)mm/,'max_deviation: 0.000000000001mm')).document,{action:'build'})).rejects.toThrow()
})
