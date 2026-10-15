import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {readSweepManifest} from './sweep-manifest-reader.mjs';
import {checkOriginalRequestPreservation} from './sweep-original-request-preservation.mjs';
import {checkAutomaticCapSourcePreservation} from './sweep-automatic-cap-preservation.mjs';
import {smoothCapGapCompanions} from './sweep-automatic-cap-contracts.mjs';
import {loadGeometryMigration} from './sweep-geometry-migration.mjs';
const [manifestPath,baselinePath,output,annularBaselinePath,migrationReviewPath]=process.argv.slice(2);assert.ok(manifestPath&&baselinePath&&output);
const sha=b=>createHash('sha256').update(b).digest('hex'),bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes),baselineBytes=readFileSync(baselinePath),baseline=JSON.parse(baselineBytes);
const migration=migrationReviewPath?loadGeometryMigration(migrationReviewPath,{manifestSha256:sha(bytes),baselineSha256:sha(baselineBytes),baseline,producerPath:new URL('../crates/brep-core/examples/certify_sweep_geometry_migration.rs',import.meta.url)}):null;
assert.equal(baseline.cases.length,157);assert.equal(manifest.cases.length,207);assert.equal(new Set(manifest.cases.map(c=>c.file)).size,207);
let annularBaseline=null;
if(manifest.automaticSmoothCaps===true){
 assert.ok(annularBaselinePath,'Automatic cap upgrades require the independently captured original wall/edge baseline');
 annularBaseline=JSON.parse(readFileSync(annularBaselinePath));
 assert.equal(annularBaseline.sourceManifestSha256,baseline.sourceManifestSha256);
 assert.deepEqual(annularBaseline.cases.map(c=>c.file).sort(),[...smoothCapGapCompanions.keys()].sort());
}
const cases=baseline.cases.map(previous=>{
 const matches=manifest.cases.filter(c=>c.file===previous.file);assert.equal(matches.length,1);
 const item=matches[0],review=migration?.has(item.file)?migration.check(item):null;
 if(annularBaseline&&smoothCapGapCompanions.has(previous.file))return checkAutomaticCapSourcePreservation(item,previous,annularBaseline.cases.find(c=>c.file===previous.file),review);
 const result=checkOriginalRequestPreservation(item,previous);
 if(review){
  assert.ok(result.geometryChanges.length>0,'Unchanged model must not use a rounding migration exception');
  assert.ok(result.geometryChanges.every(c=>c.field==='wallSurfaces'||c.field==='edgeCurves'),'Migration cannot cover topology, pcurves or orientation changes');
  return {...result,geometryMigration:review,reviewedRetainedEvaluationMigration:true};
 }
 return result;
});
if(migration)assert.deepEqual(cases.filter(c=>c.reviewedRetainedEvaluationMigration).map(c=>c.file).sort(),migration.files.sort(),'Unused or missing migration proof');
const source=baseline.cases.find(c=>c.requestAndTopology.sourceSha256),budget=baseline.cases.find(c=>c.requestAndTopology.nativeAuditBudgets);
const controls=[];
for(const [name,previous,mutate]of [['source identity changed',source,c=>{c.sourceSha256='0'.repeat(64)}],['proof budget increased',budget,c=>{c.nativeAuditBudgets.maxCells+=1}],['geometric tolerance increased',budget,c=>{c.surfaceToleranceMm*=2}],['retained topology simplified',budget,c=>{c.faces-=1}]]){
 const current=structuredClone(manifest.cases.find(c=>c.file===previous.file));mutate(current);assert.throws(()=>checkOriginalRequestPreservation(current,previous));controls.push({name,refused:true});
}
if(annularBaseline){
 const original=annularBaseline.cases[0],previous=baseline.cases.find(c=>c.file===original.file);
 const item=manifest.cases.find(c=>c.file===original.file);
 for(const [name,mutate] of [
  ['original wall removed',c=>{c.wallSurfaces.shift()}],
  ['original boundary edge changed',c=>{c.edgeCurves[0].curve.controlPoints[0][0]+=1}],
  ['native proof budget increased',c=>{c.nativeAuditBudgets.maxCells+=1}],
  ['physical endpoint cap omitted',c=>{c.physicalCapFaces.pop()}],
  ['actual native Solid unproved',c=>{c.nativeVolume.solidGeometryCertified=false}],
 ]){
  const changed=structuredClone(item);mutate(changed);
  assert.throws(()=>checkAutomaticCapSourcePreservation(changed,previous,original,migration?.has(changed.file)?migration.check(changed):null));controls.push({name,refused:true});
 }
}
if(migration){
 const item=manifest.cases.find(c=>migration.has(c.file)&&migration.check(c).correspondenceUpperMm>0);
 assert.ok(item,'Reviewed rounding changes require an actual nonzero displacement case');
 migration.check(item);
 for(const [name,mutate] of [
  ['current wall differs from native migration input',c=>{c.wallSurfaces[0].controlPoints[0][0][0]+=1}],
  ['current edge differs from native migration input',c=>{c.edgeCurves[0].curve.controlPoints[0][0]+=1}],
  ['wall omitted from native migration coverage',c=>{c.wallSurfaces.shift()}],
  ['extra wall omitted from native migration coverage',c=>{c.wallSurfaces.push(c.wallSurfaces[0])}],
  ['current boundary owner differs from composed native premise',c=>{
   if(c.boundaryError)c.boundaryError.errorUpper+=1;
   else if(c.construction.bodyBoundaryEvidence)c.construction.bodyBoundaryEvidence.boundaryErrorUpper+=1;
   else (c.construction.boundaryCertificate??c.boundaryCertificate).errorUpper+=1;
  }],
 ]){
  const changed=structuredClone(item);mutate(changed);assert.throws(()=>migration.check(changed));controls.push({name,refused:true});
 }
}
const geometryReviewRequired=cases.filter(c=>!c.capturedRetainedGeometryPreserved&&!c.sourceWallsAndBoundaryEdgesPreserved&&!c.reviewedRetainedEvaluationMigration),passed=geometryReviewRequired.length===0;
writeFileSync(output,JSON.stringify({schema:'sweep-original-request-preservation/2',passed,recordedAt:new Date().toISOString(),manifestSha256:sha(bytes),baselineSha256:sha(baselineBytes),referenceManifestSha256:baseline.sourceManifestSha256,verifierSha256:sha(readFileSync(new URL(import.meta.url))),coreVerifierSha256:sha(readFileSync(new URL('./sweep-original-request-preservation.mjs',import.meta.url))),migrationProvenance:migration?.provenance??null,cases,controls,geometryReviewRequired,scope:'Original157 source identities where captured, unchanged explicit native budgets/tolerances and retained topology. Exact retained fingerprints remain mandatory by default. An explicitly supplied rounding review requires native full-domain coefficient correspondence bound to both historical fingerprints and current arrays, composed within the original budget; these cases are never reported as exact model equality. Three declared Auto upgrades replace two planar caps with eight radial patches under separate current native cap/global/STEP checks. No historical proof outcome reuse or invented missing source/vertex data.'},null,2)+'\n');
assert.equal(passed,true,'Original retained geometry fingerprint drift requires investigation; no automatic acceptance');
console.log(`${cases.filter(c=>c.capturedRetainedGeometryPreserved).length} exact original retained models;${cases.filter(c=>c.sourceWallsAndBoundaryEdgesPreserved).length} cap upgrades retain exact walls/edges;${cases.filter(c=>c.reviewedRetainedEvaluationMigration&&c.geometryMigration.correspondenceUpperMm>0).length} native bounded rounding migrations;${controls.length} mutation controls refused`);
