import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {spatialRmfProfileG2BodySources} from '../scripts/spatial-rmf-profile-g2-sources.mjs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'

it.each(spatialRmfProfileG2BodySources)('qualifies explicit spatial profile repair through Rush, viewport and fresh Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing corrected spatial body')
 const data=JSON.parse(built.nativeGeometry.geometryJson),model=data.geometry
 const boundary=data.sweepBodyBoundaryEvidence
 expect(boundary.profileCorrectionErrorUpper).toBeGreaterThan(0)
 expect(boundary.profileCorrectionErrorUpper).toBeLessThanOrEqual(1e-9)
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,profileG1Certified:true,profileG2Certified:true,stationContinuity:'C0'})
 const actual=inspectMiterProfileSmoothness(model,[])
 expect(actual.profileG1Certified).toBe(true)
 expect(actual.profile.exactG1G2Certified).toBe(true)
 expect(actual.stationContinuity).toBe('C0')
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)!
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume.nesting!.rolesConsistent).toBe(true)
 expect(volume.orientations.every(o=>o.outward===o.expectedOutward)).toBe(true)
 expect(JSON.stringify(document)).toBe(before)
})

it('refuses exhausted spatial profile repair through Rush',async()=>{
 const source=readFileSync(`examples/rush/${spatialRmfProfileG2BodySources[0]}`,'utf8')
 const document=compileRushFrontend(source.replace('circle_correction_max_work: 1000000','circle_correction_max_work: 0')).document
 await expect(buildOwnNurbsAsync(document,{action:'build'})).rejects.toThrow(/profile circle correction refused/i)
})
