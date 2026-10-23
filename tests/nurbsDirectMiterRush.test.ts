import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it('owns original direct circle correction through Rush, viewport and fresh Solid admission',async()=>{
 const source=readFileSync('examples/rush/direct-miter-circle-owned-boundary.r','utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const node=document.nodes.find(n=>n.op==='brep_miter_sweep')!
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({retentionMode:'direct',continuousBound:true,
  boundaryErrorWithinBudget:true,profileSmoothness:{profile:{exactG1G2Certified:true},stationContinuity:'C0'},
  volume:{solidGeometryCertified:true}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true,profileG2Certified:true})
 if(!built.nativeGeometry)throw Error('Missing native direct miter body')
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,readNativeGeometryData(built.nativeGeometry).geometry))
  .toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(JSON.stringify(document)).toBe(before)
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('circle_correction_max_work: 1000000','circle_correction_max_work: 0')).document,{action:'build'})).rejects.toThrow()
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_deviation: 0.01mm','max_deviation: 0.000000000000000000000001mm')).document,{action:'build'})).rejects.toThrow(/budget|boundary error/)
 expect(()=>compileRushFrontend(source.replace('max_deviation: 0.01mm,',''))).toThrow(/max_deviation/)
})

it.each([
 '[[1,0,0,2mm],[0,1,0,3mm],[0,0,1,4mm],[0,0,0,1]]',
 '[[-1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]',
])('replays original direct ownership and material orientation after affine placement: %s',async matrix=>{
 const source=readFileSync('examples/rush/direct-miter-circle-owned-boundary.r','utf8')
  .replace('.brep_tessellate(4)',`.transform(matrix: ${matrix}).brep_tessellate(4)`)
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing placed native direct miter body')
 const data=readNativeGeometryData(built.nativeGeometry)
 expect(data.sweepMiterReplay.source).toMatchObject({direct:true,maxDeviation:.01,circleCorrection:{tolerance:1e-9}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true,solidGeometryCertified:true})
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,data.geometry)!
 expect(volume).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(volume.orientations.every(o=>o.outward===o.expectedOutward)).toBe(true)
 const changed=structuredClone(data.geometry)
 changed.faces[0].surface.controlPoints[0][0][0]+=.01
 expect(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry,changed)).toThrow(/final model differs from (the snapshot binding|original replay)/)
})
