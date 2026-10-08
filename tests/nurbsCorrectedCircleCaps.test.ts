import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

it.each([
 'oblique-rmf-circle-hollow-corrected-body-boundary.r',
 'curved-rmf-circle-hollow-corrected-body-boundary.r',
 'curved-frenet-circle-hollow-corrected-body-boundary.r',
])('proves corrected circular hollow caps through Rush, viewport and fresh Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const graph=compileRushFrontend(source).document,before=JSON.stringify(graph)
 const built=await buildOwnNurbsAsync(graph,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing corrected circular hollow body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:.01})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(model.faces.slice(-2).map((f:any)=>f.holes.length)).toEqual([1,1])
 const stored=JSON.stringify(model)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(inspectSweepVolume(model,[model.faces.length-2,model.faces.length-1],{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1}))
  .toMatchObject({solidGeometryCertified:false,boundaryEmbeddingCertified:false})
 const exhausted=source.replace('cap_correction_max_work: 1000000','cap_correction_max_work: 1')
 expect(exhausted).not.toBe(source)
 await expect(buildOwnNurbsAsync(compileRushFrontend(exhausted).document,{action:'build'})).rejects.toThrow(/work|budget|correction/i)
 expect(JSON.stringify(model)).toBe(stored)
 expect(JSON.stringify(graph)).toBe(before)
})

it('qualifies the valid 65-station oblique corrected native cancellation workload',async()=>{
 const source=readFileSync('examples/rush/oblique-rmf-circle-hollow-corrected-body-boundary.r','utf8')
  .replace('initial_sections: 2,max_sections: 2','initial_sections: 65,max_sections: 65')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing 65-station corrected oblique body')
 expect(readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true,withinBudget:true,budget:.01})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(model.faces).toHaveLength(514)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  allFacesInjective:true,allPairsClassified:true,nextPair:null})
})
