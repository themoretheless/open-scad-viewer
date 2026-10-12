import {readSweepManifest} from './sweep-manifest-reader.mjs'
// Reconcile fresh native owners only within one exact artifact and STEP inventory.
// This checks evidence composition; geometry/certificates remain Rust-owned.
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve,dirname} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
const [manifestPath,outputPath,...supplementPaths]=process.argv.slice(2)
assert.ok(manifestPath&&outputPath&&supplementPaths.length,'Usage: manifest output supplements...')
const sha=b=>createHash('sha256').update(b).digest('hex')
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes)
const inventory=new Map(manifest.cases.map(c=>[c.file,c]))
assert.equal(inventory.size,manifest.cases.length,'Duplicate STEP inventory')
const covered=new Map(),receipts=[]
for(const item of manifest.cases){
 const result=checkCompleteBoundaryContract(item)
 if(result)covered.set(item.file,result)
}
for(const path of supplementPaths){
 const data=readFileSync(path),receipt=JSON.parse(data)
 assert.equal(receipt.passed,true,'Failed supplement')
 assert.deepEqual(receipt.artifactProvenance,manifest.artifactProvenance,'Cross-artifact owner transfer')
 const local=new Set()
 for(const item of receipt.cases){
  assert.ok(!local.has(item.file),'Duplicate supplement owner');local.add(item.file)
  const original=inventory.get(item.file)
  assert.ok(original,'Unknown STEP owner')
  assert.ok(!covered.has(item.file),'Duplicate or substituted owner')
  assert.equal(item.stepSha256,original.sha256,'Different captured STEP')
  assert.equal(sha(readFileSync(resolve(dirname(manifestPath),item.file))),original.sha256,'Changed STEP bytes')
  const result=checkCompleteBoundaryContract(item)
  assert.ok(result,'Missing complete owner composition')
  assert.deepEqual(result,item.result,'Changed owner outcome')
  covered.set(item.file,result)
 }
 receipts.push({path,sha256:sha(data),owners:receipt.cases.length})
}
const cases=[...covered.values()]
writeFileSync(outputPath,JSON.stringify({schema:'sweep-boundary-owner-coverage/1',
 scope:'Finite current-artifact original-source boundary owner coverage; not universal error/global/smoothness proof.',
 artifactProvenance:manifest.artifactProvenance,manifestSha256:sha(bytes),
 verifierSha256:sha(readFileSync(new URL(import.meta.url))),receipts,cases,
 certified:cases.filter(c=>c.status==='certified').length,
 unproved:cases.filter(c=>c.status==='unproved').length,
 unclassified:manifest.cases.filter(c=>!covered.has(c.file)).map(c=>c.file),passed:true},null,2)+'\n')
console.log(JSON.stringify({certified:cases.filter(c=>c.status==='certified').length,unproved:cases.filter(c=>c.status==='unproved').length,unclassified:manifest.cases.length-covered.size}))
