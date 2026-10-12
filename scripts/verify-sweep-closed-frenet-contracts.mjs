import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve,dirname} from 'node:path'
import {bindStepOracle} from './sweep-step-oracle-binding.mjs'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'

const [manifestPath,oraclePath,originalStage,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&oraclePath&&originalStage&&outputPath)
const hash=bytes=>createHash('sha256').update(bytes).digest('hex')
const manifestBytes=readFileSync(manifestPath),manifest=JSON.parse(manifestBytes)
const oracle=JSON.parse(readFileSync(oraclePath))
const binding=await bindStepOracle(manifest,hash(manifestBytes),oracle)
assert.equal(oracle.passed,true)
assert.deepEqual(oracle.artifactProvenance,manifest.artifactProvenance)
const names=['circle-solid','circle-hollow','periodic-solid','periodic-hollow']
assert.deepEqual(manifest.cases.map(c=>c.file).sort(),names.map(n=>`closed-frenet-arc-${n}-station-g2.step`).sort())
function axis(a,g2,g1,certifiedG1,certifiedG2){
 assert.equal(a.extractionComplete,true)
 assert.deepEqual(a.unclassifiedFaces,[]);assert.deepEqual(a.unpairedEdges,[])
 assert.equal(new Set(a.edgeIds).size,a.edgeIds.length)
 assert.ok(Number.isSafeInteger(a.exactWork)&&a.exactWork>=0&&a.exactWork<=a.maxWork)
 assert.equal(a.exactWork,g2.exactWork+(g1?.exactWork??0))
 assert.equal(certifiedG2,true);assert.equal(certifiedG1,true)
 assert.equal(g2.exactG1G2Certified,true);assert.equal(g2.certifiedOrder,2)
 assert.deepEqual(g2.unresolvedSeams,[])
 assert.equal(g2.seams.length,a.edgeIds.length);assert.ok(g2.seams.length>0)
 assert.ok(g2.seams.every(s=>s.certified&&s.exactIdentity&&s.regularityCertified))
}
const results=[]
for(const c of manifest.cases){
 const source=readFileSync(resolve(dirname(manifestPath),c.sourceFile))
 assert.equal(hash(source),c.sourceSha256)
 assert.equal(hash(readFileSync(resolve(originalStage,c.sourceFile))),c.sourceSha256,'Original source changed')
 const stepHash=hash(readFileSync(resolve(dirname(manifestPath),c.file)))
 assert.equal(stepHash,c.sha256)
 const external=oracle.cases.find(e=>e.file===c.file)
 assert.equal(external?.passed,true);assert.equal(external.stepSha256,stepHash)
 const periodic=c.file.includes('periodic'),hollow=c.file.includes('hollow')
 assert.equal(c.faces,hollow?256:128);assert.deepEqual(c.capFaces,[])
 const b=c.construction.boundaryCertificate
 assert.equal(b.scope,'boundary-set-hausdorff');assert.equal(b.method,'retained-sweep-boundary-union')
 assert.equal(b.closed,true);assert.equal(b.continuousBound,true);assert.equal(b.withinBudget,true)
 assert.equal(b.budget,periodic?.5:2)
 assert.ok(Number.isFinite(b.errorUpper)&&b.errorUpper>=0&&b.errorUpper<=b.budget)
 assert.equal(c.construction.stationReconstruction.source,'original-progressive-profile-request')
 assert.equal(c.construction.stationReconstruction.closedPath,true)
 assert.equal(c.nativeAuditBudgets.cellsPerPair,1000)
 assert.equal(c.nativeAuditBudgets.domainCellsPerPair,10000)
 assert.equal(c.nativeAuditBudgets.maxDomainCells,1000000)
 checkCompleteBoundaryContract(c);checkCompleteFacePairCover(c.nativeBoundary,c.faces)
 const v=c.nativeVolume
 for(const key of ['allFacesInjective','allPairsClassified','boundaryEmbeddingCertified','solidGeometryCertified'])assert.equal(v[key],true)
 assert.equal(v.nesting.rolesConsistent,true);assert.equal(v.nesting.visitedPairs,v.nesting.totalPairs)
 assert.equal(v.orientations.length,c.shells)
 assert.ok(v.orientations.every(o=>typeof o.outward==='boolean'&&o.outward===o.expectedOutward))
 const charts=c.nativeRetainedWallCharts
 assert.equal(charts.allChartsCertified,true);assert.deepEqual(charts.unresolvedFaces,[])
 assert.deepEqual(charts.charts.map(x=>x.face).sort((a,b)=>a-b),Array.from({length:c.faces},(_,i)=>i))
 assert.ok(charts.charts.every(x=>x.audit.certified===true))
 const p=c.nativeProfileSmoothness,s=p.station
 axis(p,p.profile,p.g1Audit,p.profileG1Certified,p.profile.exactG1G2Certified)
 axis(s,s.g2,s.g1Audit,s.stationG1Certified,s.stationG2Certified)
 assert.equal(p.totalExactWork,p.exactWork+s.exactWork);assert.ok(p.totalExactWork<=p.maxWork)
 assert.equal(p.stationContinuity,'G2');assert.equal(p.capContinuity,'absent')
 assert.equal(p.fullBoundarySmoothnessCertified,false)
 assert.deepEqual(s.capEdges,[])
 const owned=[...p.edgeIds,...s.edgeIds]
 assert.equal(new Set(owned).size,owned.length);assert.equal(owned.length,c.edges)
 results.push({file:c.file,sourceSha256:c.sourceSha256,stepSha256:stepHash,faces:c.faces,bound:b,profile:'G2',station:'G2',freshGlobalSolid:true})
}
const verifierDependencies=Object.fromEntries(['sweep-step-oracle-binding.mjs','sweep-complete-boundary-contracts.mjs','sweep-face-pair-cover.mjs'].map(file=>[file,hash(readFileSync(new URL(file,import.meta.url)))]))
writeFileSync(outputPath,JSON.stringify({schema:'closed-frenet-finite-contracts/1',passed:true,scope:'Four original closed Frenet arc-length fixtures only; not universal coverage or UI/deadline qualification',manifestSha256:hash(manifestBytes),oracleSha256:hash(readFileSync(oraclePath)),verifierSha256:hash(readFileSync(new URL(import.meta.url))),verifierDependencies,originalSourceStage:resolve(originalStage),artifactProvenance:manifest.artifactProvenance,oracleBinding:binding,cases:results},null,2)+'\n')
console.log('4 original closed Frenet STEP, full boundary, global Solid and retained G2 contracts verified')
