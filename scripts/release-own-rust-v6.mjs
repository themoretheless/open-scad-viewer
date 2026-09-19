#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'

const root=resolve(import.meta.dirname,'..')
const bytes=path=>readFileSync(resolve(root,path))
const sha=value=>createHash('sha256').update(value).digest('hex')
const previousPath='docs/qualification/own-rust-cad-v5.json'
const previous=JSON.parse(bytes(previousPath))
const paths=[...new Set([
 ...previous.sourceBundle.paths,
 'crates/brep-core/src/analytic.rs',
 'crates/brep-core/src/analytic/loft.rs',
 'tests/brepQualificationV9.test.ts',
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
 id:'own-rust-cad-v6',
 successorOf:previousPath,
 previousSha256:sha(bytes(previousPath)),
 recordedAt:'2026-09-17',
 purpose:'Append-only current-byte fingerprint for the V9 exact multi-section loft and bent RMF sweep successors.',
 qualificationClaim:'none',
 wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength},
 sourceBundle:{
  canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',
  sha256:sha(sourceBundle),paths,
 },
 dependencyFingerprints:previous.dependencyFingerprints,
 claimBoundary:[
  'This is a reproducible byte fingerprint, not an external clean qualification run.',
  'It does not approve G0, G1, curved degree-greater-than-1 paths, freeform sections, cusps, singular frames, invalid scale/twist, self-intersection, closed-loop holonomy, mesh, healing or fallback.',
 ],
}
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v6.json'),`${JSON.stringify(artifact,null,2)}\n`)
