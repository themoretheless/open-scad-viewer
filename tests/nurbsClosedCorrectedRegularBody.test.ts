import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {tessellateNurbsBrep} from '../src/services/geometry/brep'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

const sources=[
 'arc-length-periodic-corrected-frenet-nonplanar-body-boundary.r','periodic-corrected-frenet-nonplanar-body-boundary.r',
 'closed-corrected-frenet-nonplanar-body-boundary.r',
 'closed-corrected-frenet-nonplanar-affine-body-boundary.r',
 'arc-length-closed-corrected-frenet-nonplanar-body-boundary.r',
]
it.each(sources)('qualifies closed corrected original error and fresh retained Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing closed corrected regular body')
 const boundary=JSON.parse(built.nativeGeometry.geometryJson).sweepBodyBoundaryEvidence
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry),JSON.stringify({file,boundary})).toMatchObject({continuousBound:true,withinBudget:true,budget:.05})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry,stored=JSON.stringify(model)
 const mesh=tessellateNurbsBrep(model,4)
 expect(mesh.indices.length/3).toBeLessThanOrEqual(20000)
 expect(new Set(mesh.faceIds).size).toBe(model.faces.length)
 expect(mesh.report).toMatchObject({closed:true,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0})
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)!
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume.boundary!.caps).toEqual([])
 expect(volume.nesting!.rolesConsistent).toBe(true)
 expect(volume.orientations.every(shell=>shell.outward===shell.expectedOutward)).toBe(true)
 expect(inspectSweepVolume(model,[],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1}))
  .toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false})
 expect(JSON.stringify(model)).toBe(stored)
 expect(JSON.stringify(document)).toBe(before)
})
