import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'
const [manifestPath,oraclePath,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&oraclePath&&outputPath)
const bytes=readFileSync(manifestPath),manifest=JSON.parse(bytes),oracle=JSON.parse(readFileSync(oraclePath))
const manifestSha256=createHash('sha256').update(bytes).digest('hex')
assert.equal(oracle.manifestSha256,manifestSha256)
assert.equal(oracle.passed,true)
assert.deepEqual(oracle.artifactProvenance,manifest.artifactProvenance)
const cases=manifest.cases,external=new Map(oracle.cases.map(c=>[c.file,c]))
function checkCapturedStep(item){
 const imported=external.get(item.file);assert.equal(imported?.passed,true)
 const hash=createHash('sha256').update(readFileSync(resolve(dirname(manifestPath),item.file))).digest('hex')
 assert.equal(hash,item.sha256);assert.equal(imported.stepSha256,hash)
}
const lawCases=[]
for(const closed of [false,true])for(let bits=0;bits<8;bits++){
 const mode={closed,affine:!!(bits&1),authored:!!(bits&2),guided:!!(bits&4)}
 const laws=[mode.affine?'affine':null,mode.authored?'authored':null,mode.guided?'guide':null].filter(Boolean).join('-')||'plain'
 const file=`miter-laws-${closed?'closed':'open'}-${laws}.step`,matches=cases.filter(c=>c.file===file)
 assert.equal(matches.length,1,file)
 const item=matches[0],construction=item.construction,bound=construction.boundaryCertificate
 assert.deepEqual(item.lawMatrixMode,mode)
 for(const [key,flag] of [['affineLawsApplied','affine'],['authoredFramesApplied','authored'],['orientationGuideApplied','guided']])assert.equal(construction[key],mode[flag],`${file}: ${key}`)
 assert.equal(bound.scope,'boundary-set-hausdorff');assert.equal(bound.method,'retained-sweep-boundary-union')
 assert.equal(bound.continuousBound,true);assert.equal(bound.withinBudget,true);assert.equal(bound.closed,closed)
 assert.ok(Number.isFinite(bound.errorUpper)&&bound.errorUpper>=0&&bound.errorUpper<=bound.budget)
 if(!closed){assert.equal(bound.filledCapErrorUpper.length,2);assert.ok(bound.filledCapErrorUpper.every(e=>Number.isFinite(e)&&e>=0&&e<=bound.budget))}
 assert.equal(item.nativeVolume.solidGeometryCertified,true)
 checkCapturedStep(item)
 lawCases.push({file,mode,boundary:bound,nativeSolid:true,stationContinuity:item.nativeProfileSmoothness.stationContinuity})
}
const spatial=cases.filter(c=>c.file.startsWith('closed-original-spatial-rmf-holonomy-'))
assert.ok(spatial.length>=11,'Missing previously qualified spatial holonomy cases')
for(const item of spatial){const b=item.boundaryError;assert.equal(b.continuousBound,true);assert.equal(b.withinBudget,true);assert.ok(Number.isFinite(b.errorUpper)&&b.errorUpper>=0&&b.errorUpper<=b.budget);assert.equal(item.nativeVolume.solidGeometryCertified,true);checkCapturedStep(item)}
const smoothness=[]
function checkAxis(axis,g2,g1,certifiedG1,certifiedG2){
 assert.equal(axis.extractionComplete,true);assert.deepEqual(axis.unclassifiedFaces,[]);assert.deepEqual(axis.unpairedEdges,[])
 assert.equal(new Set(axis.edgeIds).size,axis.edgeIds.length)
 assert.ok(Number.isSafeInteger(axis.exactWork)&&axis.exactWork>=0&&axis.exactWork<=axis.maxWork)
 assert.equal(axis.exactWork,g2.exactWork+(g1?.exactWork??0))
 assert.equal(certifiedG2,g2.exactG1G2Certified)
 assert.equal(certifiedG1,certifiedG2||g1?.exactG1G2Certified===true)
 for(const [collection,order] of [[g2,2],[g1,1]])if(collection?.exactG1G2Certified){
  assert.equal(collection.certifiedOrder,order);assert.deepEqual(collection.unresolvedSeams,[])
  assert.equal(collection.seams.length,axis.edgeIds.length)
  assert.ok(collection.seams.length>0)
  assert.ok(collection.seams.every(seam=>seam.certified&&seam.exactIdentity&&seam.regularityCertified))
 }
}
for(const item of cases){
 const p=item.nativeProfileSmoothness,station=p.station
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
 smoothness.push({file:item.file,profileContinuity:p.profile.exactG1G2Certified?'G2':p.profileG1Certified?'G1':'C0',profileG2:p.profile.exactG1G2Certified,stationContinuity:p.stationContinuity,profileEdges:p.edgeIds.length,stationEdges:station.edgeIds.length,capEdges:station.capEdges.length,wholeBoundarySmoothness:false})
}
writeFileSync(outputPath,JSON.stringify({schema:'sweep-finite-modes/1',manifestSha256,artifactProvenance:manifest.artifactProvenance,scope:'Captured finite mode examples only; no general theorem, UI completion, or whole-library completion claim',lawCases,smoothness,spatial:spatial.map(c=>({file:c.file,boundary:c.boundaryError,nativeSolid:true,stationContinuity:c.nativeProfileSmoothness.stationContinuity}))},null,2)+'\n')
console.log(`${lawCases.length} joint-law cases and ${spatial.length} spatial holonomy cases verified; ${smoothness.length} complete scoped seam ownership audits`)
