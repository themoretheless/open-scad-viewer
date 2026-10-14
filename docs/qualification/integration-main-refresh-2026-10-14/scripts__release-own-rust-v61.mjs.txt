#!/usr/bin/env node
// Append exact observed native integration inputs; historical records are immutable.
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync,mkdirSync} from 'node:fs'
import {execFileSync} from 'node:child_process'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),read=p=>readFileSync(resolve(root,p))
const sha=b=>createHash('sha256').update(b).digest('hex')
const previousPath='docs/qualification/own-rust-cad-v60.json',output='docs/qualification/own-rust-cad-v61.json'
if(existsSync(resolve(root,output)))throw new Error('Historical OwnRust61 already exists; publish a successor')
const previous=JSON.parse(read(previousPath)),next=structuredClone(previous)
const reportPath='docs/qualification/main-integration-20261010/summary.json',report=JSON.parse(read(reportPath))
if(report.native.status!=='passed'||report.architecture.status!=='passed')throw new Error('Native checks and architecture audit must pass before publication')
const wasm=read('public/wasm/geometry-kernel.wasm'),wasmSha=sha(wasm),artifact=`docs/qualification/artifacts/geometry-${wasmSha}.wasm`
if(report.wasm.sha256!==wasmSha||report.wasm.byteLength!==wasm.length)throw new Error('Report does not bind current packaged WASM')
mkdirSync(resolve(root,'docs/qualification/artifacts'),{recursive:true})
if(existsSync(resolve(root,artifact))) {if(sha(read(artifact))!==wasmSha)throw new Error('Archived artifact differs')}
else writeFileSync(resolve(root,artifact),wasm,{flag:'wx'})
next.id='own-rust-cad-v61';next.successorOf=previousPath;next.previousSha256=sha(read(previousPath));next.recordedAt='2026-10-11'
next.purpose='Bind the merged native Rust source, extracted crate compatibility adapters, architecture ratchet and observed rebuilt WASM. Earlier STEP/browser evidence remains historical and is not imported as current artifact qualification.'
next.dependencyFingerprints={noticesSha256:sha(read('THIRD_PARTY_NOTICES.md')),npmLockfileSha256:sha(read('package-lock.json')),rustLockfileSha256:sha(read('crates/Cargo.lock'))}
next.wasm={path:artifact,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha,byteLength:wasm.length,variants:[{sha256:wasmSha,byteLength:wasm.length,builder:'macos-aarch64',evidence:`Observed local rebuild. ${execFileSync('rustc',['--version'],{cwd:root,encoding:'utf8'}).trim()}; Node ${process.version}. No foreign-host identity or clean semantic qualification inferred.`}]}
const runtimePath='src/core/ownRustCadEvidence.ts',runtime=read(runtimePath).toString()
let updated=runtime.replace(previous.dependencyFingerprints.rustLockfileSha256,next.dependencyFingerprints.rustLockfileSha256)
updated=updated.replace(/export const OWN_RUST_CAD_ARTIFACTS = Object.freeze\([\s\S]*?\)\nexport function/,`export const OWN_RUST_CAD_ARTIFACTS = Object.freeze(${JSON.stringify([{sha256:wasmSha,byteLength:wasm.length}],null,2)})\nexport function`)
if(!updated.includes(wasmSha))throw new Error('Runtime identity refresh failed')
writeFileSync(resolve(root,runtimePath),updated)
const tracked=execFileSync('git',['ls-files','crates','src','scripts'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>/\.(rs|toml|ts|mjs|mts)$/.test(p))
next.sourceBundle.paths=[...new Set([...previous.sourceBundle.paths,...tracked,runtimePath,'crates/Cargo.lock','architecture-baseline.json','scripts/release-own-rust-v61.mjs',reportPath,'docs/qualification/main-integration-20261010/contracts.json','docs/qualification/main-integration-20261010/architecture.json','docs/qualification/main-integration-20261010/native.json','docs/qualification/main-integration-20261010/native.log'])].filter(p=>existsSync(resolve(root,p))).sort()
next.sourceBundle.sha256=sha(Buffer.concat(next.sourceBundle.paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))
next.finiteEvidenceSummary={path:reportPath,sha256:sha(read(reportPath))}
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log(`Published OwnRust61: ${wasmSha} (${wasm.length} bytes)`)
