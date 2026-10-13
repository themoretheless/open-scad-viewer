import assert from 'node:assert/strict'

// Admission of captured Rust reports only; no geometric proof is performed here.
export function checkRetainedAxisAudit(axis,g2,g1,certifiedG1,certifiedG2){
 assert.equal(axis.extractionComplete,true);assert.deepEqual(axis.unclassifiedFaces,[]);assert.deepEqual(axis.unpairedEdges,[])
 assert.equal(new Set(axis.edgeIds).size,axis.edgeIds.length)
 assert.ok(Number.isSafeInteger(axis.exactWork)&&axis.exactWork>=0&&axis.exactWork<=axis.maxWork)
 assert.equal(axis.exactWork,g2.exactWork+(g1?.exactWork??0))
 assert.equal(certifiedG2,g2.exactG1G2Certified)
 assert.equal(certifiedG1,certifiedG2||g1?.exactG1G2Certified===true)
 for(const collection of [g2,g1])if(collection){
  assert.equal(collection.seams.length,axis.edgeIds.length,'Every owned retained seam needs an audit outcome')
  assert.ok(collection.seams.every(seam=>seam.reason!=='seam-audit-skipped-after-refusal'),'A sharp first seam must not hide later applicable retained seams')
  assert.deepEqual(collection.unresolvedSeams,collection.seams.flatMap((seam,index)=>seam.certified?[]:[index]),'Unresolved retained seam coverage differs from actual outcomes')
  for(const seam of collection.seams)if(seam.certified){
   assert.equal(seam.exactIdentity,true,'Positive retained seam requires exact identity')
   assert.equal(seam.regularityCertified,true,'Positive retained seam requires regularity')
  }
 }
 for(const [collection,order] of [[g2,2],[g1,1]])if(collection?.exactG1G2Certified){
  assert.equal(collection.certifiedOrder,order);assert.deepEqual(collection.unresolvedSeams,[])
  assert.equal(collection.seams.length,axis.edgeIds.length)
  assert.ok(collection.seams.length>0)
  assert.ok(collection.seams.every(seam=>seam.certified&&seam.exactIdentity&&seam.regularityCertified))
 }
}
