#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {existsSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v28.json')))throw new Error('Historical own-rust-cad-v28 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v27.json',previous=JSON.parse(bytes(previousPath))
const paths=[...new Set([...previous.sourceBundle.paths, 'crates/nurbs-core/src/guided_loft.rs', 'crates/nurbs-core/src/gordon.rs', 'crates/nurbs-core/src/loft_continuity.rs', 'crates/nurbs-core/src/natural_loft.rs', 'crates/nurbs-core/src/loft_alignment.rs'])].sort()
const wasmPath='src/generated/geometry-kernels/kernel_bg.wasm',wasm=bytes(wasmPath),wasmSha256=sha(wasm)
const dependencyFingerprints={
  noticesSha256:sha(bytes('THIRD_PARTY_NOTICES.md')),
  npmLockfileSha256:sha(bytes('package-lock.json')),
  rustLockfileSha256:sha(bytes('crates/Cargo.lock')),
}
const variants=[
  {sha256:wasmSha256,byteLength:wasm.byteLength,builder:'macos-aarch64',evidence:'Clean local build with nightly-2026-09-16; Node 22 and 26 optimizers produce identical bytes.'},
  {sha256:'58889ee35bbf97aa6bb7f8bd01404928668cdddccec7a349934b797fe8d88d5c',byteLength:null,builder:'linux-x86_64',evidence:'GitHub jobs 110629675240 and 110629675250 independently report this rebuilt digest on 9f9c29a1. Length was not recorded; exact SHA-256 remains mandatory.'},
  {sha256:'c30474a3b4b6ddf0134d59a897d3b1dd1d2e93dc9c15adebdf5db47b94768b5c',byteLength:9676099,builder:'linux-aarch64',evidence:'Clean Ubuntu container build with nightly-2026-09-16 and Node 22.23.0.'},
]
writeFileSync(resolve(root,'src/core/ownRustCadEvidence.ts'),`// Recorded exact artifacts; no cross-host byte identity or clean qualification is claimed.
import packagedArtifact from '../generated/geometry-kernels/identity'
export const OWN_RUST_CAD_ARTIFACTS = Object.freeze(${JSON.stringify(variants.map(({sha256,byteLength})=>({sha256,byteLength})),null,2)})
export function recordedOwnRustCadFingerprint(artifact: {sha256:string;byteLength:number}): string|null {
  if (!Number.isSafeInteger(artifact.byteLength) || artifact.byteLength <= 0 || artifact.byteLength > 16 * 1024 * 1024) return null
  return OWN_RUST_CAD_ARTIFACTS.find(item => item.sha256 === artifact.sha256 && (item.byteLength === null || item.byteLength === artifact.byteLength))?.sha256 ?? null
}
export const OWN_RUST_CAD_EVIDENCE = Object.freeze({
  // Runtime identity follows packaged bytes; qualification freezing separately rejects unrecorded artifacts.
  "wasmSha256": packagedArtifact.sha256,
  "noticesSha256": "${dependencyFingerprints.noticesSha256}",
  "lockfileSha256": "${dependencyFingerprints.npmLockfileSha256}",
  "rustLockfileSha256": "${dependencyFingerprints.rustLockfileSha256}"
})
`)
const sourceBundle=Buffer.concat(paths.flatMap(p=>[Buffer.from(p),Buffer.from([0]),bytes(p),Buffer.from('\n')]))
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v28.json'),JSON.stringify({
  id:'own-rust-cad-v28',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-10-02',
  purpose:'Append-only fingerprint of the cleanly rebuilt loft kernel and current source bundle for G0 v36 / G1 v53. Preserve v27 as publication evidence; no qualification result is imported.',qualificationClaim:'none',
  wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength,variants},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This enumerates exact observed build artifacts; compiler hosts are not assumed to produce identical bytes. It is not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v28.json and refreshed src/core/ownRustCadEvidence.ts')
