import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'
import {bindStepOracle} from './sweep-step-oracle-binding.mjs'
import {stationFrameSources} from './station-frame-matrix-sources.mjs'

// Reconcile captured Rust certificates with independently imported STEP.
// This explicit fixture inventory does not establish unrestricted coverage.
const [directory,sourceRoot='.']=process.argv.slice(2)
assert.ok(directory,'Usage: STEP-directory [original-source-root]')
const bytes=readFileSync(resolve(directory,'manifest.json')),manifest=JSON.parse(bytes)
const oracle=JSON.parse(readFileSync(resolve(directory,'opencascade-sweep.json')))
assert.equal(oracle.passed,true)
const binding=await bindStepOracle(manifest,createHash('sha256').update(bytes).digest('hex'),oracle)
const frameMatrix=process.argv.includes('--station-frame-matrix')
const inventory=new Map(frameMatrix?stationFrameSources.map(file=>[file.replace(/\.r$/,'.step'),.05]):[
 ['straight-progressive-body-station-g2.step',.01],
 ['closed-spatial-rmf-varying-affine-hollow-body-station-g2.step',.25],
])
if(process.argv.includes('--periodic-authored')){
 assert.equal(frameMatrix,false,'Choose one explicit station inventory')
 inventory.set('arc-length-periodic-authored-station-g2.step',.5)
}
assert.equal(manifest.cases.length,inventory.size)
const cases=manifest.cases.map(item=>{
 assert.ok(inventory.has(item.file),'Unexpected station fixture')
 const budget=inventory.get(item.file);inventory.delete(item.file)
 const source=readFileSync(resolve(sourceRoot,item.sourceFile))
 assert.equal(createHash('sha256').update(source).digest('hex'),item.sourceSha256,'Original source changed')
 if(frameMatrix){
  assert.equal(item.sourceFile,'examples/rush/'+item.file.replace(/\.step$/,'.r'))
  assert.match(source.toString(),/initial_sections: 17,max_sections: 17,max_deviation: 0\.05mm/)
  assert.match(source.toString(),/wall_tolerance: 0\.01mm/)
  assert.equal(item.capHoleFaces,2);assert.equal(item.capFaces.length,2)
 }
 const bound=checkCompleteBoundaryContract(item)
 assert.equal(bound.status,'certified');assert.equal(bound.budget,budget)
 for(const key of ['continuousBound','boundaryErrorWithinBudget','profileG2Certified','stationG2Certified','solidGeometryCertified'])assert.equal(item.boundaryEvidence[key],true)
 assert.equal(item.boundaryEvidence.boundaryErrorUpper,bound.upper)
 assert.equal(item.boundaryEvidence.boundaryErrorBudget,budget)
 assert.equal(item.nativeProfileSmoothness.profile.exactG1G2Certified,true)
 const station=item.nativeProfileSmoothness.station
 assert.equal(station.stationG2Certified,true);assert.equal(station.extractionComplete,true)
 assert.deepEqual(station.unclassifiedFaces,[]);assert.deepEqual(station.unpairedEdges,[])
 for(const [axis,g2] of [[item.nativeProfileSmoothness,item.nativeProfileSmoothness.profile],[station,station.g2]]){
  assert.equal(axis.extractionComplete,true)
  assert.deepEqual(axis.unclassifiedFaces,[]);assert.deepEqual(axis.unpairedEdges,[])
  assert.equal(new Set(axis.edgeIds).size,axis.edgeIds.length)
  assert.equal(g2.exactG1G2Certified,true);assert.equal(g2.certifiedOrder,2)
  assert.deepEqual(g2.unresolvedSeams,[])
  assert.equal(g2.seams.length,axis.edgeIds.length);assert.ok(g2.seams.length>0)
  assert.ok(g2.seams.every(s=>s.certified&&s.exactIdentity&&s.regularityCertified))
  assert.equal(axis.exactWork,g2.exactWork+(axis.g1Audit?.exactWork??0))
 }
 assert.equal(item.nativeProfileSmoothness.totalExactWork,item.nativeProfileSmoothness.exactWork+station.exactWork)
 assert.ok(item.nativeProfileSmoothness.totalExactWork<=item.nativeProfileSmoothness.maxWork)
 assert.equal(item.nativeRetainedWallCharts.allChartsCertified,true)
 const charts=item.nativeRetainedWallCharts
 assert.deepEqual(charts.unresolvedFaces,[])
 assert.deepEqual(charts.charts.map(c=>c.face).sort((a,b)=>a-b),Array.from({length:item.faces},(_,face)=>face).filter(face=>!item.capFaces.includes(face)))
 assert.ok(charts.charts.every(c=>c.audit.certified===true))
 const owned=[...item.nativeProfileSmoothness.edgeIds,...station.edgeIds,...station.capEdges]
 assert.equal(new Set(owned).size,owned.length,'Overlapping retained seam ownership')
 assert.equal(owned.length,item.edges,'Uncovered retained boundary edges')
 for(const key of ['allFacesInjective','allPairsClassified','boundaryEmbeddingCertified','solidGeometryCertified'])assert.equal(item.nativeVolume[key],true)
 checkCompleteFacePairCover(item.nativeBoundary,item.faces)
 const nesting=item.nativeVolume.nesting
 assert.equal(nesting.rolesConsistent,true);assert.equal(nesting.visitedPairs,nesting.totalPairs)
 assert.equal(item.nativeVolume.orientations.length,item.shells)
 assert.ok(item.nativeVolume.orientations.every(v=>typeof v.outward==='boolean'&&v.outward===v.expectedOutward))
 const bad=structuredClone(item);bad.construction.boundaryCertificate.continuousBound=false
 assert.throws(()=>checkCompleteBoundaryContract(bad),'False whole-bound claim admitted')
 return {file:item.file,bound,actualProfileG2:true,actualStationG2:true,actualGlobalSolid:true,completeFacePairCover:true}
})
assert.equal(inventory.size,0)
writeFileSync(resolve(directory,'station-contracts.json'),JSON.stringify({scope:`${cases.length} explicit owned station fixtures; not universal mode coverage`,binding,cases,passed:true},null,2)+'\n')
console.log(`${cases.length}/${cases.length} owned station contracts verified`)
