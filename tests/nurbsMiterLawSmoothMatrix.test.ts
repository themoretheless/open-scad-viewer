import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {expect,it} from 'vitest'
import {miterLawMatrixModes,miterLawMatrixSmoothSource} from '../scripts/miter-law-matrix-sources'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each(miterLawMatrixModes)('proves retained profile G2 for explicitly repaired joint laws: %j',async mode=>{
 const document=compileRushFrontend(miterLawMatrixSmoothSource(mode)).document,before=JSON.stringify(document)
 const node=document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true,
  authoredFramesApplied:mode.authored,orientationGuideApplied:mode.guided,affineLawsApplied:mode.affine,
  profileSmoothness:{profileG1Certified:true,profile:{exactG1G2Certified:true},stationContinuity:'C0',fullBoundarySmoothnessCertified:false}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true,
  solidGeometryCertified:true,profileG1Certified:true,profileG2Certified:true,stationContinuity:'C0'})
 if(!built.nativeGeometry)throw Error('Missing repaired joint-law body')
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,readNativeGeometryData(built.nativeGeometry).geometry)!
 expect(volume).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(volume.orientations.every(o=>o.outward===o.expectedOutward)).toBe(true)
 expect(JSON.stringify(document)).toBe(before)
})
