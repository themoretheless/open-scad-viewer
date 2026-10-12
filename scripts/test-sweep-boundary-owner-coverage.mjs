// Actual captured-native receipt controls; no JS geometry proof.
import {readFileSync,writeFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs'
import {join,resolve} from 'node:path'
import {tmpdir} from 'node:os'
import {spawnSync} from 'node:child_process'
import assert from 'node:assert/strict'
const [receiptPath,stepDirectory]=process.argv.slice(2)
assert.ok(receiptPath&&stepDirectory)
const original=JSON.parse(readFileSync(receiptPath)),directory=mkdtempSync(join(tmpdir(),'sweep-owner-controls-'))
const inventory={artifactProvenance:original.artifactProvenance,cases:original.cases.map(c=>({file:c.file,sha256:c.stepSha256}))}
const run=receipt=>{
 writeFileSync(join(directory,'receipt.json'),JSON.stringify(receipt))
 return spawnSync(process.execPath,[resolve('scripts/verify-sweep-boundary-owner-coverage.mjs'),join(directory,'manifest.json'),join(directory,'coverage.json'),join(directory,'receipt.json')],{encoding:'utf8'})
}
try {
 for(const c of original.cases)copyFileSync(resolve(stepDirectory,c.file),join(directory,c.file))
 writeFileSync(join(directory,'manifest.json'),JSON.stringify(inventory))
 const positive=run(original);assert.equal(positive.status,0,positive.stderr)
 const coverage=JSON.parse(readFileSync(join(directory,'coverage.json')))
 assert.equal(coverage.certified,original.certified);assert.equal(coverage.unproved,original.unproved);assert.deepEqual(coverage.unclassified,[])
 for(const [name,mutate] of [
  ['cross-artifact',r=>{r.artifactProvenance.geometryWasmSha256='0'.repeat(64)}],
  ['duplicate-owner',r=>{r.cases.push(r.cases[0])}],
  ['unknown-owner',r=>{r.cases[0].file='missing.step'}],
  ['changed-step-identity',r=>{r.cases[0].stepSha256='0'.repeat(64)}],
  ['forged-owner-outcome',r=>{r.cases[0].result.upper=1}],
  ['failed-supplement',r=>{r.passed=false}],
 ]){
  const receipt=structuredClone(original);mutate(receipt)
  assert.notEqual(run(receipt).status,0,name+' accepted');console.log(name+': refused')
 }
 writeFileSync(join(directory,original.cases[0].file),'changed STEP bytes')
 assert.notEqual(run(original).status,0,'Changed actual STEP accepted')
 console.log('changed actual STEP: refused; valid captured-native receipt: accepted')
} finally {rmSync(directory,{recursive:true,force:true})}
