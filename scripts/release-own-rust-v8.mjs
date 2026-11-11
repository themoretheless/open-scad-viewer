#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
const previousPath='docs/qualification/own-rust-cad-v7.json',previous=JSON.parse(bytes(previousPath))
const paths=[...new Set([...previous.sourceBundle.paths,
 'crates/brep-core/src/iges_interchange_v2.rs','crates/brep-core/src/step_interchange_v3.rs',
 'crates/brep-core/src/lib.rs','crates/geometry-bridge/src/lib.rs','crates/geometry-bridge/src/tests.rs',
 'src/services/cadIges.ts','src/services/cadNurbsStep.ts','src/services/geometry/brepCapability.ts',
 'tests/brepInterchangeV11.test.ts','tests/brepQualificationV11.test.ts',
])].sort()
const wasmPath='src/generated/geometry-kernels/kernel_bg.wasm',wasm=bytes(wasmPath),wasmSha256=sha(wasm)
writeFileSync(resolve(root,'src/core/ownRustCadEvidence.ts'),`// Exact artifact checked by the own Rust CAD qualification corpus.
export const OWN_RUST_CAD_EVIDENCE = Object.freeze({
  "wasmSha256": "${wasmSha256}",
  "noticesSha256": "${previous.dependencyFingerprints.noticesSha256}",
  "lockfileSha256": "${previous.dependencyFingerprints.npmLockfileSha256}",
  "rustLockfileSha256": "${previous.dependencyFingerprints.rustLockfileSha256}"
})
`)
const sourceBundle=Buffer.concat(paths.flatMap(p=>[Buffer.from(p),Buffer.from([0]),bytes(p),Buffer.from('\n')]))
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v8.json'),JSON.stringify({
 id:'own-rust-cad-v8',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-09-17',
 purpose:'Append-only current-byte fingerprint for V11 direct IGES and STEP interchange successors.',qualificationClaim:'none',
 wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength},
 sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
 dependencyFingerprints:previous.dependencyFingerprints,
 claimBoundary:['This is a reproducible byte fingerprint, not an external clean qualification run.','It covers only the qualified finite direct interchange cells and their explicit typed refusals.','It does not approve G0, G1, production cutover, mesh fallback, healing, arbitrary assemblies or external references.'],
},null,2)+'\n')
