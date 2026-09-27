#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {existsSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v19.json')))throw new Error('Historical own-rust-cad-v19 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v18.json',previous=JSON.parse(bytes(previousPath))
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
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v19.json'),JSON.stringify({
  id:'own-rust-cad-v19',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-09-27',
  purpose:'Append-only current-byte fingerprint re-issued for the G0 v27 / G1 v44 re-freeze after main rebuilt the geometry kernel: 72a9a6e added sketch dimensions and associative G2 bridge curves to geometry-bridge (new cad_dimensions and cad_bridge_curve modules plus a one-line cad_sketch_trim change). The path-independent WASM is rebuilt on the merged sources. THIRD_PARTY_NOTICES.md, package-lock.json and crates/Cargo.lock are byte-identical to own-rust-cad-v18; the own-Rust source bundle differs only in geometry-bridge/src/lib.rs, which declares the two new modules and registers the new cad_dimensions and cad_bridge_curve operations in the dispatcher; existing operations are untouched. The own-Rust oracle v3 (44 cases) passes unchanged on the rebuilt kernel, and the V12 bounded non-manifold and mixed-dimensional topology scope is unchanged.',qualificationClaim:'none',
  wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This is a reproducible byte fingerprint, not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v19.json and refreshed src/core/ownRustCadEvidence.ts')
