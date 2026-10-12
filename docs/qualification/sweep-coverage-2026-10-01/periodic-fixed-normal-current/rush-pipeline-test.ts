// Qualification transport only: all geometry and certification execute in Rust WASM.
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
const cases=[false,true].flatMap(joint=>[false,true].map(hollow=>({joint,hollow,file:`periodic-fixed-normal-arc-${joint?'affine':'plain'}-${hollow?'hollow':'solid'}-station-g2.r`})))
import type {NurbsBrep} from '../src/services/geometry/brep'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each(cases)('keeps original periodic FixedNormal arc bound and G2 through Rush/viewport/Solid: $file',async ({file,hollow})=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing retained station body')
 const data=readNativeGeometryData(built.nativeGeometry),model=data.geometry as NurbsBrep
 expect(data.sweepEvidence.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,budget:.5})
 expect(data.sweepEvidence.profileSmoothness).toMatchObject({profile:{exactG1G2Certified:true},station:{stationG2Certified:true}})
 expect(data.sweepStationReconstruction.source).toBe('original-progressive-profile-request')
 expect(model.shells).toHaveLength(hollow?2:1)
 expect(model.faces).toHaveLength(hollow?256:128)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true,profileG2Certified:true,stationG2Certified:true})
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(JSON.stringify(document)).toBe(before)
})
