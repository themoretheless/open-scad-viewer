import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
const [planPath,group,output,...inputs]=process.argv.slice(2)
assert.ok(planPath&&group&&output&&inputs.length>1,'Supply inventory, group, output and sequential reports')
const sha=b=>createHash('sha256').update(b).digest('hex'),key=c=>`${c.viewport.width}:${c.file}`
const plan=JSON.parse(readFileSync(planPath)),job=plan.jobs.find(j=>j.name===group);assert.ok(job)
const expected=job.caseKeys,cases=[],provenance=[];let base
for(const input of inputs){
 const path=resolve(input),bytes=readFileSync(path),r=JSON.parse(bytes)
 assert.deepEqual(r.selection.fullInventory,expected)
 assert.deepEqual(r.selection.plannedCaseKeys,expected.slice(cases.length),'Continuation must begin at the first omitted case')
 assert.equal(r.geometryWasmSha256,plan.geometryWasmSha256);assert.equal(r.distIndexSha256,plan.distIndexSha256)
 assert.equal(r.headed,false);assert.equal(r.evidenceOnly,false);assert.equal(r.gpuBackendRequested,'metal')
 if(!base)base=r;else assert.deepEqual(r.sourceHashes,base.sourceHashes,'Continuation product and harness must be identical')
 const failed=r.cases.findIndex(c=>c.status!=='passed'),success=failed<0?r.cases:r.cases.slice(0,failed)
 if(failed>=0)assert.equal(r.cases.length,failed+1,'Cases after a failure cannot be silently skipped')
 assert.deepEqual(success.map(key),expected.slice(cases.length,cases.length+success.length))
 for(const c of success){assert.deepEqual(c.pageErrors,[]);assert.ok(c.assertions.length>0&&c.assertions.every(a=>a.status==='passed'))}
 provenance.push({path,sha256:sha(bytes),successfulCases:success.length,originalPassed:r.passed===true,failedCase:failed<0?null:key(r.cases[failed])});cases.push(...success)
}
assert.deepEqual(cases.map(key),expected,'No missing, duplicated or reordered cases')
assert.equal(new Set(cases.map(key)).size,expected.length)
const receipt={...base,recordedAt:new Date().toISOString(),scope:'Exact contiguous UI continuation union; original failed reports preserved. Same product, harness, runtime, distribution and inputs across segments; not a single uninterrupted run.',selection:{...base.selection,startAt:null,plannedCaseKeys:expected},cases,passed:true,continuationInputs:provenance,continuationVerifierSha256:sha(readFileSync(new URL(import.meta.url)))}
writeFileSync(output,JSON.stringify(receipt,null,2)+'\n');console.log(`${cases.length} exact UI continuation cases reconciled`)
