#!/usr/bin/env node
// Publish only a completed exact-artifact direct-API baseline; never infer broader qualification.
import assert from 'node:assert/strict'
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import {fileURLToPath} from 'node:url'
import {packGeometryKernel} from './pack-geometry-kernel.mjs'

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const path=value=>resolve(root,value)
const bytes=value=>readFileSync(path(value))
const sha256=value=>createHash('sha256').update(value).digest('hex')
const readJson=value=>JSON.parse(bytes(value))
const writeExclusive=(value,data)=>{
  mkdirSync(dirname(path(value)),{recursive:true})
  writeFileSync(path(value),data,{flag:'wx'})
}
const writeJson=(value,data)=>writeExclusive(value,JSON.stringify(data,null,2)+'\n')

export function publishRuntimeDirectApi(qualificationDirectory,recordedAt){
  assert(/^\d{4}-\d{2}-\d{2}$/u.test(recordedAt),'recordedAt must be YYYY-MM-DD')
  const wasmPath='public/wasm/geometry-kernel.wasm'
  const wasm=bytes(wasmPath),wasmSha256=sha256(wasm),short=wasmSha256.slice(0,12)
  assert.equal(qualificationDirectory,`docs/qualification/runtime-direct-api-${wasmSha256.slice(0,6)}-${recordedAt}`,
    'Qualification directory must be derived from the observed artifact and date')
  packGeometryKernel(wasm,path('src/generated/geometry-kernels'),path('public/wasm'))
  assert.deepEqual(bytes('src/generated/geometry-kernels/kernel_bg.wasm'),wasm,'Packed/public candidate bytes differ')
  const identity=readJson(`${qualificationDirectory}/report.json`).identity
  assert.deepEqual(identity,{sha256:wasmSha256,byteLength:wasm.byteLength},'Browser candidate identity differs')
  const generatedIdentity=bytes('src/generated/geometry-kernels/identity.ts').toString('utf8')
    .match(/^export default Object\.freeze\((\{[^\n]+\})\)\s*$/m)
  assert(generatedIdentity,'Unrecognized generated identity')
  assert.deepEqual(JSON.parse(generatedIdentity[1]),identity,'Generated identity differs from actual bytes')

  const node=readJson(`${qualificationDirectory}/node.json`)
  assert(node.success&&node.numFailedTests===0&&node.numPendingTests===0,'Incomplete Node direct-API baseline')
  assert.equal(node.numPassedTests,115,'Unexpected Node direct-API baseline size')
  const browser=readJson(`${qualificationDirectory}/report.json`)
  assert(browser.passed&&browser.productionFilesUnchanged&&browser.embeddedOnly,'Incomplete isolated browser baseline')
  assert.deepEqual(browser.pageErrors,[],'Browser page errors')
  assert(browser.result.main.passed&&browser.result.main.checks===40,'Incomplete Chromium main baseline')
  assert(browser.result.thread.ok&&browser.result.thread.result.passed&&browser.result.thread.result.checks===40,
    'Incomplete Chromium worker baseline')
  assert.deepEqual(browser.result.thread.identity,identity,'Browser worker identity differs')
  assert.equal(sha256(wasm),wasmSha256,'WASM changed while reading qualification evidence')

  const artifact={
    wasmSha256,
    noticesSha256:sha256(bytes('THIRD_PARTY_NOTICES.md')),
    lockfileSha256:sha256(bytes('package-lock.json')),
    rustLockfileSha256:sha256(bytes('crates/Cargo.lock')),
    recordId:`docs/qualification/runtime-provider-${short}-${recordedAt}.json`,
    corpusVersion:`runtime-direct-api-${short}-${recordedAt}`,
  }
  const baseline={
    schema:'runtime-direct-api-baseline/1',state:'complete',wasmSha256,
    checks:{directNodeApiPassed:node.numPassedTests,chromiumMainChecks:40,chromiumIndependentWorkerChecks:40},
    evidence:[
      {path:`${qualificationDirectory}/node.json`,sha256:sha256(bytes(`${qualificationDirectory}/node.json`))},
      {path:`${qualificationDirectory}/report.json`,sha256:sha256(bytes(`${qualificationDirectory}/report.json`))},
    ],
    scope:'Finite exact-artifact direct API baseline: 115 Node tests and 40 Chromium main/40 independent-worker embedded checks. No complete frontend, application worker protocol, global geometry, Linux or hosted CI qualification.',
    qualificationClaim:'finite-runtime-direct-api-only',
  }
  const verificationReceipt=`${qualificationDirectory}/runtime-api-baseline.json`
  writeJson(verificationReceipt,baseline)
  const archivedArtifact=`docs/qualification/artifacts/geometry-${wasmSha256}.wasm`
  writeExclusive(archivedArtifact,wasm)
  writeJson(artifact.recordId,{
    schema:'runtime-provider-qualification/1',state:'complete',qualified:true,recordId:artifact.recordId,
    scope:baseline.scope,artifact,qualificationDirectory,qualificationSha256:sha256(bytes(verificationReceipt)),
    verificationReceipt,deliveryState:'local-direct-api-qualified',
    claimBoundary:{
      G0G1QualificationClaim:'none',completedCleanWorkUnits:0,plannedCleanWorkUnits:4740,
      universalCadCoverage:false,fullFrontendQualification:false,applicationWorkerProtocolQualification:false,
      productionPackagingQualification:false,linuxQualification:false,hostedCiQualification:false,mainPushed:false,
    },
  })
  writeFileSync(path('src/core/currentRuntimeGeometryEvidence.ts'),
    `// Runtime v10: finite exact-artifact API baseline, broader qualification separate.\nexport const CURRENT_RUNTIME_GEOMETRY_EVIDENCE = Object.freeze(${JSON.stringify(artifact,null,2)} as const)\n`)
  const auditPath=`${qualificationDirectory}/runtime-binding.json`
  const audit=execFileSync(process.execPath,['--import','tsx','scripts/audit-runtime-manifests.mts'],{cwd:root,encoding:'utf8'})
  writeExclusive(auditPath,audit)
  const report=JSON.parse(audit)
  assert(report.matches&&report.publicArtifactMatches&&report.artifactSha256===wasmSha256,'Published runtime binding audit failed')
  return {artifact,archivedArtifact,verificationReceipt,auditPath,nodePassed:node.numPassedTests,
    chromiumMainChecks:40,chromiumIndependentWorkerChecks:40,manifestAuditMatches:report.matches}
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const [qualificationDirectory,recordedAt]=process.argv.slice(2)
  assert(qualificationDirectory&&recordedAt,'Usage: node scripts/publish-runtime-direct-api.mjs <qualification-directory> <YYYY-MM-DD>')
  console.log(JSON.stringify(publishRuntimeDirectApi(qualificationDirectory,recordedAt),null,2))
}
