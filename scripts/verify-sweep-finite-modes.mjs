import {checkRetainedAxisAudit as checkAxis} from './retained-axis-audit-contract.mjs'
import {readSweepManifest} from './sweep-manifest-reader.mjs'
import {spatialRmfProfileG2BodySources} from './spatial-rmf-profile-g2-sources.mjs'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'
import {checkRetainedSmoothnessContract,checkRetainedSmoothnessInventory} from './sweep-retained-smoothness-contracts.mjs'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
import {bindStepOracle} from './sweep-step-oracle-binding.mjs'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'
const [manifestPath,oraclePath,outputPath]=process.argv.slice(2)
const profileG2Only=process.argv.includes('--law-profile-g2-only')
const spatialG2Only=process.argv.includes('--spatial-profile-g2-only')
const correctedProfileOnly=profileG2Only||spatialG2Only
assert.ok(!(profileG2Only&&spatialG2Only),'Choose one explicit profile matrix')
assert.ok(manifestPath&&oraclePath&&outputPath)
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes),oracle=JSON.parse(readFileSync(oraclePath))
const manifestSha256=createHash('sha256').update(bytes).digest('hex')
const oracleBinding=await bindStepOracle(manifest,manifestSha256,oracle)
assert.equal(oracle.passed,true)
assert.deepEqual(oracle.artifactProvenance,manifest.artifactProvenance)
const cases=manifest.cases,external=new Map(oracle.cases.map(c=>[c.file,c]))
if(profileG2Only)assert.equal(cases.length,16,'Explicit profile G2 matrix must contain every joint-law combination exactly once')
else if(spatialG2Only)assert.equal(cases.length,11,'Spatial profile matrix needs all eleven original companions')
else checkRetainedSmoothnessInventory(cases)
function checkCapturedStep(item){
 const imported=external.get(item.file);assert.equal(imported?.passed,true)
 const hash=createHash('sha256').update(readFileSync(resolve(dirname(manifestPath),item.file))).digest('hex')
 assert.equal(hash,item.sha256);assert.equal(imported.stepSha256,hash)
}
function checkCorrectedGlobal(item){
  const volume=item.nativeVolume
  for(const key of ['allFacesInjective','allPairsClassified','boundaryEmbeddingCertified'])assert.equal(volume[key],true)
  assert.equal(volume.nesting.rolesConsistent,true)
  assert.equal(volume.nesting.visitedPairs,volume.nesting.totalPairs)
  assert.equal(volume.orientations.length,item.shells)
  assert.ok(volume.orientations.every(o=>typeof o.outward==='boolean'&&o.outward===o.expectedOutward))
  const charts=item.nativeRetainedWallCharts
  assert.equal(charts.allChartsCertified,true)
  assert.deepEqual(charts.unresolvedFaces,[])
  assert.deepEqual(charts.charts.map(c=>c.face).sort((a,b)=>a-b),Array.from({length:item.faces},(_,face)=>face).filter(face=>!item.capFaces.includes(face)))
  assert.ok(charts.charts.every(c=>c.audit.certified===true))
  checkCompleteFacePairCover(item.nativeBoundary,item.faces)
}
const lawCases=[]
if(!spatialG2Only)for(const closed of [false,true])for(let bits=0;bits<8;bits++){
 const mode={closed,affine:!!(bits&1),authored:!!(bits&2),guided:!!(bits&4)}
 const laws=[mode.affine?'affine':null,mode.authored?'authored':null,mode.guided?'guide':null].filter(Boolean).join('-')||'plain'
 const file=`miter-laws-${closed?'closed':'open'}-${laws}${profileG2Only?'-profile-g2':''}.step`,matches=cases.filter(c=>c.file===file)
 assert.equal(matches.length,1,file)
 const item=matches[0],construction=item.construction,bound=construction.boundaryCertificate
 assert.deepEqual(item.lawMatrixMode,mode)
 for(const [key,flag] of [['affineLawsApplied','affine'],['authoredFramesApplied','authored'],['orientationGuideApplied','guided']])assert.equal(construction[key],mode[flag],`${file}: ${key}`)
 assert.equal(bound.scope,'boundary-set-hausdorff');assert.equal(bound.method,'retained-sweep-boundary-union')
 assert.equal(bound.continuousBound,true);assert.equal(bound.withinBudget,true);assert.equal(bound.closed,closed)
 assert.ok(Number.isFinite(bound.errorUpper)&&bound.errorUpper>=0&&bound.errorUpper<=bound.budget)
 if(!closed){assert.equal(bound.filledCapErrorUpper.length,2);assert.ok(bound.filledCapErrorUpper.every(e=>Number.isFinite(e)&&e>=0&&e<=bound.budget))}
 assert.equal(item.nativeVolume.solidGeometryCertified,true)
 if(profileG2Only){
  assert.equal(item.explicitCircleCorrection,true)
  assert.deepEqual(item.requestedSmoothness,{profile:'G2',station:'C0'})
  const source=readFileSync(resolve(dirname(manifestPath),item.sourceFile))
  assert.equal(createHash('sha256').update(source).digest('hex'),item.sourceSha256,'Changed original explicit-repair source')
  assert.match(source.toString(),/circle_correction_tolerance: 0\.000000001mm/)
  assert.equal(bound.budget,closed?10:2,'Original law matrix deviation budget changed')
  checkCorrectedGlobal(item)
 }
 checkCapturedStep(item)
 lawCases.push({file,mode,boundary:bound,nativeSolid:true,stationContinuity:item.nativeProfileSmoothness.stationContinuity})
}
const spatial=cases.filter(c=>c.file.startsWith('closed-original-spatial-rmf-holonomy-'))
if(profileG2Only)assert.equal(spatial.length,0,'Explicit profile matrix must not substitute spatial fixtures')
else assert.ok(spatial.length>=11,'Missing previously qualified spatial holonomy cases')
if(spatialG2Only){
 const modes=['affine','affine-hollow','varying-affine-hollow','periodic-affine','periodic-varying-affine-hollow','parameter-varying-affine-hollow','parameter-periodic-varying-affine-hollow','periodic-varying-affine-three-holes','parameter-periodic-varying-affine-three-holes','rational-nonuniform-affine-hollow','antipodal-affine-hollow']
 assert.deepEqual(cases.map(c=>c.file).sort(),modes.map(mode=>`closed-original-spatial-rmf-holonomy-${mode}-tube-profile-g2.step`).sort(),'Wrong spatial mode inventory')
 assert.deepEqual(cases.map(c=>c.sourceFile).sort(),spatialRmfProfileG2BodySources.map(file=>'examples/rush/'+file).sort(),'Wrong original source inventory')
 for(const [index,item] of cases.entries()){
  const expectedFile=`closed-original-spatial-rmf-holonomy-${modes[index]}-tube-profile-g2.step`
  assert.equal(item.file,expectedFile);assert.equal(item.sourceFile,'examples/rush/'+spatialRmfProfileG2BodySources[index])
  assert.equal(item.explicitCircleCorrection,true);assert.deepEqual(item.requestedSmoothness,{profile:'G2',station:'C0'})
  const source=readFileSync(resolve(dirname(manifestPath),item.sourceFile))
  assert.equal(createHash('sha256').update(source).digest('hex'),item.sourceSha256,'Changed explicit original source')
  assert.match(source.toString(),/circle_correction_tolerance: 0\.000000001mm/)
  const expectedBudget=modes[index].includes('antipodal')?1:modes[index].includes('rational-nonuniform')?.4:.25
  assert.equal(item.boundaryError.budget,expectedBudget,'Original spatial deviation budget changed')
  const owner=item.construction[item.boundaryError.nodeId].bodyBoundaryEvidence
  assert.equal(owner.closedPath,true);assert.ok(Number.isFinite(owner.profileCorrectionErrorUpper)&&owner.profileCorrectionErrorUpper>0&&owner.profileCorrectionErrorUpper<=1e-9)
  assert.equal(owner.retainedWalls.certified,true);assert.equal(owner.retainedWalls.faceCoverageCertified,true);assert.equal(owner.retainedWalls.coefficientFamilyCertified,true)
  assert.equal(item.boundaryError.profileG2Certified,true);assert.equal(item.boundaryError.stationContinuity,'C0')
  checkCorrectedGlobal(item)
 }
}
for(const item of spatial){const b=item.boundaryError;assert.equal(b.continuousBound,true);assert.equal(b.withinBudget,true);assert.ok(Number.isFinite(b.errorUpper)&&b.errorUpper>=0&&b.errorUpper<=b.budget);assert.equal(item.nativeVolume.solidGeometryCertified,true);checkCapturedStep(item)}
const smoothness=[]

for(const item of cases){
 checkCompleteBoundaryContract(item)
 const requiredSmoothness=correctedProfileOnly?item.requestedSmoothness:checkRetainedSmoothnessContract(item)
 const p=item.nativeProfileSmoothness,station=p.station
 if(correctedProfileOnly){
  assert.equal(p.profileG1Certified,true)
  assert.equal(p.profile.exactG1G2Certified,true)
  assert.equal(station.stationG1Certified,false)
  assert.equal(station.stationG2Certified,false)
  assert.equal(p.stationContinuity,'C0')
 }
 checkAxis(p,p.profile,p.g1Audit,p.profileG1Certified,p.profile.exactG1G2Certified)
 checkAxis(station,station.g2,station.g1Audit,station.stationG1Certified,station.stationG2Certified)
 assert.equal(p.totalExactWork,p.exactWork+station.exactWork)
 assert.ok(p.totalExactWork<=p.maxWork)
 assert.equal(p.stationContinuity,station.stationG2Certified?'G2':station.stationG1Certified?'G1':'C0')
 assert.equal(p.capContinuity,item.capFaces.length?'C0':'absent')
 assert.equal(p.fullBoundarySmoothnessCertified,false,'Scoped wall audits must not claim the entire boundary')
 const owned=[...p.edgeIds,...station.edgeIds,...station.capEdges]
 assert.equal(new Set(owned).size,owned.length,'Seam ownership overlaps')
 assert.equal(owned.length,item.edges,'Uncovered retained boundary edges')
 checkCapturedStep(item)
 smoothness.push({file:item.file,requiredSmoothness,profileContinuity:p.profile.exactG1G2Certified?'G2':p.profileG1Certified?'G1':'C0',profileG2:p.profile.exactG1G2Certified,stationContinuity:p.stationContinuity,profileEdges:p.edgeIds.length,stationEdges:station.edgeIds.length,capEdges:station.capEdges.length,wholeBoundarySmoothness:false})
}
writeFileSync(outputPath,JSON.stringify({schema:'sweep-finite-modes/2',manifestSha256,artifactProvenance:manifest.artifactProvenance,oracleBinding,oracleBindingVerifierSha256:createHash('sha256').update(readFileSync(new URL('./sweep-step-oracle-binding.mjs',import.meta.url))).digest('hex'),declaredSmoothnessContractCases:smoothness.filter(s=>s.requiredSmoothness!==null).length,verifierSourceSha256:createHash('sha256').update(readFileSync(new URL(import.meta.url))).digest('hex'),contractSourceSha256:createHash('sha256').update(readFileSync(new URL('./sweep-retained-smoothness-contracts.mjs',import.meta.url))).digest('hex'),retainedAxisVerifierSha256:createHash('sha256').update(readFileSync(new URL('./retained-axis-audit-contract.mjs',import.meta.url))).digest('hex'),completeBoundaryVerifierSha256:createHash('sha256').update(readFileSync(new URL('./sweep-complete-boundary-contracts.mjs',import.meta.url))).digest('hex'),scope:'Captured finite mode examples only; no general theorem, UI completion, or whole-library completion claim',lawCases,smoothness,spatial:spatial.map(c=>({file:c.file,boundary:c.boundaryError,nativeSolid:true,stationContinuity:c.nativeProfileSmoothness.stationContinuity}))},null,2)+'\n')
console.log(`${lawCases.length} joint-law cases and ${spatial.length} spatial holonomy cases verified; ${smoothness.length} complete scoped seam ownership audits`)
