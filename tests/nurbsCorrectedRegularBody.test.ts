import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {correctedRegularBodySources} from '../scripts/sweep-body-matrix-sources.mjs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {tessellateNurbsBrep} from '../src/services/geometry/brep'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

it.each(correctedRegularBodySources)('preserves corrected regular whole-body proof, preview and fresh Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing native corrected regular body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:.05})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 const stored=JSON.stringify(model),caps=[model.faces.length-2,model.faces.length-1]
 const mesh=tessellateNurbsBrep(model,4)
 expect(mesh.indices.length/3).toBeLessThanOrEqual(20000)
 expect(new Set(mesh.faceIds).size).toBe(model.faces.length)
 expect(mesh.report).toMatchObject({closed:true,nonManifoldEdges:0,orientationConflicts:0,degenerateTriangles:0})
 if(file.startsWith('arc-length-')){
  // Two four-span contours contribute eight walls per retained station interval;
  // adaptive refinement may choose fewer intervals when the proven bound improves.
  const wallCount=model.faces.length-2
  expect(wallCount).toBeGreaterThanOrEqual(16)
  expect(wallCount).toBeLessThanOrEqual(8*(129-1))
  expect(wallCount%8).toBe(0)
 }
 const volume=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)!
 expect(volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(volume.boundary!.caps.map(cap=>cap.face)).toEqual(caps)
 expect(volume.boundary!.caps.every(cap=>cap.capCertified&&cap.unresolvedWalls.length===0)).toBe(true)
 expect(volume.orientations.every(shell=>shell.outward===shell.expectedOutward)).toBe(true)
 expect(inspectSweepVolume(model,caps,{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1}))
  .toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false})
 expect(JSON.stringify(model)).toBe(stored)
 expect(JSON.stringify(document)).toBe(before)
})
