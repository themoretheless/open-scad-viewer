#!/usr/bin/env node
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {existsSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v37.json')))throw new Error('Historical own-rust-cad-v37 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v36.json',previous=JSON.parse(bytes(previousPath))
const changed=[...execFileSync('git',['diff','--name-only','HEAD'],{cwd:root,encoding:'utf8'}).trim().split('\n'),...execFileSync('git',['ls-files','--others','--exclude-standard'],{cwd:root,encoding:'utf8'}).trim().split('\n')]
const additions=changed.filter(path=>/^(crates\/.*\.(rs|toml)|src\/services\/.*\.ts|src\/workers\/geometry\.worker\.ts|src\/App\.vue|docs\/languages\/.*\.json)$/.test(path))
const paths=[...new Set([...previous.sourceBundle.paths,...additions])].sort()

const wasmPath='src/generated/geometry-kernels/kernel_bg.wasm',wasm=bytes(wasmPath),wasmSha256=sha(wasm)
const dependencyFingerprints={
  noticesSha256:sha(bytes('THIRD_PARTY_NOTICES.md')),
  npmLockfileSha256:sha(bytes('package-lock.json')),
  rustLockfileSha256:sha(bytes('crates/Cargo.lock')),
}
const observed=JSON.parse(readFileSync('/tmp/sweep-124-observed-linux-artifacts.json','utf8'))
const ciBytes=readFileSync('/tmp/sweep-124-linux-node22/kernel_bg.wasm')
if(sha(ciBytes)!==observed.variants[1].sha256 || ciBytes.byteLength!==observed.variants[1].byteLength)throw new Error('Downloaded CI artifact changed')
const variants=[...previous.wasm.variants,{sha256:sha(ciBytes),byteLength:ciBytes.byteLength,builder:'github-actions-ubuntu-latest',evidence:'Observed downloaded artifacts 11305156565 and 11305117762 from CI run 37206543588 at head 124eea3b4e2c331959908badb512238fc6efd46f. Node 20.19 and 22 bytes match. CI qualification remains pending; no result is imported.'}]
if(wasmSha256!==previous.wasm.sha256 || wasm.byteLength!==previous.wasm.byteLength)throw new Error('Local artifact changed')


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
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v37.json'),JSON.stringify({
  id:'own-rust-cad-v37',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-10-04',
  purpose:'Append-only artifact and source fingerprint for expanded native sweep/miter laws, boundary certificates, retained decomposition, continuity, Rust/WASM/Rush transport and viewport integration, including separately observed Linux CI build identity. Preserve v36; no qualification result is imported.',qualificationClaim:'none',
  wasm:{path:wasmPath,sha256:wasmSha256,byteLength:wasm.byteLength,variants},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This enumerates exact observed build artifacts; compiler hosts are not assumed to produce identical bytes. It is not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v37.json and refreshed src/core/ownRustCadEvidence.ts')
