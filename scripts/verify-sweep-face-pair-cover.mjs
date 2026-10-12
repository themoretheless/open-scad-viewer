import {readSweepManifest} from './sweep-manifest-reader.mjs'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'

const [manifestPath,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&outputPath,'Usage: manifest.json pair-cover.json')
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes),cases=[]
for(const item of manifest.cases){
 if(!item.nativeBoundary?.boundaryEmbeddingCertified)continue
 cases.push({file:item.file,...checkCompleteFacePairCover(item.nativeBoundary,item.faces)})
}
const item=manifest.cases.find(c=>c.nativeBoundary?.boundaryEmbeddingCertified&&c.nativeBoundary.pairs.length)
assert.ok(item,'Missing certified individual-pair inventory')
const controls=[]
for(const [name,alter] of [
 ['missing final-model pair',r=>{r.pairs.pop();r.individualPairs--}],
 ['duplicated canonical pair',r=>{r.pairs.push(r.pairs[0]);r.individualPairs++}],
 ['incorrect final-model total',r=>{r.totalPairs--}],
 ['unvisited pair falsely promoted',r=>{r.nextPair=[0,1]}],
 ['invalid pair identity',r=>{r.pairs[0].faces.push(0)}],
]){
 const report=structuredClone(item.nativeBoundary);alter(report)
 assert.throws(()=>checkCompleteFacePairCover(report,item.faces))
 controls.push({name,refused:true})
}
const grouped=manifest.cases.find(c=>c.nativeBoundary?.boundaryEmbeddingCertified&&c.nativeBoundary.disjointGroups.length)
assert.ok(grouped,'Missing certified compressed-pair inventory')
const report=structuredClone(grouped.nativeBoundary),group=report.disjointGroups[0]
report.disjointGroups.push(structuredClone(group));report.groupedPairs+=group.range[1]-group.range[0]
assert.throws(()=>checkCompleteFacePairCover(report,grouped.faces))
controls.push({name:'duplicated compressed group',refused:true})
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
writeFileSync(outputPath,JSON.stringify({schema:'sweep-face-pair-cover/1',
 scope:'Complete captured final-model face-pair enumeration; not independent geometric intersection proof.',
 manifestSha256:sha(bytes),artifactProvenance:manifest.artifactProvenance,
 verifierSha256:sha(readFileSync(new URL('./sweep-face-pair-cover.mjs',import.meta.url))),
 controlVerifierSha256:sha(readFileSync(new URL(import.meta.url))),cases,controls,passed:true},null,2)+'\n')
console.log(`${cases.length} captured complete pair covers; ${controls.length} altered inventories refused`)
