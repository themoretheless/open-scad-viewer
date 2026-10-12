import {readSweepManifest} from './sweep-manifest-reader.mjs'
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {retainedSmoothnessContracts,checkRetainedSmoothnessContract,checkRetainedSmoothnessInventory} from './sweep-retained-smoothness-contracts.mjs'

const [manifestPath,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&outputPath,'Usage: manifest.json refusal-controls.json')
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes)
checkRetainedSmoothnessInventory(manifest.cases)
const controls=[]
for(const [file,required] of Object.entries(retainedSmoothnessContracts)){
 const item=manifest.cases.find(item=>item.file===file)
 checkRetainedSmoothnessContract(item)
 // A self-consistent lower-order report must not erase a declared contract.
 // Keep the captured geometry and all native fields; only replace certificate
 // outcomes as a hypothetical regression, never as new geometry evidence.
 const downgrade=(axis)=>{
  const p=item.nativeProfileSmoothness
  const changed={...item,nativeProfileSmoothness:{...p,
   ...(axis==='profile'?{profileG1Certified:false,profile:{...p.profile,exactG1G2Certified:false}}:
    {stationContinuity:'C0',station:{...p.station,stationG1Certified:false,stationG2Certified:false}})}}
  assert.throws(()=>checkRetainedSmoothnessContract(changed),/declared retained smoothness/)
  controls.push({file,fault:`${axis} certificate downgraded to C0`,refused:true})
 }
 if(required.profile==='G1'||required.profile==='G2')downgrade('profile')
 if(required.station==='G2')downgrade('station')
 if(required.station==='C0'){
  const p=item.nativeProfileSmoothness
  const promoted={...item,nativeProfileSmoothness:{...p,station:{...p.station,stationG1Certified:true,stationG2Certified:true}}}
  assert.throws(()=>checkRetainedSmoothnessContract(promoted),/declared retained smoothness/)
  controls.push({file,fault:'sharp station promoted to G2',refused:true})
 }
 if(required.profile==='C0'||required.profileG2===false){
  const p=item.nativeProfileSmoothness
  const promoted={...item,nativeProfileSmoothness:{...p,profileG1Certified:true,profile:{...p.profile,exactG1G2Certified:true}}}
  assert.throws(()=>checkRetainedSmoothnessContract(promoted),/declared retained smoothness/)
  controls.push({file,fault:'sharp/G1-only profile promoted to G2',refused:true})
 }
 assert.throws(()=>checkRetainedSmoothnessInventory(manifest.cases.filter(item=>item.file!==file)),/Missing declared retained smoothness fixture/)
 controls.push({file,fault:'required fixture omitted',refused:true})
 assert.throws(()=>checkRetainedSmoothnessInventory([...manifest.cases,item]),/Duplicate declared retained smoothness fixture/)
 controls.push({file,fault:'required fixture duplicated',refused:true})
}
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
writeFileSync(outputPath,JSON.stringify({schema:'sweep-retained-smoothness-refusal-controls/1',
 scope:'Mutated captured certificate outcomes and omitted fixtures; contract-admission regression only, not new geometric qualification.',
 manifestSha256:sha(bytes),artifactProvenance:manifest.artifactProvenance,
 contractSourceSha256:sha(readFileSync(new URL('./sweep-retained-smoothness-contracts.mjs',import.meta.url))),
 declaredCases:Object.keys(retainedSmoothnessContracts).length,controls,passed:true},null,2)+'\n')
console.log(`${Object.keys(retainedSmoothnessContracts).length} declared contracts; ${controls.length} downgrade/promotion/omission controls refused`)
