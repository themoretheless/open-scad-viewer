#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {existsSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v21.json')))throw new Error('Historical own-rust-cad-v21 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v20.json',previous=JSON.parse(bytes(previousPath))
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
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v21.json'),JSON.stringify({
  id:'own-rust-cad-v21',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-09-29',
  purpose:'Append-only current-byte fingerprint re-issued for the G0 v29 / G1 v46 re-freeze. Drift since G0 v28 / G1 v45: the own-Rust geometry kernel was rebuilt from current crate sources (mesh intersection enumeration, NURBS distance and trim work on the feature branch), the source bundle files crates/brep-core/src/operations.rs, crates/brep-core/src/lib.rs, crates/geometry-bridge/src/lib.rs and src/services/geometry/brep.ts changed, package.json gained the test:cad-roadmap script, and tests/manifoldPlanBackend.test.ts now also lists the CadQuantityInput and SceneObjectControls components (neither imports manifold-3d). Notices and lockfiles are byte-identical to own-rust-cad-v20, and the V12 bounded non-manifold and mixed-dimensional topology scope is unchanged.',qualificationClaim:'none',
  wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This is a reproducible byte fingerprint, not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v21.json and refreshed src/core/ownRustCadEvidence.ts')
