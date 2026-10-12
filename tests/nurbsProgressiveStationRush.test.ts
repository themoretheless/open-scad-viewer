import type {NurbsBrep} from '../src/services/geometry/brep'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {MAX_NATIVE_GEOMETRY_CHARACTERS,isNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each(['straight-progressive-body-station-g2.r','closed-spatial-rmf-varying-affine-hollow-body-station-g2.r','arc-length-periodic-authored-station-g2.r'])('qualifies owned station reconstruction through Rush, viewport and fresh Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8'),document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing reconstructed body')
 expect(isNativeGeometryArtifact(built.nativeGeometry)).toBe(true)
 expect(built.nativeGeometry.geometryJson.length+built.nativeGeometry.documentJson.length).toBeLessThanOrEqual(MAX_NATIVE_GEOMETRY_CHARACTERS)
 if(file.startsWith('closed'))expect(JSON.parse(built.nativeGeometry.geometryJson).encoding).toBe('native-deflate-json/1')
 const data=readNativeGeometryData(built.nativeGeometry),evidence=data.sweepEvidence
 expect(evidence.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,budget:file.startsWith('straight')?0.01:file.startsWith('arc-length')?0.5:0.25})
 expect(evidence.profileSmoothness).toMatchObject({profile:{exactG1G2Certified:true},station:{stationG2Certified:true}})
 if(file.startsWith('arc-length'))expect((data.geometry as NurbsBrep).faces.every(face=>[5,13,25].includes(face.surface.degreeV))).toBe(true)
 expect(data.sweepBodyBoundaryEvidence).toBeUndefined()
 expect(data.sweepStationReconstruction.source).toBe('original-progressive-profile-request')
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,profileG2Certified:true,stationG2Certified:true})
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,data.geometry)).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(JSON.stringify(document)).toBe(before)
})

it('refuses exhausted station work and a replacement source budget',async()=>{
 const source=readFileSync('examples/rush/straight-progressive-body-station-g2.r','utf8')
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('quantum: 0.125mm,max_work: 1000000','quantum: 0.125mm,max_work: 0')).document,{action:'build'})).rejects.toThrow(/reconstruction/)
 expect(()=>compileRushFrontend(source.replace('smoothness_max_work: 2000000','smoothness_max_work: 2000000,max_deviation: 1mm'))).toThrow()
})
