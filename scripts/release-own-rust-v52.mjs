#!/usr/bin/env node
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve,sep} from 'node:path'
const root=resolve(import.meta.dirname,'..'),bytes=p=>readFileSync(resolve(root,p)),sha=v=>createHash('sha256').update(v).digest('hex')
if(existsSync(resolve(root,'docs/qualification/own-rust-cad-v52.json')))throw new Error('Historical own-rust-cad-v52 evidence already exists; publish a new version instead of overwriting it')
const prepareArgument=process.argv.find(value=>value.startsWith('--prepare='))
const preparedPath=prepareArgument?.slice('--prepare='.length)
if(prepareArgument&&!preparedPath)throw new Error('A prepared metadata path is required')
if(preparedPath&&(resolve(preparedPath)===root||resolve(preparedPath).startsWith(root+sep)))throw new Error('Prepared metadata must stay outside the checkout and its historical archives')
const previousPath='docs/qualification/own-rust-cad-v51.json',previous=JSON.parse(bytes(previousPath))
const changed=[...execFileSync('git',['diff','--name-only','HEAD'],{cwd:root,encoding:'utf8'}).trim().split('\n'),...execFileSync('git',['ls-files','--others','--exclude-standard'],{cwd:root,encoding:'utf8'}).trim().split('\n')]
const additions=changed.filter(path=>path==='docs/design/sweep-qualification-catalog.json'||path==='docs/qualification/historical-record-restoration-20261009.json'||path.startsWith('docs/qualification/native-integration-20261009/')||/^(crates\/.*\.(rs|toml|json)|src\/.*\.(ts|vue|css)|tests\/.*\.(ts|mjs)|scripts\/.*\.(mjs|mts)|vite.*\.ts|docs\/languages\/.*\.json)$/.test(path))
const deliveryPaths=['scripts/qualificationKernelModuleDelivery.mjs','src/services/geometry/kernelCompilationRequired.ts','src/workers/manifoldPlanQualification.worker.ts']
const paths=[...new Set([...previous.sourceBundle.paths.filter(p=>existsSync(resolve(root,p))),...additions,...deliveryPaths,...execFileSync('git',['ls-files','crates/brep-core/src','crates/nurbs-core/src','crates/geometry-bridge/src','crates/bridge-cam/src','crates/bridge-analysis/src','crates/bridge-svg/src'],{cwd:root,encoding:'utf8'}).trim().split('\n').filter(p=>p.endsWith('.rs'))])].filter(p=>existsSync(resolve(root,p))).sort()

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
writeFileSync(preparedPath?resolve(preparedPath):resolve(root,'docs/qualification/own-rust-cad-v52.json'),JSON.stringify({
  id:'own-rust-cad-v52',successorOf:previousPath,previousSha256:sha(bytes(previousPath)),recordedAt:'2026-10-09',
  purpose:'Merged native geometry domains, original-parameter tangent routing, source law payload ownership, UV boundary rays and whole-chart fixed-projection injectivity. The anisotropic hollow profile and 1024-face spatial RMF affine hollow fixture pass native complete shell and Solid admission. The two-shell fixture fits the former 20000 linear-cell allowance; the three-cavity fixture requires 28180 cells and now has an explicit 30000 default allowance. Individual-pair and contact-cell limits remain unchanged. Singleton control-hull ranges retain complete pair accounting. Packaging and finite regressions are recorded separately; arbitrary global geometry and full clean G1 qualification remain unproved.',qualificationClaim:'none',
  wasm:{path:wasmPath,publishedPath:'public/wasm/geometry-kernel.wasm',sha256:wasmSha256,byteLength:wasm.byteLength,variants},
  sourceBundle:{canonicalization:'UTF-8 path sorted: path + NUL + exact file bytes + LF',sha256:sha(sourceBundle),paths},
  dependencyFingerprints,
  finiteEvidenceSummary:{path:'docs/qualification/native-integration-20261009/summary.json',sha256:sha(bytes('docs/qualification/native-integration-20261009/summary.json'))},
  claimBoundary:[
    'This enumerates exact observed build artifacts; compiler hosts are not assumed to produce identical bytes. It is not an external clean qualification run.',
    'It covers only the finite explicit topology, correspondence, direct-cell interchange and typed-refusal matrix.',
    'It does not approve G0, G1, production cutover, silent healing, tolerance growth, arbitrary branching or singular topology.',
  ],
},null,2)+'\n',preparedPath?{}:{flag:'wx'})
console.log(preparedPath?'Prepared exact artifact metadata without publishing qualification evidence: '+resolve(preparedPath):'Recorded docs/qualification/own-rust-cad-v52.json and refreshed src/core/ownRustCadEvidence.ts')
