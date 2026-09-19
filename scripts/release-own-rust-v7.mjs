#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'

const root=resolve(import.meta.dirname,'..')
const bytes=path=>readFileSync(resolve(root,path))
const sha=value=>createHash('sha256').update(value).digest('hex')
const previousPath='docs/qualification/own-rust-cad-v6.json'
const previous=JSON.parse(bytes(previousPath))
const paths=[...new Set([
 ...previous.sourceBundle.paths,
 'crates/brep-core/src/analysis.rs',
 'crates/brep-core/src/intersections.rs',
 'crates/brep-core/src/solid_audit.rs',
 'crates/brep-core/src/trim_sew.rs',
 'crates/geometry-bridge/src/brep.rs',
 'tests/brepQualificationV10.test.ts',
])].sort()
const wasmPath='src/generated/geometry-kernels/kernel_bg.wasm'
const wasm=bytes(wasmPath)
const wasmSha256=sha(wasm)
writeFileSync(resolve(root,'src/core/ownRustCadEvidence.ts'),`// Exact artifact checked by the own Rust CAD qualification corpus.
export const OWN_RUST_CAD_EVIDENCE = Object.freeze({
  "wasmSha256": "${wasmSha256}",
  "noticesSha256": "${previous.dependencyFingerprints.noticesSha256}",
  "lockfileSha256": "${previous.dependencyFingerprints.npmLockfileSha256}",
  "rustLockfileSha256": "${previous.dependencyFingerprints.rustLockfileSha256}"
})
`)
const sourceBundle=Buffer.concat(paths.flatMap(path=>[
 Buffer.from(path),Buffer.from([0]),bytes(path),Buffer.from('\n'),
]))
const artifact={
 id:'own-rust-cad-v7',
 successorOf:previousPath,
 previousSha256:sha(bytes(previousPath)),
 recordedAt:'2026-09-17',
 purpose:'Append-only current-byte fingerprint for V10 certified analytic tessellation and mass-property successors.',
 qualificationClaim:'none',
 wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength},
 sourceBundle:{
  canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',
  sha256:sha(sourceBundle),paths,
 },
 dependencyFingerprints:previous.dependencyFingerprints,
 claimBoundary:[
  'This is a reproducible byte fingerprint, not an external clean qualification run.',
  'It covers only the released finite analytic shell cells; generic rational/freeform, graph/multispan Boolean, nonplanar loft and bent/twisted/scaled sweep certification remains refused.',
  'It does not approve G0, G1, production cutover, mesh fallback, healing or estimates relabeled certified.',
 ],
}
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v7.json'),`${JSON.stringify(artifact,null,2)}\n`)
