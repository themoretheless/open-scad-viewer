import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'

const output=process.argv[2],baseline=process.argv[3]
assert.ok(output,'Usage: output.json [baseline.json]')
const files=['arc-length-curved-guided-body-boundary.r','arc-length-corrected-frenet-inflection-hollow-body-boundary.r']
const cases=[]
for(const file of files){
 const source=readFileSync(resolve('examples/rush',file),'utf8')
 const document=compileRushFrontend(source).document
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 assert.ok(built.nativeGeometry,'Missing original retained body')
 const model=readNativeGeometryData(built.nativeGeometry).geometry
 const evidence=readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)
 assert.equal(evidence?.continuousBound,true)
 assert.equal(evidence?.withinBudget,true)
 const modelJson=JSON.stringify(model)
 cases.push({file,sourceSha256:createHash('sha256').update(source).digest('hex'),model,
  modelSha256:createHash('sha256').update(modelJson).digest('hex')})
}
if(baseline){
 const previous=JSON.parse(readFileSync(baseline,'utf8')) as {cases:typeof cases}
 assert.deepEqual(cases.map(c=>c.file),previous.cases.map(c=>c.file),'Changed preservation inventory')
 for(const [i,item] of cases.entries()){
  assert.equal(item.sourceSha256,previous.cases[i].sourceSha256,'Changed original source')
  assert.deepEqual(item.model,previous.cases[i].model,'Original retained body geometry changed')
  assert.equal(item.modelSha256,previous.cases[i].modelSha256,'Original retained model bytes changed')
 }
}
writeFileSync(output,JSON.stringify({scope:'Two unchanged original Rush bodies; exact retained model comparison',
 artifactProvenance:sweepStepArtifactProvenance(),baseline:baseline??null,cases,passed:true},null,2)+'\n')
console.log(`${cases.length} original retained bodies captured${baseline?' and matched exactly':''}`)
