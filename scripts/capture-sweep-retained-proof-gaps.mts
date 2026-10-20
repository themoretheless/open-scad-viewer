/** Capture native seam receipts missing from the original-source STEP supplement. */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync} from 'node:fs'
import {resolve} from 'node:path'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {checkRetainedAxisAudit} from './retained-axis-audit-contract.mjs'

const [gatePath,output]=process.argv.slice(2)
assert(gatePath&&output&&!existsSync(output),'Supply a completed STEP gate and a new output file')
const sha=(data:string|Buffer)=>createHash('sha256').update(data).digest('hex')
const gate=JSON.parse(readFileSync(gatePath,'utf8'))
assert.equal(gate.state,'complete');assert.equal(gate.passed,true)
const bytes=readFileSync(gate.manifest);assert.equal(sha(bytes),gate.manifestSha256)
const manifest=JSON.parse(bytes.toString())
const selected=manifest.cases.filter((c:any)=>!c.construction?.profileSmoothness)
assert.equal(selected.length,9,'Do not silently alter the missing native seam receipt inventory')
const cases:any[]=[]
const inputHashes:Record<string,string>={[resolve(gatePath)]:sha(readFileSync(gatePath)),[gate.manifest]:sha(bytes)}
for(const item of selected) {
 console.log('Capture native retained seams:',item.file)
 const source=readFileSync(item.sourceFile,'utf8');assert.equal(sha(source),item.sourceSha256)
 inputHashes[item.sourceFile]=sha(source)
 const mesh=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 assert(mesh.nativeGeometry)
 const data=readNativeGeometryData(mesh.nativeGeometry)
 assert.equal(sha(JSON.stringify(data.geometry)),item.uiModelSha256,'Retained model differs from both qualified UI widths')
 const profile=data.sweepEvidence?.profileSmoothness
 assert(profile,'Missing native retained profile/station receipt')
 checkRetainedAxisAudit(profile,profile.profile,profile.g1Audit,profile.profileG1Certified,profile.profile.exactG1G2Certified)
 const station=profile.station
 checkRetainedAxisAudit(station,station.g2,station.g1Audit,station.stationG1Certified,station.stationG2Certified)
 assert.equal(profile.totalExactWork,profile.exactWork+station.exactWork)
 assert.equal(profile.fullBoundarySmoothnessCertified,false)
 cases.push({file:item.file,sourceSha256:item.sourceSha256,uiModelSha256:item.uiModelSha256,
  profileSmoothness:profile,construction:mesh.report?.construction??null})
}
for(const [file,digest]of Object.entries(inputHashes))assert.equal(sha(readFileSync(file)),digest,file)
writeFileSync(output,JSON.stringify({schema:'sweep-retained-proof-gap-capture/1',passed:true,
 geometryWasmSha256:gate.geometryWasmSha256,manifestSha256:gate.manifestSha256,inputsSha256:inputHashes,
 scriptSha256:sha(readFileSync(new URL(import.meta.url))),cases,
 scope:'Actual native owned profile/station receipts for nine unchanged source/model/UI bindings. C0/refusal remains explicit; no whole-boundary G2 promotion.',fullGoalComplete:false},null,2)+'\n')
console.log('All nine missing native retained seam receipts captured and reconciled.')
