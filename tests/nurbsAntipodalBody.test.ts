import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

it('owns antipodal holonomy error and independently audits retained hollow Solid',async()=>{
 const source=readFileSync('examples/rush/closed-antipodal-spatial-rmf-affine-hollow-body.r','utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Missing antipodal retained body')
 const evidence=readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)
 expect(evidence).toMatchObject({continuousBound:true,withinBudget:true,budget:1})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(model.faces).toHaveLength(512)
 expect(model.shells).toHaveLength(2)
 expect(model.bodies[0].innerShells).toEqual([1])
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)!
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,
  allPairsClassified:true,nextPair:null,nesting:{rolesConsistent:true,parents:[null,0]}})
 expect(volume.individualPairs+volume.groupedPairs).toBe(512*511/2)
 expect(volume.orientations.map(o=>[o.expectedOutward,o.outward])).toEqual([[true,true],[false,false]])
 expect(inspectSweepVolume(model,[],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1}))
  .toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false})
 for(const change of [
  source.replace('max_deviation: 1mm','max_deviation: 0.000001mm'),
  source.replace('error_max_cells: 100000','error_max_cells: 0'),
  source.replace('[1mm,0mm,0mm]','[1.0000000000000002mm,0mm,0mm]'),
 ]){
  expect(change).not.toBe(source)
  await expect(buildOwnNurbsAsync(compileRushFrontend(change).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error/)
 }
 expect(JSON.stringify(document)).toBe(before)
})
