#!/usr/bin/env node
// Append exact observed native integration inputs; historical records are immutable.
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync,mkdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),read=p=>readFileSync(resolve(root,p))
const sha=b=>createHash('sha256').update(b).digest('hex')
const previousPath='docs/qualification/own-rust-cad-v68.json',output='docs/qualification/own-rust-cad-v69.json'
if(existsSync(resolve(root,output)))throw new Error('Historical OwnRust69 already exists; publish a successor')
const previous=JSON.parse(read(previousPath)),next=structuredClone(previous)
const reportPath='docs/qualification/rush-emitter-snapshot-2026-10-14/summary.json',report=JSON.parse(read(reportPath))
if(report.native.status!=='passed'||report.architecture.status!=='passed'||report.node.status!=='passed'||report.browser.status!=='passed')throw new Error('Native checks and architecture audit must pass before publication')
const wasm=read('public/wasm/geometry-kernel.wasm'),wasmSha=sha(wasm),artifact=`docs/qualification/artifacts/geometry-${wasmSha}.wasm`
if(report.wasm.sha256!==wasmSha||report.wasm.byteLength!==wasm.length)throw new Error('Report does not bind current packaged WASM')
for(const kind of ['node','browser']){
 const observed=JSON.parse(read(report[kind].path))
 if(observed.passed!==72||observed.refused!==24||observed.intersections.length!==5||observed.artifact.geometryWasmSha256!==wasmSha||observed.artifact.geometryWasmByteLength!==wasm.length)throw new Error(`Incomplete or stale ${kind} qualification`)
}
mkdirSync(resolve(root,'docs/qualification/artifacts'),{recursive:true})
if(existsSync(resolve(root,artifact))) {if(sha(read(artifact))!==wasmSha)throw new Error('Archived artifact differs')}
else writeFileSync(resolve(root,artifact),wasm,{flag:'wx'})
next.id='own-rust-cad-v69';next.successorOf=previousPath;next.previousSha256=sha(read(previousPath));next.recordedAt='2026-10-14'
next.purpose='Revalidate the measured unchanged WASM and unchanged production Rust sources after migrating canonical Rush snapshot test inputs and the verified 81-row reference. Bind both compiler fixtures and the delivery CI definition; increase only overall native-job time after an observed Windows 90-minute timeout, with every test and mathematical budget unchanged. Preserve completed G1v100 evidence without importing any credit into fresh G1v101.'
next.dependencyFingerprints={noticesSha256:sha(read('THIRD_PARTY_NOTICES.md')),npmLockfileSha256:sha(read('package-lock.json')),rustLockfileSha256:sha(read('crates/Cargo.lock'))}
next.wasm={path:artifact,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha,byteLength:wasm.length,variants:[{sha256:wasmSha,byteLength:wasm.length,builder:'macos-aarch64',evidence:`Post-fixture complete build pipeline executed successfully; identical production inputs and byte-identical packaged geometry were retained from OwnRust64. ${execFileSync('rustc',['--version'],{cwd:root,encoding:'utf8'}).trim()}; Node ${process.version}. No foreign-host identity or clean semantic qualification inferred.`}]}
const runtimePath='src/core/ownRustCadEvidence.ts',runtime=read(runtimePath).toString()
let updated=runtime.replace(previous.dependencyFingerprints.rustLockfileSha256,next.dependencyFingerprints.rustLockfileSha256)
updated=updated.replace(/export const OWN_RUST_CAD_ARTIFACTS = Object.freeze\([\s\S]*?\)\nexport function/,`export const OWN_RUST_CAD_ARTIFACTS = Object.freeze(${JSON.stringify([{sha256:wasmSha,byteLength:wasm.length}],null,2)})\nexport function`)
if(!updated.includes(wasmSha))throw new Error('Runtime identity refresh failed')
writeFileSync(resolve(root,runtimePath),updated)
execFileSync(process.execPath,['scripts/audit-algorithm-owners.mjs','--write'],{cwd:root,stdio:'inherit'})
const tracked=execFileSync('git',['ls-files','crates','src','scripts'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(rs|toml|ts|mjs|mts)$/.test(p))
const evidence=execFileSync('git',['ls-files','docs/qualification/global-geometry-2026-10-14','docs/qualification/parallel-main-history-2026-10-14','docs/qualification/rush-emitter-snapshot-2026-10-14'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(json|md|mjs|ts|mts|log)$/.test(p))
next.sourceBundle.paths=[...new Set([...previous.sourceBundle.paths,...tracked,...evidence,'docs/qualification/rush-emitter-snapshot-2026-10-14/verify-payloads.py','docs/qualification/g1-v100-clean-matrix-2026-10-14/manifest.json',runtimePath,'.gitattributes','tests/g0ToolchainFingerprints.test.ts','tests/g1GithubActionsQualification.test.ts','crates/Cargo.lock','architecture-baseline.json','scripts/release-own-rust-v69.mjs','docs/qualification/environment-freeze/g1-github-actions-v37.json','docs/qualification/g0-v84-g1-v101-refreeze-review.md','crates/geometry-bridge/tests/fixtures/golden_numeric.txt','crates/rush-runtime/tests/fixtures/emitter-snapshot.tsv','crates/rush-runtime/tests/fixtures/emitter-parity.json','.github/workflows/ci.yml','tests/quinticPolygonBody.test.ts','tests/solidSelfIntersection.test.ts','tests/nurbsClosedPlanarRmfSweep.test.ts','tests/nurbsClosedAuthoredSweepBound.test.ts',reportPath,'docs/design/algorithm-owners.json','docs/qualification/algorithm-owners-2026-10-14.json'])].filter(p=>existsSync(resolve(root,p))).sort()
next.sourceBundle.sha256=sha(Buffer.concat(next.sourceBundle.paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))
next.finiteEvidenceSummary={path:reportPath,sha256:sha(read(reportPath))}
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log(`Published OwnRust69: ${wasmSha} (${wasm.length} bytes)`)
