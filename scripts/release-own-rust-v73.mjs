#!/usr/bin/env node
// Exact observed native-source/artifact successor; historical versions immutable.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync, writeFileSync, existsSync, mkdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {resolve} from 'node:path'
const root = resolve(import.meta.dirname, '..'), read = p => readFileSync(resolve(root, p))
const sha = b => createHash('sha256').update(b).digest('hex')
const previousPath = 'docs/qualification/own-rust-cad-v72.json'
const output = 'docs/qualification/own-rust-cad-v73.json'
assert(!existsSync(resolve(root, output)), 'Historical OwnRust73 exists; publish a successor')
const evidenceRoot = 'docs/qualification/general-frame-cap-2026-10-15'
const reportPath = `${evidenceRoot}/summary.json`, report = JSON.parse(read(reportPath))
assert.equal(report.status, 'passed')
assert.equal(report.native.brepPassed, 1108)
assert.equal(report.native.nurbsPassed, 1545)
for (const record of report.logs) {
  const b = read(record.path)
  assert.equal(sha(b), record.sha256, `Changed evidence: ${record.path}`)
  assert.equal(b.length, record.byteLength)
}
const inputs = JSON.parse(read(`${evidenceRoot}/build-inputs.json`))
for (const input of inputs.inputs) assert.equal(sha(read(input.path)), input.sha256, `Stale build input ${input.path}`)
for (const [name, count] of [['native-brep.log',1108],['native-nurbs.log',1545],
  ['native-correspondence.log',7],['native-source-owned.log',4],['native-protocol-step.log',4],['native-spatial-regressions.log',2]]) {
  assert(read(`${evidenceRoot}/${name}`).toString().includes(`test result: ok. ${count} passed; 0 failed;`))
}
assert(read(`${evidenceRoot}/native-no-codec.log`).toString().includes('Finished `dev` profile'))
assert(read(`${evidenceRoot}/wasm-build.log`).toString().includes('Finished `release` profile'))
const wasm = read('public/wasm/geometry-kernel.wasm'), wasmSha = sha(wasm)
assert.equal(wasmSha, report.wasm.sha256)
assert.equal(wasm.length, report.wasm.byteLength)
assert.notEqual(wasmSha, JSON.parse(read(previousPath)).wasm.sha256, 'New geometry must bind its rebuilt bytes')
for (const name of ['node', 'browser']) {
  const r = JSON.parse(read(`${evidenceRoot}/${name}.json`))
  assert.equal(r.passed, 11)
  assert.equal(r.artifact.geometryWasmSha256, wasmSha)
  assert.equal(r.artifact.geometryWasmByteLength, wasm.length)
  assert(r.records.every(x => x.passed))
}
const previousMatrix = JSON.parse(read(`${evidenceRoot}/previous-matrix-node.json`))
assert.equal(previousMatrix.passed,72); assert.equal(previousMatrix.refused,24)
assert.equal(previousMatrix.artifact.geometryWasmSha256,wasmSha)
const occt = JSON.parse(read(`${evidenceRoot}/occt.json`))
assert(occt.passed && occt.valid && occt.volumeError < 1e-9)
assert.equal(occt.stepSha256, sha(read(`${evidenceRoot}/nonplanar-frame-body.step`)))
assert.equal(occt.oracleSha256, sha(read('scripts/verify-general-boundary-step.py')))
const fixturePath = 'crates/geometry-bridge/tests/fixtures/general-frame-cap-certificate-v1.json'
assert.equal(occt.fixtureSha256,sha(read(fixturePath)))
execFileSync(process.execPath, ['scripts/audit-architecture.mjs'], {cwd:root,stdio:'inherit'})
const artifact = `docs/qualification/artifacts/geometry-${wasmSha}.wasm`
mkdirSync(resolve(root, 'docs/qualification/artifacts'), {recursive:true})
if (existsSync(resolve(root, artifact))) assert.equal(sha(read(artifact)), wasmSha)
else writeFileSync(resolve(root,artifact),wasm,{flag:'wx'})
const runtimePath = 'src/core/ownRustCadEvidence.ts', runtime = read(runtimePath).toString()
const updated = runtime.replace(/export const OWN_RUST_CAD_ARTIFACTS = Object.freeze\([\s\S]*?\)\nexport function/,
  `export const OWN_RUST_CAD_ARTIFACTS = Object.freeze(${JSON.stringify([{sha256:wasmSha,byteLength:wasm.length}],null,2)})\nexport function`)
assert(updated.includes(wasmSha)); writeFileSync(resolve(root,runtimePath),updated)
execFileSync(process.execPath,['scripts/audit-algorithm-owners.mjs','--write'],{cwd:root,stdio:'inherit'})
const previous = JSON.parse(read(previousPath)), next = structuredClone(previous)
delete next.environmentUpdate
next.id = 'own-rust-cad-v73'; next.successorOf = previousPath; next.previousSha256 = sha(read(previousPath)); next.recordedAt = '2026-10-15'
next.purpose = 'Bind newly rebuilt Rust geometry with general authored-boundary correspondence and original moving-frame/filled-cap construction, strict common-metric contacts, original spatial path/chord bounds and preserved non-dyadic natural edges. Native full BRep/NURBS tests, shared-budget refusals, Node/Chromium protocol checks and independent exact retained-volume/OCCT interchange pass. No universal acceptance, ideal-body embedding, G1/G2, imported semantic fragments or approval is inferred.'
next.dependencyFingerprints = {noticesSha256:sha(read('THIRD_PARTY_NOTICES.md')),npmLockfileSha256:sha(read('package-lock.json')),rustLockfileSha256:sha(read('crates/Cargo.lock'))}
next.wasm = {path:artifact,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha,byteLength:wasm.length,
  variants:[{sha256:wasmSha,byteLength:wasm.length,builder:'macos-aarch64',evidence:`Observed stable-source rebuild with Binaryen116 and Node ${process.version}; ${execFileSync('rustc',['--version'],{cwd:root,encoding:'utf8'}).trim()}. No cross-builder byte identity or foreign-host run inferred.`}]}
const tracked = execFileSync('git',['ls-files','crates','src','scripts'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(rs|toml|ts|mjs|mts)$/.test(p))
next.sourceBundle.paths = [...new Set([...previous.sourceBundle.paths,...tracked,
  ...report.logs.map(x=>x.path).filter(p=>!p.endsWith('.step')), reportPath, fixturePath,
  'scripts/release-own-rust-v73.mjs','scripts/generalBoundaryChecks.mjs','scripts/qualify-general-boundary.mts',
  'scripts/qualify-general-boundary-browser.mjs','scripts/verify-general-boundary-step.py',
  'docs/qualification/general-frame-cap-certificate-2026-10-15.md',
  'docs/qualification/g0-v87-g1-v104-refreeze-review.md',runtimePath,
  'tests/g0ToolchainFingerprints.test.ts','tests/g1GithubActionsQualification.test.ts',
  '.github/workflows/g1-qualification-clean.yml','docs/qualification/algorithm-owners-2026-10-14.json'
])].filter(p=>existsSync(resolve(root,p))).sort()
next.sourceBundle.sha256 = sha(Buffer.concat(next.sourceBundle.paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))
next.sourceBundle.binaryEvidenceBinding = 'The finite summary binds exact STEP bytes by SHA256 and byte length; binary archives are not treated as canonical LF text.'
next.finiteEvidenceSummary = {path:reportPath,sha256:sha(read(reportPath))}
next.sourceBuildObservation = {sourceSha:inputs.sourceSha,path:`${evidenceRoot}/build-inputs.json`,sha256:sha(read(`${evidenceRoot}/build-inputs.json`)),stableInputsVerified:true}
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log(`Published OwnRust73: ${wasmSha} (${wasm.length} bytes)`)
