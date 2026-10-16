#!/usr/bin/env node
// Bind finite observed delivery evidence; this never executes or approves G1.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync,mkdirSync,readdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),read=p=>readFileSync(resolve(root,p))
const sha=b=>createHash('sha256').update(b).digest('hex')
const evidenceRoot='docs/qualification/fibered-ci-size-2026-10-17'
const reportPath=`${evidenceRoot}/summary.json`,report=JSON.parse(read(reportPath))
const output='docs/qualification/own-rust-cad-v75.json',previousPath='docs/qualification/own-rust-cad-v74.json'
assert(!existsSync(resolve(root,output)),'Historical OwnRust75 exists; publish a successor')
assert.equal(report.qualificationClaim,'none')
assert.equal(report.nativeFocusedTests.passed,76);assert.equal(report.nativeFocusedTests.failed,0)
assert.equal(report.publicWasm.casesPassed,14);assert.equal(report.distribution.verified,true)
for(const record of report.records) {
  const bytes=read(record.path);assert.equal(sha(bytes),record.sha256,`Changed receipt ${record.path}`)
  assert.equal(bytes.length,record.byteLength)
}
for(const [name,count] of [['step',30],['injectivity',14],['closed',32]]) {
  const log=read(`${evidenceRoot}/osv-fibered-ci-${name}-tests-20261016.log`).toString()
  assert(log.includes(`test result: ok. ${count} passed; 0 failed;`));assert(!log.includes('test result: FAILED'))
}
const wasm=read('public/wasm/geometry-kernel.wasm'),wasmSha=sha(wasm)
assert.equal(wasmSha,report.publicWasm.sha256);assert.equal(wasm.length,report.publicWasm.byteLength)
for(const [name,count] of [['fibered',4],['caps',6],['sphere',4]]) {
  const r=JSON.parse(read(`${evidenceRoot}/osv-fibered-ci-public-${name}-runtime-20261017.json`))
  assert.equal(r.passed,true);assert.equal(r.cases.length,count);assert(r.cases.every(c=>c.passed))
  assert.equal(r.wasmSha256,wasmSha);assert.equal(r.wasmBytes,wasm.length)
}
const source='cec378bb23e029fe8498b6c89fe1e1d3e8aa86c7'
// The observed binary was built from this exact production Rust/build recipe.
// CI and qualification metadata may advance independently; no old test credit
// is transferred to any changed Rust source.
const rustPaths=execFileSync('git',['ls-tree','-r','--name-only',source,'crates'],{cwd:root,encoding:'utf8'}).trim().split('\n')
const recipes=['rust-toolchain.toml','scripts/build-geometry-kernels.mjs','scripts/wasm-optimize.mjs','scripts/pack-geometry-kernel.mjs','scripts/build-wasm-brotli.mjs']
execFileSync('git',['diff','--quiet',source,'--','crates',...recipes],{cwd:root})
const buildInputs=[...new Set([...rustPaths,...recipes.filter(p=>existsSync(resolve(root,p)))])].sort().map(path=>{
 return {path,sha256:sha(read(path))}
})
const observationPath=`${evidenceRoot}/build-inputs.json`
const observation={schema:'observed-public-build-source-comparison/1',sourceSha:source,recordedAt:'2026-10-17',observation:'Post-build exact comparison to immutable committed Rust and recipe bytes; not a pre-build snapshot or a foreign-host claim.',wasmSha256:wasmSha,inputs:buildInputs}
const observationBytes=Buffer.from(JSON.stringify(observation,null,2)+'\n')
if(existsSync(resolve(root,observationPath)))assert.deepEqual(read(observationPath),observationBytes)
else writeFileSync(resolve(root,observationPath),observationBytes,{flag:'wx'})
const artifact=`docs/qualification/artifacts/geometry-${wasmSha}.wasm`
mkdirSync(resolve(root,'docs/qualification/artifacts'),{recursive:true})
if(existsSync(resolve(root,artifact)))assert.equal(sha(read(artifact)),wasmSha)
else writeFileSync(resolve(root,artifact),wasm,{flag:'wx'})
const previous=JSON.parse(read(previousPath))
const tracked=execFileSync('git',['ls-files','crates','src','scripts'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(rs|toml|ts|mjs|mts)$/.test(p))
const newExecutors=['scripts/release-own-rust-v75.mjs','scripts/refresh-qualification-fingerprints-v88-v105.mjs','scripts/qualificationFiberedSourceArchives.mjs']
const paths=[...new Set([...previous.sourceBundle.paths,...tracked,...newExecutors,
 ...readdirSync(resolve(root,evidenceRoot)).map(p=>`${evidenceRoot}/${p}`),
 'docs/qualification/g0-v89-g1-v106-refreeze-review.md',
 'docs/qualification/environment-freeze/g1-github-actions-v39.json',
 'tests/g0ToolchainFingerprints.test.ts','tests/g1GithubActionsQualification.test.ts',
 '.github/workflows/ci.yml','.github/workflows/g1-qualification-clean.yml',
 'docs/qualification/algorithm-owners-2026-10-14.json',
])].filter(p=>existsSync(resolve(root,p))).sort()
const next={id:'own-rust-cad-v75',successorOf:previousPath,previousSha256:sha(read(previousPath)),recordedAt:'2026-10-17',
 purpose:'Bind the rebuilt fibered Rust geometry WASM, CI module layout repairs and measured package budgets to exact current source, lockfile and finite local evidence. No complete semantic matrix, browser, hosted CI or production approval is claimed.',qualificationClaim:'none',
 wasm:{path:artifact,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha,byteLength:wasm.length,variants:[{sha256:wasmSha,byteLength:wasm.length,builder:'macos-aarch64',evidence:'Observed normal public rebuild, finite native WASM runtime matrix and unchanged production release profile. No cross-host byte identity inferred.'}]},
 sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',paths,sha256:sha(Buffer.concat(paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))},
 dependencyFingerprints:{noticesSha256:sha(read('THIRD_PARTY_NOTICES.md')),npmLockfileSha256:sha(read('package-lock.json')),rustLockfileSha256:sha(read('crates/Cargo.lock'))},
 finiteEvidenceSummary:{path:reportPath,sha256:sha(read(reportPath))},
 sourceBuildObservation:{sourceSha:source,path:observationPath,sha256:sha(observationBytes),stableInputsVerified:true},
 claimBoundary:['Finite observed evidence only: 76 focused native tests, 14 public WASM cases and verified distribution.',
 'Historical whole-workspace/independent STEP receipts remain tied to their own original immutable revisions.',
 'No G0 closure, semantic G1 result, browser/UI qualification, cross-host CI success or production cutover approval.']}
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log(`Recorded OwnRust75: ${wasmSha} (${wasm.length} bytes), ${paths.length} bound paths; qualificationClaim=none`)
