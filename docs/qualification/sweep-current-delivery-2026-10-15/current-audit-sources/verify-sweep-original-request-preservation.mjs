import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {readSweepManifest} from './sweep-manifest-reader.mjs';
import {checkOriginalRequestPreservation} from './sweep-original-request-preservation.mjs';
import {checkAutomaticCapSourcePreservation} from './sweep-automatic-cap-preservation.mjs';
import {smoothCapGapCompanions} from './sweep-automatic-cap-contracts.mjs';
const [manifestPath,baselinePath,output,annularBaselinePath]=process.argv.slice(2);assert.ok(manifestPath&&baselinePath&&output);
const sha=b=>createHash('sha256').update(b).digest('hex'),bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes),baselineBytes=readFileSync(baselinePath),baseline=JSON.parse(baselineBytes);
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
 return annularBaseline&&smoothCapGapCompanions.has(previous.file)
  ?checkAutomaticCapSourcePreservation(matches[0],previous,annularBaseline.cases.find(c=>c.file===previous.file))
  :checkOriginalRequestPreservation(matches[0],previous);
});
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
  assert.throws(()=>checkAutomaticCapSourcePreservation(changed,previous,original));controls.push({name,refused:true});
 }
}
const geometryReviewRequired=cases.filter(c=>!c.capturedRetainedGeometryPreserved&&!c.sourceWallsAndBoundaryEdgesPreserved),passed=geometryReviewRequired.length===0;
writeFileSync(output,JSON.stringify({schema:'sweep-original-request-preservation/1',passed,recordedAt:new Date().toISOString(),manifestSha256:sha(bytes),baselineSha256:sha(baselineBytes),referenceManifestSha256:baseline.sourceManifestSha256,verifierSha256:sha(readFileSync(new URL(import.meta.url))),coreVerifierSha256:sha(readFileSync(new URL('./sweep-original-request-preservation.mjs',import.meta.url))),cases,controls,geometryReviewRequired,scope:'Exact original157 source identities where captured, explicit native audit budgets/tolerances/topology, and exported retained wall/edge/loop/pcurve/orientation fingerprints. No historical proof outcome reuse. Three explicitly declared Auto cap upgrades, when present, replace two legacy planar caps by eight radial patches while retaining exact original walls and boundary edges; they are reported separately from unchanged models. Missing historical source request fields and vertex arrays are not invented; source-code review/native original-bound proof and independent current OCCT remain separate.'},null,2)+'\n');
assert.equal(passed,true,'Original retained geometry fingerprint drift requires investigation; no automatic acceptance');
console.log(`${cases.filter(c=>c.capturedRetainedGeometryPreserved).length} original retained models preserved;${cases.filter(c=>c.sourceWallsAndBoundaryEdgesPreserved).length} explicit native cap upgrades preserve original walls/edges;${controls.length} mutation controls refused`);
