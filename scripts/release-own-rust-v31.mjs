#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {existsSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v31.json')))throw new Error('Historical own-rust-cad-v31 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v30.json',previous=JSON.parse(bytes(previousPath))
const paths=[...new Set([...previous.sourceBundle.paths, 'crates/nurbs-core/src/guided_loft.rs', 'crates/nurbs-core/src/gordon.rs', 'crates/nurbs-core/src/loft_continuity.rs', 'crates/nurbs-core/src/natural_loft.rs', 'crates/nurbs-core/src/loft_alignment.rs', 'crates/nurbs-core/src/loft_reparameterization.rs', 'crates/nurbs-core/src/reparameterization_retention.rs', 'crates/nurbs-core/src/reparameterization_residual.rs', 'crates/nurbs-core/src/gordon_tangent_audit.rs', 'crates/nurbs-core/src/gordon_tangent_fields.rs', 'crates/nurbs-core/src/transport.rs', 'crates/modelgraph-runtime/src/nurbs.rs', 'crates/modelgraph-text/src/lower.rs', 'docs/languages/modelgraph-nurbs-1.schema.json', 'src/services/nurbsFoundation.ts', 'src/services/nurbsConstructors.ts', 'src/services/modelGraphNurbsKernel.ts', 'crates/cad-predicates/src/arithmetic.rs', 'crates/cad-predicates/src/bezier_identity.rs', 'crates/cad-predicates/src/bezier_strip_jets.rs', 'crates/cad-predicates/src/projective_strip_jets.rs', 'crates/cad-predicates/src/lib.rs', 'crates/nurbs-core/src/lib.rs', 'crates/nurbs-core/src/continuity.rs', 'crates/nurbs-core/src/continuity/regularity.rs', 'crates/nurbs-core/src/continuity/station_scale.rs', 'crates/nurbs-core/src/continuity/exact_strip.rs', 'crates/nurbs-core/src/retained_wall_coefficients.rs'])].sort()
const wasmPath='src/generated/geometry-kernels/kernel_bg.wasm',wasm=bytes(wasmPath),wasmSha256=sha(wasm)
const dependencyFingerprints={
  noticesSha256:sha(bytes('THIRD_PARTY_NOTICES.md')),
  npmLockfileSha256:sha(bytes('package-lock.json')),
  rustLockfileSha256:sha(bytes('crates/Cargo.lock')),
}
const variants=[
  {sha256:wasmSha256,byteLength:wasm.byteLength,builder:'macos-aarch64',evidence:'Observed local rebuild with pinned nightly-2026-09-16 and Node 22.23.3. No foreign-host artifact is inferred.'},
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
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v31.json'),JSON.stringify({
  id:'own-rust-cad-v31',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-10-04',
  purpose:'Append-only fingerprint of the cleanly rebuilt native strip and retained-family APIs for G0 v39 / G1 v56. Preserve v30 as publication evidence; no qualification result is imported.',qualificationClaim:'none',
  wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength,variants},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This enumerates exact observed build artifacts; compiler hosts are not assumed to produce identical bytes. It is not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v31.json and refreshed src/core/ownRustCadEvidence.ts')
