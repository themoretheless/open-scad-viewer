import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const canonical=v=>v===null||typeof v!=='object'?v:Array.isArray(v)?v.map(canonical):Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]));
const hashValue=v=>sha(Buffer.from(JSON.stringify(canonical(v))));
export function checkOriginalRequestPreservation(item,baseline){
 assert.equal(item.file,baseline.file);
 const actual=Object.fromEntries(Object.keys(baseline.requestAndTopology).map(k=>[k,item[k]??null]));
 assert.deepEqual(actual,baseline.requestAndTopology,`${item.file}: original source, budgets, tolerances or topology changed`);
 const budget=item.boundaryError?.budget??item.construction?.boundaryCertificate?.budget??null;
 if(baseline.completeBoundaryBudget!==null)assert.equal(budget,baseline.completeBoundaryBudget,`${item.file}: original continuous budget changed`);
 const geometryChanges=[];
 for(const [field,previousSha256]of Object.entries(baseline.retainedGeometryFingerprints)){
  const value=field==='edgeCurves'?item.edgeCurves?.map(e=>e.curve)??null:item[field]??null;
  const currentSha256=hashValue(value);if(currentSha256!==previousSha256)geometryChanges.push({field,previousSha256,currentSha256});
 }
 return {file:item.file,sourceBound:baseline.requestAndTopology.sourceSha256!==null,continuousBudgetPreviouslyAvailable:baseline.completeBoundaryBudget!==null,requestAndTopologyPreserved:true,capturedRetainedGeometryPreserved:geometryChanges.length===0,geometryChanges};
}
