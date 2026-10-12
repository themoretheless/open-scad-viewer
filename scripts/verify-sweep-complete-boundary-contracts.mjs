import {readSweepManifest} from './sweep-manifest-reader.mjs'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
const [manifestPath,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&outputPath,'Usage: manifest.json complete-boundary-contracts.json')
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes)
const cases=manifest.cases.map(checkCompleteBoundaryContract).filter(Boolean)
const covered=new Set(cases.map(c=>c.file))
const open=manifest.cases.find(c=>c.construction?.boundaryCertificate?.closed===false)
assert.ok(open,'Missing open union-bound control')
const controls=[]
for(const [name,alter] of [
 ['missing filled-cap premise',b=>{b.filledCapErrorUpper=null}],
 ['one filled cap omitted',b=>{b.filledCapErrorUpper.pop()}],
 ['cap bound outside claimed union',b=>{b.filledCapErrorUpper[0]=b.errorUpper+1}],
 ['wall-only scope promoted',b=>{b.scope='retained-patches-relative-to-original-profile-transport'}],
 ['unproved union promoted',b=>{b.continuousBound=false}],
]){
 const item=structuredClone(open);alter(item.construction.boundaryCertificate)
 assert.throws(()=>checkCompleteBoundaryContract(item));controls.push({name,refused:true})
}
const nested=manifest.cases.find(c=>c.boundaryError)
assert.ok(nested,'Missing node-owned spatial boundary control')
for(const [name,alter] of [
 ['missing exact owner node',c=>{c.boundaryError.nodeId='missing-owner'}],
 ['viewport upper rebound to another report',c=>{c.boundaryError.errorUpper+=1}],
 ['original owner evidence removed',c=>{delete c.construction[c.boundaryError.nodeId].bodyBoundaryEvidence}],
]){
 const item=structuredClone(nested);alter(item)
 assert.throws(()=>checkCompleteBoundaryContract(item));controls.push({name,refused:true})
}
const sha=b=>createHash('sha256').update(b).digest('hex')
writeFileSync(outputPath,JSON.stringify({schema:'sweep-complete-boundary-contracts/1',
 scope:'Captured constructor-owned complete-boundary report composition; not new geometric proof or all-input coverage.',
 manifestSha256:sha(bytes),artifactProvenance:manifest.artifactProvenance,
 verifierSha256:sha(readFileSync(new URL('./sweep-complete-boundary-contracts.mjs',import.meta.url))),
 controlVerifierSha256:sha(readFileSync(new URL(import.meta.url))),cases,
 certified:cases.filter(c=>c.status==='certified').length,unproved:cases.filter(c=>c.status==='unproved').length,
 unclassified:manifest.cases.filter(c=>!covered.has(c.file)).map(c=>c.file),controls,passed:true},null,2)+'\n')
console.log(`${cases.filter(c=>c.status==='certified').length} certified boundary owners; ${cases.filter(c=>c.status==='unproved').length} unproved; ${controls.length} incomplete/promoted reports refused`)
