import assert from 'node:assert/strict'

// Audit the captured Rust pair inventory, including compressed disjoint groups.
// This checks complete enumeration; geometric certificates remain Rust-owned.
export function checkCompleteFacePairCover(report,faces){
 assert.ok(Number.isSafeInteger(faces)&&faces>0)
 const total=faces*(faces-1)/2
 assert.equal(report.allPairsClassified,true,'Face-pair classification unresolved')
 assert.equal(report.nextPair,null,'Face-pair enumeration incomplete')
 assert.equal(report.totalPairs,total,'Face-pair total must cover the actual final model')
 assert.equal(report.individualPairs,report.pairs.length,'Individual pair count mismatch')
 const covered=new Set()
 const add=(a,b)=>{
  assert.ok(Number.isSafeInteger(a)&&Number.isSafeInteger(b)&&a>=0&&a<b&&b<faces,'Invalid canonical face pair')
  const key=a*faces+b
  assert.ok(!covered.has(key),'Duplicate face-pair coverage')
  covered.add(key)
 }
 for(const pair of report.pairs){
  assert.ok(Array.isArray(pair.faces)&&pair.faces.length===2,'Invalid face-pair identity')
  add(...pair.faces)
 }
 let grouped=0
 for(const group of report.disjointGroups){
  assert.ok(Array.isArray(group.range)&&group.range.length===2,'Invalid face-pair group range')
  const [start,end]=group.range
  assert.ok(Number.isSafeInteger(start)&&Number.isSafeInteger(end)&&start<end&&start>group.face&&end<=faces,'Invalid face-pair group extent')
  for(let b=start;b<end;b++)add(group.face,b)
  grouped+=end-start
 }
 assert.equal(report.groupedPairs,grouped,'Grouped face-pair count mismatch')
 assert.equal(report.individualPairs+grouped,total,'Missing face-pair coverage')
 assert.equal(covered.size,total,'Missing unique face-pair coverage')
 return {total,individual:report.individualPairs,grouped}
}
