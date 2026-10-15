import assert from 'node:assert/strict'

// Preserve captured native whole-boundary scope rather than a wall-only flag.
// This validates report composition; it does not recompute geometry in JS.
export function checkCompleteBoundaryContract(item){
 let construction=item.construction
 if(item.boundaryError){
  const node=item.boundaryError.nodeId
  assert.ok(typeof node==='string'&&Object.hasOwn(construction,node),'Missing exact boundary owner node')
  construction=construction[node]
  assert.ok(construction.bodyBoundaryEvidence,'Missing original constructor body-boundary evidence')
  assert.equal(item.boundaryError.errorUpper,construction.bodyBoundaryEvidence.boundaryErrorUpper,'Viewport bound differs from constructor owner')
  assert.equal(item.boundaryError.budget,construction.bodyBoundaryEvidence.budget,'Viewport budget differs from constructor owner')
 }
 if(item.boundaryCertificate&&construction?.boundaryCertificate){
  assert.deepEqual(item.boundaryCertificate,construction.boundaryCertificate,'Conflicting constructor boundary reports')
 }
 const miter=construction?.boundaryCertificate??item.boundaryCertificate,body=construction?.bodyBoundaryEvidence
 if(!miter&&!body)return null
 const bound=miter??body,closed=miter?bound.closed:bound.closedPath
 const upper=miter?bound.errorUpper:bound.boundaryErrorUpper
 const complete=miter?bound.continuousBound:bound.boundaryContinuousBound
 const within=miter?bound.withinBudget:bound.boundaryErrorWithinBudget
 assert.equal(typeof closed,'boolean',`${item.file}: missing boundary closure`)
 assert.ok(Number.isFinite(bound.budget)&&bound.budget>0)
 if(complete===false){
  assert.equal(upper,null,`${item.file}: unproved boundary cannot publish an upper bound`)
  assert.equal(within,null,`${item.file}: unproved boundary cannot publish a budget decision`)
  assert.equal(miter?bound.scope:bound.boundaryErrorScope,miter?'boundary-set-hausdorff':'constructor-owned-retained-wall-and-cap-union')
  return {file:item.file,owner:miter?'miter-boundary-union':'progressive-body-boundary-union',status:'unproved',closed,upper:null,budget:bound.budget}
 }
 assert.equal(complete,true,`${item.file}: whole boundary unproved`)
 assert.equal(within,true,`${item.file}: whole boundary outside budget`)
 assert.ok(Number.isFinite(upper)&&upper>=0&&upper<=bound.budget)
 if(miter){
  assert.equal(bound.method,'retained-sweep-boundary-union')
  assert.equal(bound.scope,'boundary-set-hausdorff')
  assert.equal(bound.reason,null)
  assert.ok(Number.isFinite(bound.wallErrorUpper)&&bound.wallErrorUpper>=0&&bound.wallErrorUpper<=upper,'Uncovered wall bound')
 }else assert.equal(bound.boundaryErrorScope,'constructor-owned-retained-wall-and-cap-union')
 if(closed){
  assert.equal(item.capFaces.length,0,'Closed boundary must have no fabricated caps')
  assert.equal(bound.filledCapErrorUpper,null)
  if(body?.capRepresentation!==undefined){
   assert.equal(body.capRepresentation,'closed')
   assert.deepEqual(body.capFaces,[])
   assert.deepEqual(body.capFaceGroups,[[],[]])
   assert.deepEqual(body.planarCapFaces,[])
   assert.equal(body.retainedAnnularFilling,null)
   assert.equal(body.capSelectionExactWork,0)
  }
 }else{
  if(body?.capRepresentation==='radial-annular'){
   const groups=body.capFaceGroups,physical=body.capFaces,filling=body.retainedAnnularFilling
   assert.ok(Array.isArray(groups)&&groups.length===2&&groups.every(g=>Array.isArray(g)&&g.length>0),'Missing native physical endpoint groups')
   assert.deepEqual(groups.flat(),physical,'Native physical cap mask differs from endpoint groups')
   assert.equal(new Set(physical).size,physical.length,'Duplicate physical cap face')
   assert.ok(physical.every(i=>Number.isInteger(i)&&i>=0&&(!Number.isInteger(item.faces)||i<item.faces)),'Physical cap outside actual model')
   assert.deepEqual(body.planarCapFaces,[],'Radial cap cannot use planar optimization')
   assert.deepEqual(item.capFaces,[],'STEP radial patches must be included in the surface audit')
   if(item.physicalCapFaces)assert.deepEqual(item.physicalCapFaces,physical)
   assert.equal(filling?.sourceFillingCovered,true,'Original filled cap domain not covered')
   assert.equal(filling.sourceUvFacesInjective,true)
   assert.equal(filling.sourceUvPairsClassified,true)
   assert.equal(filling.withinBudget,true)
   assert.deepEqual(filling.filledCapErrorUpper,body.filledCapErrorUpper)
   assert.equal(filling.selectionExactWork,body.capSelectionExactWork)
   assert.ok([filling.selectionExactWork,filling.constructionExactWork,filling.fillingExactWork]
    .every(work=>Number.isSafeInteger(work)&&work>=0),'Invalid shared cap-work ledger')
   assert.equal(filling.exactWork,filling.selectionExactWork+filling.constructionExactWork+filling.fillingExactWork)
  }else assert.equal(item.capFaces.length,2,'Open boundary requires both retained caps')
  const caps=bound.filledCapErrorUpper
  assert.ok(Array.isArray(caps)&&caps.length===2,'Missing filled-cap bound')
  assert.ok(caps.every(e=>Number.isFinite(e)&&e>=0&&e<=upper),'Filled cap exceeds the claimed union bound')
 }
 return {file:item.file,owner:miter?'miter-boundary-union':'progressive-body-boundary-union',status:'certified',closed,upper,budget:bound.budget}
}
