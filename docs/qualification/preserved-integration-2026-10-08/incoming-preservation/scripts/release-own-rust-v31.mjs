#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {existsSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v31.json')))throw new Error('Historical own-rust-cad-v31 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v30.json',previous=JSON.parse(bytes(previousPath))
const paths=[...previous.sourceBundle.paths].sort()
const wasmPath='src/generated/geometry-kernels/kernel_bg.wasm',wasm=bytes(wasmPath),wasmSha256=sha(wasm)
const dependencyFingerprints={
  noticesSha256:sha(bytes('THIRD_PARTY_NOTICES.md')),
  npmLockfileSha256:sha(bytes('package-lock.json')),
  rustLockfileSha256:sha(bytes('crates/Cargo.lock')),
}
writeFileSync(resolve(root,'src/core/ownRustCadEvidence.ts'),`// Exact artifact checked by the own Rust CAD qualification corpus.
export const OWN_RUST_CAD_EVIDENCE = Object.freeze({
  "wasmSha256": "${wasmSha256}",
  "noticesSha256": "${dependencyFingerprints.noticesSha256}",
  "lockfileSha256": "${dependencyFingerprints.npmLockfileSha256}",
  "rustLockfileSha256": "${dependencyFingerprints.rustLockfileSha256}"
})
`)
const sourceBundle=Buffer.concat(paths.flatMap(p=>[Buffer.from(p),Buffer.from([0]),bytes(p),Buffer.from('\n')]))
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v31.json'),JSON.stringify({
  id:'own-rust-cad-v31',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-10-06',
  purpose:'Append-only current-byte fingerprint re-issued for the G0 v39 / G1 v56 re-freeze. Drift since G0 v38 / G1 v55: the manifold library reached production scope (Full repair, mesh metrics and guaranteed-manifold booleans exposed through the geometry-kernel ABI), the concurrent process restructured brep-core direct editing and rebound qualification documents to current bytes, and crates/Cargo.lock gained the manifold-ops workspace dependency, so the rebuilt packed kernel WASM, the source-bundle digest and the Rust lockfile fingerprint changed; the V12 bounded non-manifold and mixed-dimensional topology scope is unchanged.',qualificationClaim:'none',
  wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This is a reproducible byte fingerprint, not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v31.json and refreshed src/core/ownRustCadEvidence.ts')
