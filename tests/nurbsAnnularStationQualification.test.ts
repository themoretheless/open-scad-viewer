import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

// Actual frozen WASM; original curves/laws and0.01mm source budget.
// G2 is required at retained station joins. Cap-wall joints retain C0.
const source=readFileSync('examples/rush/oblique-rmf-annular-body-station-g2.r','utf8')
it.each(['sync','async'] as const)('preserves native radial caps and actual station G2 through Rush (%s)',async mode=>{
 const document=compileRushFrontend(source).document,before=structuredClone(document)
 const options={action:'build' as const,display:{segments:2,subdivisionLevels:0}}
 const built=mode==='sync'?buildOwnNurbs(document,options):await buildOwnNurbsAsync(document,options)
 if(!built.nativeGeometry)throw Error('Missing actual reconstructed snapshot')
 const data=readNativeGeometryData(built.nativeGeometry),evidence=data.sweepEvidence
 expect(data.geometry.faces).toHaveLength(24)
 expect(data.sweepPlanarCapFaces).toEqual([])
 expect(data.sweepStationReconstruction).toMatchObject({source:'original-progressive-profile-request',
  capRepresentation:'radial-annular',closedPath:false,planarCapFaces:[]})
 expect(data.sweepStationReconstruction.capFaceGroups.map((g:number[])=>g.length)).toEqual([4,4])
 expect(data.sweepStationReconstruction.capFaceGroups.flat()).toEqual(data.sweepStationReconstruction.capFaces)
 expect(data.sweepStationReconstruction.capFaces).toEqual(Array.from({length:8},(_,i)=>16+i))
 expect(evidence.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,budget:0.01})
 expect(evidence.profileSmoothness.station.stationG2Certified).toBe(true)
 expect(evidence.retainedSmoothness).toBeUndefined()
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,
  boundaryErrorWithinBudget:true,stationG2Certified:true})
 const actual=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,data.geometry)
 expect(actual).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(actual!.boundary!.totalPairs).toBe(276)
 expect(actual!.boundary!.individualPairs+actual!.boundary!.groupedPairs).toBe(276)
 expect(document).toEqual(before)
})
it('refuses zero station work through the same original Rush source',async()=>{
 const refused=source.replace('max_work: 1000000','max_work: 0')
 expect(refused).not.toBe(source)
 await expect(buildOwnNurbsAsync(compileRushFrontend(refused).document,{action:'build'})).rejects.toThrow(/reconstruction|work|unproved/i)
})
