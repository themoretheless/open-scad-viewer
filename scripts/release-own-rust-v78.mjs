#!/usr/bin/env node
// Append finite PR58 correction evidence. No full workspace or semantic G1 claim.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync,mkdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),read=p=>readFileSync(resolve(root,p))
const sha=b=>createHash('sha256').update(b).digest('hex'),load=p=>JSON.parse(read(p))
const previousPath='docs/qualification/own-rust-cad-v77.json',output='docs/qualification/own-rust-cad-v78.json'
assert(!existsSync(resolve(root,output)),'OwnRust78 exists; publish a successor')
const q='docs/qualification/pr58-ci-mesh-2026-10-19',receiptPath=`${q}/verification.json`,receipt=load(receiptPath)
assert.equal(receipt.schema,'pr58-ci-and-mesh/1')
assert.equal(receipt.checks.nativeTestsPassed,13)
assert.equal(receipt.checks.node20.passed,5);assert.equal(receipt.checks.node22.passed,86)
assert.equal(receipt.fullNativeWorkspaceQualified,false);assert.equal(receipt.fullSemanticG1Qualified,false)
const inputsPath=`${q}/source-inputs.json`,inputs=load(inputsPath)
for(const i of inputs.files)assert.equal(sha(read(i.path)),i.sha256,`Changed measured input ${i.path}`)
const wasm=read('public/wasm/geometry-kernel.wasm'),wasmSha=sha(wasm)
assert(WebAssembly.validate(wasm)&&wasm.length<=16*1024*1024)
assert.deepEqual(read('src/generated/geometry-kernels/kernel_bg.wasm'),wasm)
assert(read('src/core/ownRustCadEvidence.ts').toString().includes(wasmSha))
const browser=load(`${q}/browser.json`),global=load(`${q}/global-geometry-wasm.json`)
assert(browser.passed&&browser.cases.length===9&&browser.cases.every(c=>c.passed&&c.levels.length===3&&c.afterStep.closed))
assert.equal(browser.geometryArtifact.sha256,wasmSha)
assert.equal(global.passed,72);assert.equal(global.refused,24)
assert.equal(global.artifact.geometryWasmSha256,wasmSha)
for(const a of receipt.artifacts)assert.equal(sha(read(a.path)),a.sha256,a.path)
const recipes=['rust-toolchain.toml','scripts/build-geometry-kernels.mjs','scripts/wasm-optimize.mjs','scripts/pack-geometry-kernel.mjs','scripts/build-wasm-brotli.mjs','scripts/reproducible-cargo.mjs']
execFileSync('git',['diff','--quiet',inputs.geometryBuildSourceCommit,'--','crates',...recipes],{cwd:root})
const artifact=`docs/qualification/artifacts/geometry-${wasmSha}.wasm`
mkdirSync(resolve(root,'docs/qualification/artifacts'),{recursive:true})
if(existsSync(resolve(root,artifact)))assert.equal(sha(read(artifact)),wasmSha)
else writeFileSync(resolve(root,artifact),wasm,{flag:'wx'})
const previous=load(previousPath)
const tracked=execFileSync('git',['ls-files','crates','src','scripts'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(rs|toml|ts|mjs|mts)$/u.test(p))
const additions=[
 'scripts/release-own-rust-v78.mjs','scripts/refresh-qualification-fingerprints-v91-v108.mjs','scripts/qualificationPr58Archives.mjs',
 'tests/g0ToolchainFingerprints.test.ts','tests/g1GithubActionsQualification.test.ts',
 'tests/nurbsGuidedPatchError.test.ts','tests/nurbsProgressiveBodyBoundary.test.ts','tests/nurbsSpatialRmfError.test.ts','tests/solidSelfIntersection.test.ts',
 '.github/workflows/g1-qualification-clean.yml','docs/qualification/algorithm-owners-2026-10-14.json',
 'docs/qualification/g0-v92-g1-v109-refreeze-review.md',
 `${q}/README.md`,receiptPath,inputsPath,`${q}/browser.json`,`${q}/cases.json`,`${q}/global-geometry-wasm.json`,
]
const paths=[...new Set([...previous.sourceBundle.paths,...tracked,...additions])].filter(p=>existsSync(resolve(root,p))).sort()
const evidencePaths=[receiptPath,`${q}/browser.json`,`${q}/global-geometry-wasm.json`,
 `${q}/native-mesh.log.gz`,`${q}/native-regressions.log.gz`,`${q}/node20-five.log.gz`,`${q}/node22-full.log.gz`,
 `${q}/source-guards.log.gz`,`${q}/verify-dist.log.gz`,
 'docs/qualification/pr58-ci-mesh-fingerprint-refresh-2026-10-19/e514-node20-ci.log.gz']
const next={id:'own-rust-cad-v78',successorOf:previousPath,previousSha256:sha(read(previousPath)),recordedAt:'2026-10-19',
 purpose:'Bind the final PR58 archive registry split and current executors to the unchanged measured Rust, public WASM and finite regression receipts. Preserve every predecessor and import no clean semantic or full-workspace result.',qualificationClaim:'none',
 wasm:{path:artifact,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha,byteLength:wasm.length,
  variants:[{sha256:wasmSha,byteLength:wasm.length,builder:'macos-aarch64',evidence:'Observed public release/Binaryen/Brotli artifact; finite Node20/Node22/Chromium checks. No cross-host byte identity is inferred.'}]},
 sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',paths,
  sha256:sha(Buffer.concat(paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))},
 dependencyFingerprints:{noticesSha256:sha(read('THIRD_PARTY_NOTICES.md')),npmLockfileSha256:sha(read('package-lock.json')),rustLockfileSha256:sha(read('crates/Cargo.lock'))},
 evidenceRecords:evidencePaths.map(path=>({path,sha256:sha(read(path)),byteLength:read(path).length})),
 finiteEvidenceSummary:{path:receiptPath,sha256:sha(read(receiptPath))},
 sourceBuildObservation:{sourceSha:inputs.geometryBuildSourceCommit,path:inputsPath,sha256:sha(read(inputsPath)),stableInputsVerified:true},
 hostedCiObservation:{commit:'e51471d52a46c75de5d0402ba31bc84757f22ec6',originalFiveFailuresRemaining:0,
  newFailureScope:'Seven active G0/G1 fingerprint tests; fresh hosted qualification is still required after the append-only re-freeze.'},
 claimBoundary:['Only 13 named native tests, five Node20 regressions, 86 Node22 tests, nine Chromium native mesh/STEP cases, 96 global cases, source guards, types and distribution checks are measured here.',
  'No previous full-workspace, STEP-oracle, UI lifecycle or hosted success is imported as current evidence.',
  'No G0 closure, full semantic G1, arbitrary ideal-body geometry guarantee or production cutover approval.']}
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log(`Recorded OwnRust78: ${wasmSha} (${wasm.length} bytes), ${paths.length} source paths; qualificationClaim=none`)
