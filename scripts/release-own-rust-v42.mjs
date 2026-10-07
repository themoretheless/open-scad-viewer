#!/usr/bin/env node
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v42.json')))throw new Error('Historical own-rust-cad-v42 evidence already exists; publish a new version instead of overwriting it')
const previousPath='docs/qualification/own-rust-cad-v41.json',previous=JSON.parse(bytes(previousPath))
const changed=[...execFileSync('git',['diff','--name-only','HEAD'],{cwd:root,encoding:'utf8'}).trim().split('\n'),...execFileSync('git',['ls-files','--others','--exclude-standard'],{cwd:root,encoding:'utf8'}).trim().split('\n')]
const additions=changed.filter(path=>/^(crates\/.*\.(rs|toml)|src\/services\/.*\.ts|src\/workers\/geometry\.worker\.ts|src\/App\.vue|docs\/languages\/.*\.json)$/.test(path))
const paths=[...new Set([...previous.sourceBundle.paths,...additions,...execFileSync('git',['ls-files','crates/brep-core/src','crates/nurbs-core/src','crates/geometry-bridge/src'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>p.endsWith('.rs'))])].sort()

const wasm=bytes('public/wasm/geometry-kernel.wasm'),wasmSha256=sha(wasm)
const wasmPath=`docs/qualification/artifacts/geometry-${wasmSha256}.wasm`
mkdirSync(resolve(root,'docs/qualification/artifacts'),{recursive:true})
if(existsSync(resolve(root,wasmPath))) {
  if(sha(bytes(wasmPath))!==wasmSha256)throw new Error('Archived release binary changed')
} else writeFileSync(resolve(root,wasmPath),wasm,{flag:'wx'})
const dependencyFingerprints={
  noticesSha256:sha(bytes('THIRD_PARTY_NOTICES.md')),
  npmLockfileSha256:sha(bytes('package-lock.json')),
  rustLockfileSha256:sha(bytes('crates/Cargo.lock')),
}
const variants=[{sha256:wasmSha256,byteLength:wasm.byteLength,builder:'macos-aarch64',evidence:`Observed local rebuild. ${execFileSync('rustc',['--version'],{cwd:root,encoding:'utf8'}).trim()}; Node ${process.version}. No foreign-host artifact is inferred.`}]


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
writeFileSync(resolve(root,'docs/qualification/own-rust-cad-v42.json'),JSON.stringify({
  id:'own-rust-cad-v42',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-10-07',
  purpose:'Append-only native whole-surface periodic embedding, generalized periodic seam and open spatial frame-bound evidence. Preserve all earlier release bytes; no clean qualification results are imported.',qualificationClaim:'none',
  wasm:{path:wasmPath,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha256,byteLength:wasm.byteLength,variants},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  claimBoundary:[
    'This enumerates exact observed build artifacts; compiler hosts are not assumed to produce identical bytes. It is not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n')
console.log('Recorded docs/qualification/own-rust-cad-v42.json and refreshed src/core/ownRustCadEvidence.ts')
