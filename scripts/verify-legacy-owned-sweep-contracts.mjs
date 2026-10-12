import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
import {bindStepOracle} from './sweep-step-oracle-binding.mjs'

const [directory]=process.argv.slice(2)
assert.ok(directory,'Usage: legacy-owned STEP directory')
const bytes=readFileSync(resolve(directory,'manifest.json')),manifest=JSON.parse(bytes)
const oracle=JSON.parse(readFileSync(resolve(directory,'opencascade-sweep.json')))
const binding=await bindStepOracle(manifest,createHash('sha256').update(bytes).digest('hex'),oracle)
const missing=new Set(['open-straight-hollow-miter.step','open-spatial-hollow-miter.step',
 'open-spatial-bounded-correction.step','open-spatial-constructor-correction.step',
 'closed-planar-hollow-miter.step','affine-oblique-hollow-miter.step','affine-oblique-dyadic-hollow.step'])
assert.equal(manifest.cases.length,9,'Seven legacy inputs and two companion controls required')
const cases=manifest.cases.map(item=>{
 const bound=checkCompleteBoundaryContract(item)
 assert.ok(bound,'Missing whole-boundary native receipt')
 const path=resolve(directory,item.file)
 assert.equal(createHash('sha256').update(readFileSync(path)).digest('hex'),item.sha256)
 if(missing.has(item.file)){
  missing.delete(item.file)
  const owner=item.construction
  assert.equal(owner.legacyModelBound,true)
  assert.equal(owner.sourceRequest.direct,true)
  assert.equal(owner.proofPolicyScope,'explicit-qualification-policy-not-an-inferred-author-tolerance')
  assert.equal(owner.sourceRequest.maxDeviation,.01)
  assert.equal(bound.budget,.01)
  if(item.file==='open-spatial-hollow-miter.step')assert.equal(bound.status,'unproved')
  else assert.equal(bound.status,'certified')
 }
 const external=oracle.cases.find(c=>c.file===item.file)
 assert.equal(external?.passed,true);assert.equal(external.stepSha256,item.sha256)
 // E never promotes a globally refused raw body to a Solid.
 if(['open-spatial-hollow-miter.step','affine-oblique-hollow-miter.step'].includes(item.file))assert.equal(item.nativeVolume.solidGeometryCertified,false)
 else assert.equal(item.nativeVolume.solidGeometryCertified,true)
 const bad=structuredClone(item)
 bad.construction.boundaryCertificate.continuousBound=!bad.construction.boundaryCertificate.continuousBound
 assert.throws(()=>checkCompleteBoundaryContract(bad))
 return {file:item.file,bound,globalSolid:item.nativeVolume.solidGeometryCertified}
})
assert.equal(missing.size,0)
writeFileSync(resolve(directory,'legacy-owned-contracts.json'),JSON.stringify({scope:'Seven formerly missing legacy owners plus two controls; explicit qualification policy, no inferred author tolerance or universal coverage',binding,cases,passed:true},null,2)+'\n')
console.log('Seven legacy owners and two controls reconciled')
