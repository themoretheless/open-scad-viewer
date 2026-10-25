#!/usr/bin/env node
// Append finite observed original-carrier delivery evidence. Never executes G1.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync,mkdirSync,readdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),read=p=>readFileSync(resolve(root,p))
const sha=b=>createHash('sha256').update(b).digest('hex'),load=p=>JSON.parse(read(p))
const q='docs/qualification/interpolated-carrier-delivery-2026-10-18'
const summaryPath=`${q}/summary.json`,report=load(summaryPath)
const previousPath='docs/qualification/own-rust-cad-v75.json',output='docs/qualification/own-rust-cad-v76.json'
assert(!existsSync(resolve(root,output)),'Historical OwnRust76 exists; publish a successor')
assert.equal(report.passed,true);assert.equal(report.qualificationClaim,'none')
for(const record of report.records){const bytes=read(record.path);assert.equal(sha(bytes),record.sha256,record.path);assert.equal(bytes.length,record.byteLength)}
const native=load(report.nativeWorkspace.path),cover=load(report.nativeExecutionCover.path)
assert(native.complete&&native.passed&&native.sourceGuardVerified)
assert(cover.complete&&cover.totals.failed===0&&cover.totals.ignored===5)
assert.deepEqual(native.totals,cover.totals)
assert.equal(cover.checks.reduce((n,c)=>n+c.executed,0),report.nativeCatalogCases)
const predecessor=load('docs/qualification/interpolated-carrier-source-2026-10-17/complete-2026-10-18/summary.json')
assert.equal(predecessor.productionRustRevision,'312bb2613fe034a9cd3d7ffcebf4f46a1eac15f4')
assert.deepEqual(predecessor.native,{passed:3408,failed:0,ignored:4})
assert.equal(predecessor.privateWasmCasesPassed,19);assert.equal(predecessor.independentOcctCasesPassed,19)
for(const record of predecessor.records){const bytes=read(record.path);assert.equal(sha(bytes),record.sha256,record.path);assert.equal(bytes.length,record.byteLength)}
const runtime=load(report.publicWasmRuntime.path)
assert(runtime.passed&&runtime.sourceGuardVerified&&runtime.artifactUnchanged)
assert.equal(runtime.casesPassed,19)
const wasm=read('public/wasm/geometry-kernel.wasm'),wasmSha=sha(wasm)
assert.equal(runtime.wasmSha256,wasmSha);assert.equal(runtime.wasmBytes,wasm.length)
assert(wasm.length<=16*1024*1024)
for(const [name,count] of [['fibered',4],['caps',6],['sphere',4],['bishop',1],['general',1],['affine',1],['guided',1],['interpolated',1]]){
 const check=runtime.checks.find(c=>c.name===name);assert(check&&check.exitCode===0&&check.inputsUnchanged&&check.cases===count)
}
const carrier=load(report.browserCarrier.path),general=load(report.browserGeneral.path),ui=load(report.browserSolid.path)
assert(carrier.passed&&carrier.sourcePreserved&&carrier.stepRoundTripPassed&&carrier.zeroAndOneShortRefusalsPassed&&carrier.forgedFlagsRefused)
assert.equal(carrier.artifact.geometryWasmSha256,wasmSha);assert.equal(carrier.seamContinuity,'C0');assert.equal(carrier.smoothnessCertified,false)
assert.equal(general.passed,11);assert.equal(general.artifact.geometryWasmSha256,wasmSha)
assert(ui.passed&&ui.cases.length===4&&ui.cases.every(c=>c.status==='passed'&&c.checks.length===9))
assert.equal(ui.geometryWasmSha256,wasmSha);assert.equal(report.distributionVerified,true)
const nativeInputs=load(`${q}/native-inputs.json`)
for(const i of nativeInputs.inputs)assert.equal(sha(read(i.path)),i.sha256,`Changed native input ${i.path}`)
const auxiliary=load(`${q}/native-example-inputs.json`)
for(const i of auxiliary.files)assert.equal(sha(read(i.path)),i.sha256,`Changed native example ${i.path}`)
const source=nativeInputs.revision
const recipes=['rust-toolchain.toml','scripts/build-geometry-kernels.mjs','scripts/wasm-optimize.mjs','scripts/pack-geometry-kernel.mjs','scripts/build-wasm-brotli.mjs','scripts/reproducible-cargo.mjs']
execFileSync('git',['diff','--quiet',source,'--','crates',...recipes],{cwd:root})
const buildPaths=[...new Set([...nativeInputs.inputs.map(i=>i.path),...recipes])].sort()
const observationPath=`${q}/build-inputs.json`
const observation={schema:'observed-public-original-carrier-build-source/1',sourceSha:source,productionRustEquivalentTo:nativeInputs.productionRustEquivalentTo,recordedAt:'2026-10-18',wasmSha256:wasmSha,
 observation:'Post-build exact comparison to committed Rust and recipe bytes plus the original native guard; independent of mutable qualification metadata.',
 inputs:buildPaths.map(path=>({path,sha256:sha(read(path))}))}
const observationBytes=Buffer.from(JSON.stringify(observation,null,2)+'\n')
if(existsSync(resolve(root,observationPath)))assert.deepEqual(read(observationPath),observationBytes)
else writeFileSync(resolve(root,observationPath),observationBytes,{flag:'wx'})
const artifact=`docs/qualification/artifacts/geometry-${wasmSha}.wasm`
mkdirSync(resolve(root,'docs/qualification/artifacts'),{recursive:true})
if(existsSync(resolve(root,artifact)))assert.equal(sha(read(artifact)),wasmSha)
else writeFileSync(resolve(root,artifact),wasm,{flag:'wx'})
assert(read('src/core/ownRustCadEvidence.ts').toString().includes(wasmSha),'Runtime metadata must name the observed artifact before freezing')
function ordinaryFiles(dir){return readdirSync(resolve(root,dir),{withFileTypes:true}).flatMap(e=>e.isDirectory()?ordinaryFiles(`${dir}/${e.name}`):/\.(?:json|mjs|mts|py|md|log|toml|rs)$/u.test(e.name)?[`${dir}/${e.name}`]:[])}
const previous=load(previousPath)
const tracked=execFileSync('git',['ls-files','crates','src','scripts'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(rs|toml|ts|mjs|mts)$/u.test(p))
const paths=[...new Set([...previous.sourceBundle.paths,...tracked,...ordinaryFiles(q),...auxiliary.files.map(i=>i.path).filter(p=>/\.(r|scad|json|md|txt|csv|svg)$/u.test(p)),
 'scripts/release-own-rust-v76.mjs','scripts/refresh-qualification-fingerprints-v89-v106.mjs','scripts/qualify-interpolated-carrier-browser.mjs',
 'docs/qualification/g0-v90-g1-v107-refreeze-review.md','docs/qualification/environment-freeze/g1-github-actions-v40.json',
 'docs/design/original-sweep-delivery-gates.md','docs/design/ideal-interpolated-carrier-charts.md',
 'tests/g0ToolchainFingerprints.test.ts','tests/g1GithubActionsQualification.test.ts','tests/qualificationDriftAudit.test.mjs',
 '.github/workflows/ci.yml','.github/workflows/g1-qualification-clean.yml','docs/qualification/algorithm-owners-2026-10-14.json',
])].filter(p=>existsSync(resolve(root,p))).sort()
const next={id:'own-rust-cad-v76',successorOf:previousPath,previousSha256:sha(read(previousPath)),recordedAt:'2026-10-18',
 purpose:'Bind the combined original-carrier Rust guarantees, complete local workspace, fresh public WASM/Chromium/Solid lifecycle cases and distribution to exact source, artifact and dependencies. Full semantic G1 and new hosted CI remain independent.',qualificationClaim:'none',
 wasm:{path:artifact,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha,byteLength:wasm.length,variants:[{sha256:wasmSha,byteLength:wasm.length,builder:'macos-aarch64',evidence:'Observed normal public release/Binaryen/Brotli recipe with finite current runtime checks. No cross-host byte identity is inferred.'}]},
 sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',paths,sha256:sha(Buffer.concat(paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))},
 dependencyFingerprints:{noticesSha256:sha(read('THIRD_PARTY_NOTICES.md')),npmLockfileSha256:sha(read('package-lock.json')),rustLockfileSha256:sha(read('crates/Cargo.lock'))},
 evidenceRecords:[...new Map([...report.records,...predecessor.records].map(r=>[r.path,r])).values()].sort((a,b)=>a.path.localeCompare(b.path)),
 finiteEvidenceSummary:{path:summaryPath,sha256:sha(read(summaryPath))},sourceBuildObservation:{sourceSha:source,path:observationPath,sha256:sha(observationBytes),stableInputsVerified:true},
 claimBoundary:['Complete named native workspace and finite public runtime/browser/distribution checks only.',
 'Selected source-kernel/independent STEP receipts remain bound to their unchanged Rust source, with exact current native inputs; they do not qualify arbitrary source classes.',
 'The new raw carrier API browser check does not imply Rush authoring or viewport integration of arbitrary authored source caps. Physical cap C0 remains explicit.',
 'No G0 closure, full semantic G1 result, cross-host CI success, universal acceptance or production cutover approval.']}
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log(`Recorded OwnRust76: ${wasmSha} (${wasm.length} bytes), ${paths.length} bound paths; qualificationClaim=none`)
