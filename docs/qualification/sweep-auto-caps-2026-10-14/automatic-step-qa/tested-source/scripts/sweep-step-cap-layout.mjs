import assert from 'node:assert/strict'

// Transport the native selection. capFaces is the existing STEP oracle's
// planar-region mask; physicalCapFaces also includes rational radial patches.
export function sweepStepCapLayout(body){
 if(body.capRepresentation===undefined)return null
 const {capRepresentation,capFaceGroups,capFaces,planarCapFaces,model}=body
 assert.ok(['legacy','closed','radial-annular'].includes(capRepresentation),'Unknown native cap representation')
 assert.ok(Array.isArray(capFaceGroups)&&capFaceGroups.length===2,'Missing native endpoint groups')
 assert.ok(capFaceGroups.every(Array.isArray),'Invalid native endpoint group')
 assert.deepEqual(capFaceGroups.flat(),capFaces,'Physical cap mask differs from native endpoint groups')
 assert.equal(new Set(capFaces).size,capFaces.length,'Duplicate physical cap face')
 assert.ok(capFaces.every(i=>Number.isInteger(i)&&i>=0&&i<model.faces.length),'Physical cap face outside actual model')
 assert.ok(Array.isArray(planarCapFaces),'Missing native planar mask')
 if(capRepresentation==='closed'){
  assert.equal(body.approximation.report.closedPath,true)
  assert.deepEqual(capFaces,[])
  assert.deepEqual(planarCapFaces,[])
 }else{
  assert.equal(body.approximation.report.closedPath,false)
  assert.ok(capFaceGroups.every(group=>group.length>0),'Missing physical endpoint cap')
  if(capRepresentation==='legacy'){
   assert.ok(capFaceGroups.every(group=>group.length===1))
   assert.deepEqual(planarCapFaces,capFaces)
  }else{
   assert.deepEqual(planarCapFaces,[],'Radial cap cannot use a planar-region shortcut')
   const filling=body.retainedAnnularFilling
   assert.ok(filling,'Missing native radial filling report')
   assert.equal(filling.selectionExactWork,body.capSelectionExactWork)
   assert.ok([filling.selectionExactWork,filling.constructionExactWork,filling.fillingExactWork]
    .every(work=>Number.isSafeInteger(work)&&work>=0),'Invalid shared cap-work ledger')
   assert.equal(filling.exactWork,filling.selectionExactWork+filling.constructionExactWork+filling.fillingExactWork)
  }
 }
 return {capRepresentation,physicalCapFaces:[...capFaces],capFaceGroups:capFaceGroups.map(group=>[...group]),
  capFaces:[...planarCapFaces],capHoleFaces:model.faces.filter(face=>face.holes.length>0).length}
}
