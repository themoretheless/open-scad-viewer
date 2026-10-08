import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it.each([17,65])('carries a nonbinary13/12 original guide phase through Rush, viewport evidence and fresh Solid (%i initial sections)',async count=>{
 const source=readFileSync('examples/rush/closed-arc-length-rational-phase-contact-hollow-body-boundary.r','utf8').replace('initial_sections: 17',`initial_sections: ${count}`)
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing nonbinary original guide phase boundary')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:2})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 // Only the65-station cancellation workload fixes both station limits.
 // The17-station request can refine up to65 to meet the original bound.
 if(count===65)expect(model.faces).toHaveLength(512)
 const before=JSON.stringify(model)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null,
  nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{outward:true},{outward:false}]})
 expect(JSON.stringify(model)).toBe(before)
})

it.each(['closed-contact-hollow-body-boundary.r','closed-arc-length-contact-hollow-body-boundary.r'])(
 'carries original closed contact boundaries and material roles through Rush and fresh Solid: %s',async name=>{
 const source=readFileSync(`examples/rush/${name}`,'utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing native closed contact boundary')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:2})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(model.shells).toHaveLength(2)
 const before=JSON.stringify(model)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null,
  nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{outward:true},{outward:false}]})
 const refused=source.replace('max_deviation: 2mm','max_deviation: 0.000000000001mm')
 expect(refused).not.toBe(source)
 await expect(buildOwnNurbsAsync(compileRushFrontend(refused).document,{action:'build'}))
  .rejects.toThrow(/refinement exceeds budget/)
 expect(JSON.stringify(model)).toBe(before)
})

it.each(['closed-contact-hollow-body-boundary.r','closed-arc-length-contact-hollow-body-boundary.r'])(
 'qualifies the valid 65-section closed contact native cancellation workload: %s',async name=>{
 const original=readFileSync(`examples/rush/${name}`,'utf8')
 const source=original.replace('initial_sections: 17','initial_sections: 65')
 expect(source).not.toBe(original)
 let last:unknown
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}},
  {onSweepPreview:(_id,preview)=>{last=preview.report}}).catch(error=>{throw Error('65-section contact workload: '+JSON.stringify(last),{cause:error})})
 if(!built.nativeGeometry)throw Error('Missing native cancellation workload boundary')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry),JSON.stringify(JSON.parse(built.nativeGeometry.geometryJson).sweepBodyBoundaryEvidence)+" last="+JSON.stringify(last)).toMatchObject({continuousBound:true,withinBudget:true,budget:2})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(model.faces).toHaveLength(512)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  allPairsClassified:true,nesting:{rolesConsistent:true,parents:[null,0]},orientations:[{outward:true},{outward:false}]})
})
