import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {stationFrameSources} from '../scripts/station-frame-matrix-sources.mjs'
import type {NurbsBrep} from '../src/services/geometry/brep'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each(stationFrameSources)('keeps the original curved frame bound and retained G2 through Rush and Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing retained station body')
 const data=readNativeGeometryData(built.nativeGeometry),model=data.geometry as NurbsBrep
 expect(data.sweepEvidence.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,budget:.05})
 expect(data.sweepEvidence.profileSmoothness).toMatchObject({profile:{exactG1G2Certified:true},station:{stationG2Certified:true}})
 expect(data.sweepStationReconstruction.source).toBe('original-progressive-profile-request')
 expect(model.shells).toHaveLength(1)
 expect(model.faces.slice(-2).map(face=>face.holes.length)).toEqual([1,1])
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true,profileG2Certified:true,stationG2Certified:true})
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(JSON.stringify(document)).toBe(before)
})
