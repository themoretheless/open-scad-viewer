#!/usr/bin/env node
// Explicit append-only re-freeze. This script never executes a qualification row.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { REFREEZE_CORE, digest, jsonBytes, shaRecord, bundleBytes, ordinaryBytes, snapshotFiles, publishExclusive, assertNoCandidateResults as assertCandidateResultsAbsent } from './qualificationRefreezeCore.mjs'

const defaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
export const REFRESH_SCRIPT = 'scripts/refresh-qualification-fingerprints.mjs'
export const REFRESH_REVIEW = 'docs/qualification/g0-v22-g1-v39-refreeze-review.md'
const OWN_RUST_EVIDENCE = 'docs/qualification/own-rust-cad-v14.json'
export const REFRESH_EXECUTORS = Object.freeze([
  '.github/workflows/g1-qualification-clean.yml',
  'scripts/g1-github-actions.mjs',
  'scripts/run-g1-candidate-clean-fragment.mjs',
  'scripts/run-g1-ubuntu-docker-fragment.mjs',
])
export const REFRESH_OUTPUTS = Object.freeze({
  fingerprint: 'docs/qualification/g0-toolchain-fingerprints-v22.json',
  plan: 'docs/qualification/semantic-manifold-g1-plan-v39.json',
  status: 'docs/qualification/g0-v22-g1-v39-refreeze-status-v1.json',
})
const oldFingerprint = 'docs/qualification/g0-toolchain-fingerprints-v21.json'
const oldPlan = 'docs/qualification/semantic-manifold-g1-plan-v38.json'
const oldStatus = 'docs/qualification/g0-v21-g1-v38-refreeze-status-v1.json'
const oldReview = 'docs/qualification/g0-v21-g1-v38-refreeze-review.md'
const githubEnvironment = 'docs/qualification/environment-freeze/g1-github-actions-v34.json'
const historicalPlanHashes = [
  '050a1dd7a30d19dd85a8ed16fd724f7579c2430f1d7bf03ed68009d46bd2cbfa',
  '90452963dfdc47e492725c6bcd60a2dcdc7ffa748f848e95831c5d5750f5f0a2',
  '40c7d885b34013270f45b4f2cd7ce29bb83293ee61bf8c60c11a6613bd3f282c',
  '4d8b69fd2afeb6c828f0ab79b1ddaf6dac68b83a5b9b8bc0a3f59a5b63320ee1',
  'a5c4b365263d74b6d539d7f6539bcc4a571d055bd518b70ec1bb93dd6df2d9ee',
  '5a0ef0359fd6c1f5ddad43715d621cd549da9a319dde9454e06dec9e67835566',
  '6cf7e79bd0240b41ec7dbd7acedeb9a5818e72af13803e976ca19cf57d77d000',
  'ccede0404cc9655c9acac317dbf9b63bf1ee4aa3453c0cd3f337cf5b9d7b5b5f',
  '0efdfe73a0b43f5190d2cc66a594dd1909399852d6a6a485fd41997fe563cf21',
  '0ee191fbf83e80b95ec940e2f8286e4c5d80ce214941f1c4aba57405cb7f4ec9',
  'e758898a3539a91360ad4e52458dc4d0413e6c000f55043bdf0a216c496e3d0d',
  '3534a6f16b6443cd6898863b86985feb4278aebcc6024faadc0423a0de51faea',
  'c736b508bade157c2a466c35963953e982ec4ef080219c7b0c271d4a1f0ada1b',
  '15ff9a8eb6631ae9a425853b72df88384c9e218bdc46e83f1a3414961b90fb41',
  'fecc5f2f3b08b98549f9b34f3c80c659cfb698b889e46461c9726acfdd4a9a1a',
  'f421e75e4ef19ffc6cef0d36745428ea086e3797b911ed84dd00a5c2636b1622',
  'ce0f2121aa0014dcc9439e63654e71502fa33f13a07564a65a8e0ec22ff993e4',
  '7e129bd193d33ee6c4205c26e6f9b5747e13d48cea06800946a1ef639f5f99ce',
  '14fb0fdc79532dcd9a8bed3a93128f152fe3b68cc56529b6cc83561b5b899ed3',
  'c5d903ba5b7383ea79df20626de6f24c1cef9a564bb977adde5b5a362d75b9e6',
  'bfb889481c1a1b1f360380599858be9d117500e9f3ceb311cc8a642a7a10682b',
  '63a55e60751a35af3c676de1a537477e845ea2439db7199cb4e14e89bdd56ea6',
  'a62609ebefe2f07994266b3bce8641c368de8280d7ff690d318f411c74810c0d',
  '05f572b7afd1311f7056b2cb3102ef117d67a7d8cb48c5a81057685d0bc5b7c4',
  '2361b59e14acadd72b94d348083390cd3ca74a906049aea582e37453297889c8',
  '0016bbc4ef57c02e17f61a9f4d1d84e21d37c444f15e061f77554bf357f12967',
  '84f3fe996f3694b3a051e077ed4150a1e64d81c390559f44df69213bdc216866',
  '04a958f79231395c9a009e20a5b7d3abe91246e8fbfc5a1bccb89abeeb6bf79e',
  '509e61dc8c915ef19303927c9f025868137e3112fc95fede8970370ccd76220c',
  '466890322e840b9dacb2fd449ba7ac5fee972fd0c7dc22d616e5eaf06c143047',
  'b0303dbc26372b348078902dfd4a85d707bf2522eaa114423b6062a93b374052',
  'c4f7abb174659a84d7f5167ee04e5d3ff79746b0048ba84e428ad94e9e8765db',
  '27b756158efdee25758bb012394fc056d1fb676e4f87af393ec2a4c4d784fbda',
]
export const FROZEN_ARCHIVES = Object.freeze({
  'docs/qualification/g0-toolchain-fingerprints-v1.json': '4c2a286ea9567ddef66d2a0335a4960bddc94921b662c9e34af063b480cb8af3',
  'docs/qualification/g0-toolchain-fingerprints-v2.json': 'd040f252ce41177e24b02491beef59ccb57771e99fc0efaf9f0e7e4b2a081e76',
  'docs/qualification/g0-toolchain-fingerprints-v3.json': 'c8c74e76f7e6ce9e85c8f1c4267e678ca8c243b16ec4e6dd050584b489c0c171',
  'docs/qualification/g0-toolchain-fingerprints-v4.json': '38b9fac7b3e03a301b0dfdeadf514572f337feff37b186089d968f3afa9e5a46',
  'docs/qualification/g0-toolchain-fingerprints-v5.json': '9da6b4921492e2f6592194b58f144d6c7460f3c6787717f1b311cf7040428040',
  'docs/qualification/g0-toolchain-fingerprints-v6.json': 'ef2f25017daec1d75fae2725a85a500fa59d36c448f8a5ba0b9407f90a291eaf',
  'docs/qualification/g0-toolchain-fingerprints-v7.json': '6a364833e07040d59cd47564bfb791ff97f9bca812239b5f72ce19423a23dcbf',
  'docs/qualification/g0-toolchain-fingerprints-v8.json': '8446a3193659cf9325579e9f1286d8367a517eb9af780950357d35121e65ec62',
  'docs/qualification/g0-toolchain-fingerprints-v10.json': 'cc27233347f9b177691ed08ae77e2eb7a38777133f6451e61b6f3ab160ca7ac3',
  'docs/qualification/g0-toolchain-fingerprints-v11.json': '5ae685b005bbbeaf3e7c84cf06011f4136e997cf7f6dfab38e439c72ca34ec19',
  'docs/qualification/g0-toolchain-fingerprints-v12.json': '1af08861746409347f5f2b931c7d794dffc9df3c0321c27cb534b93a3eac3be7',
  'docs/qualification/g0-toolchain-fingerprints-v16.json': '4d74667880d6214105fb8d6f0c423ef1a016c45fdd011ef336431c33521b749c',
  'docs/qualification/g0-toolchain-fingerprints-v17.json': 'c331887ec7e77d9d7c641eafa322adf6d6c41e07efcf5d92a9530d4bb1abd9a0',
  'docs/qualification/g0-toolchain-fingerprints-v18.json': '2dbc8fd89e57620112638d8d57714aef555470d6d89b3b9c1ee886168ca796ab',
  'docs/qualification/g0-toolchain-fingerprints-v19.json': 'b928af58435dfeada9a68de5c65e07f8d25c43c9af622e2b6c80b6db141419b5',
  'docs/qualification/g0-toolchain-fingerprints-v20.json': '4ce1d2a7855d22684f59ed6ab1c20537b2f957a4cba1c98509d4b8b14927ea73',
  [oldFingerprint]: '88206f45c60ca20698e20fd3c4a7a1dbca43d70793d7db7eace2c554c0bb97fe',
  'docs/qualification/semantic-manifold-g1-plan-v34.json': 'e89a6a738d2ba22709fe38fbd7ccc5108743820118a019658be2bb3323a549db',
  'docs/qualification/semantic-manifold-g1-plan-v35.json': '72ab87d35b71a43cb92f6e937df41178db650ef18bfc3e09737b38186c9cf031',
  'docs/qualification/semantic-manifold-g1-plan-v36.json': '6a0611b45b2cdee900807f112657143fe7942ce4a69bbc505b53c15f608186f2',
  'docs/qualification/semantic-manifold-g1-plan-v37.json': '1532e6f73c61cdbcbe8da17a79f4738c2b8bbae31eaf487112a2bb40e053792c',
  'docs/qualification/semantic-manifold-g1-plan-v38.json': '360821de40c7e846028491c177e3a73741d49d9c5491529fae5afe57d3196d22',
  'docs/qualification/g0-v17-g1-v34-refreeze-status-v1.json': 'ad10d6a990af2fd89d918c77332738bd5021e521904923b73503ee86f64ebde6',
  'docs/qualification/g0-v18-g1-v35-refreeze-status-v1.json': '13ccd67872d2b3453f94841e0157b25fbf016bff3e97a90412b8dfbd0a1d595a',
  'docs/qualification/g0-v19-g1-v36-refreeze-status-v1.json': 'f340f0c350dd5facae5e898a36806f272e7dc877b3517461b6f30f3d52a92f25',
  'docs/qualification/g0-v20-g1-v37-refreeze-status-v1.json': '83ea9b7f47236409c80e6b3f844dfd080a7b23857a363366174e6cc55122f436',
  [oldStatus]: '8e7baef32969ba130e7882aae4d908fee74e7f8f1b14f4a6e4a7270886b251fa',
  'docs/qualification/g0-v17-g1-v34-refreeze-review.md': '6b5d012113950b23710dc2e7e0d24944f0a742303a8383d4444d6eb391e5befc',
  'docs/qualification/g0-v18-g1-v35-refreeze-review.md': 'd679c5f98e0ec80b30cca43ff58651eebc9fa53a9ba475229965eeb6db1898c1',
  'docs/qualification/g0-v19-g1-v36-refreeze-review.md': 'd72a0ca39c17fdf7daa62d655cd4944ab32cc8229c205edc869cfb747f1b8de2',
  'docs/qualification/g0-v20-g1-v37-refreeze-review.md': 'f020e018a61e6cf9bb0fcbc4819f5a628b61d8fa8f1ed8826660f2529378f6de',
  [oldReview]: 'bb04f6215550df547617c01de3da5f9e32f997807952e0b880f59a359545fd31',
  'docs/qualification/g0-v3-g1-v19-refreeze-status-v1.json': '1c22fe7e7a5451a067be0b6b99b62a4f2c3be2e8067518b10d06d04c51c68b25',
  'docs/qualification/g1-v20-refreeze-status-v1.json': 'ec3fc7f7a87ceda8f6269cb2271648d9418167227c31887bf2b9614961d705d9',
  'docs/qualification/g0-v4-g1-v21-refreeze-status-v1.json': '1c1ab4668ea6a47f57d0f3316688e182b1b87ae5c78bccea902ca57f528b586d',
  'docs/qualification/g0-v5-g1-v22-refreeze-status-v1.json': 'e3229477708ad67db3f632225ac89a6be5de91f7e2a07aaa8c3c06740a2cf84e',
  'docs/qualification/g0-v6-g1-v23-refreeze-status-v1.json': 'f3e9033d518d14f3fd317974cfb8bb1b4706ec5dda9661b9e011ff5ee2e593d7',
  'docs/qualification/g0-v7-g1-v24-refreeze-status-v1.json': '4843096898d7ec49e7f2ecd7a84e87a2364db4aaa26a333e3e2f8be9fb7b7887',
  'docs/qualification/g0-v8-g1-v25-refreeze-status-v1.json': '8d9edcf9f625cfa9620f87d597c05e259d568bda5863e2d3aa13b6ca7c850845',
  'docs/qualification/g0-v10-g1-v27-refreeze-status-v1.json': 'faef038c289bccc878be5de8a6b9abfc33fffa614adc7da3295e35479d9a7973',
  'docs/qualification/g0-v11-g1-v28-refreeze-status-v1.json': '43dce53f3c2423996aefb64cbb08af020e24ff5fe758cb1573a7bb950684c068',
  'docs/qualification/g0-v12-g1-v29-refreeze-status-v1.json': '99d779b63aa1689a691c4152b616b3712000bc9f06f8648a871f713b118be25c',
  'docs/qualification/g0-v16-g1-v33-refreeze-status-v1.json': '7ca84a52754237b09ce0d0e2baaa0a79f18516fcca858b6d397a747c36df5f10',
  'docs/qualification/g0-v6-g1-v23-refreeze-review.md': '182970eb33bcb479d6b0c4258f665d5023a13ee059a39644fce2a202b0f75666',
  'docs/qualification/g0-v7-g1-v24-refreeze-review.md': '97ccdb97fae739eeea04d3301287adb3ebc78ce3073c4ca4ca67507b8d328df6',
  'docs/qualification/g0-v8-g1-v25-refreeze-review.md': 'ad46d89bdfbaf1b8883f85e5c4079a8ce5727db3615770b3c596fafce23f9fc5',
  'docs/qualification/g0-v9-g1-v26-refreeze-review.md': 'ea9f86e6cea8f8192b38da470339d144a5ab15dfc02499b2ba2fa367a392de55',
  'docs/qualification/g0-v10-g1-v27-refreeze-review.md': '26746d7642c58877e7725baa75803a52f5b2811bbbd4dcae62d2e916a36f4d86',
  'docs/qualification/g0-v11-g1-v28-refreeze-review.md': '4e34ca2cc6344772f169252a5ab33e3257f7a341baf288aafe4ba109c67e2210',
  'docs/qualification/g0-v12-g1-v29-refreeze-review.md': '4fa39795f01fb3b366bd6ea8d622ff38d25d3ff1f55995aa0f9d63de71ae20e6',
  'docs/qualification/g0-v13-g1-v30-refreeze-review.md': '84db2030df75d13b2af69266a163cb890cdff8809b1156789819258508f428b3',
  'docs/qualification/g0-v14-g1-v31-refreeze-review.md': '56291b7b527eddb8c81bbb38a0d94ff91197f5a8c97692ab90e8ce441514a55e',
  'docs/qualification/g0-v15-g1-v32-refreeze-review.md': '22b8c19bd8b0332aab425c29bde0d61e9bb438eba1c805d510c4275cba677130',
  'docs/qualification/g0-v16-g1-v33-refreeze-review.md': '8fcbb8414c1d3310cd0af4be11b9f84950d3beb46ee5050a487bea74e320b9c1',
  ...Object.fromEntries(historicalPlanHashes.map((hash, index) => [
    `docs/qualification/semantic-manifold-g1-plan-v${index + 1}.json`, hash,
  ])),
})

export function refreshInputPaths(root = defaultRoot) {
  const fingerprint = JSON.parse(ordinaryBytes(root, oldFingerprint))
  const plan = JSON.parse(ordinaryBytes(root, oldPlan))
  const ownRust = JSON.parse(ordinaryBytes(root, OWN_RUST_EVIDENCE))
  return [...new Set([
    ...Object.keys(FROZEN_ARCHIVES), REFRESH_SCRIPT, REFRESH_REVIEW, REFREEZE_CORE,
    githubEnvironment, ...REFRESH_EXECUTORS,
    OWN_RUST_EVIDENCE, ...ownRust.sourceBundle.paths,
    ...fingerprint.artifacts.map(item => item.path),
    ...plan.bindings.artifacts.map(item => item.path),
    ...plan.bindings.bundles.flatMap(item => item.paths),
  ])].sort()
}

export function refreshBinaryInputPaths(root = defaultRoot) {
  const ownRust = JSON.parse(ordinaryBytes(root, OWN_RUST_EVIDENCE))
  return [ownRust.wasm.path]
}

export function assertRefreshStable(prepared) {
  assertNoCandidateResults(prepared.root)
  const current = snapshotFiles(prepared.root, [...prepared.snapshot.keys()])
  assert.deepEqual(current, prepared.snapshot, 'Bound sources or archived evidence changed during re-freeze')
  assert.deepEqual({
    path: prepared.ownRustWasm.path,
    ...shaRecord(readFileSync(resolve(prepared.root, prepared.ownRustWasm.path))),
  },
    prepared.ownRustWasm, 'Bound own-Rust WASM changed during re-freeze')
}

function assertNoCandidateResults(root) {
  assertCandidateResultsAbsent(root, 'semantic-manifold-g1-candidate-run-v39')
}

/** Pure preparation apart from reading ordinary files; it writes no artifacts. */
export function prepareQualificationRefresh({
  repositoryRoot = defaultRoot,
  recordedAt,
  reason,
  observedRustcVersionLine,
}) {
  assert(/^\d{4}-\d{2}-\d{2}$/u.test(recordedAt ?? ''), 'recordedAt must be YYYY-MM-DD')
  assert(new Date(`${recordedAt}T00:00:00Z`).toISOString().slice(0, 10) === recordedAt, 'Invalid calendar date')
  assert(typeof reason === 'string' && reason.trim().length >= 20, 'A concrete amendment reason is required')
  assert(typeof observedRustcVersionLine === 'string' && /^rustc [^\r\n]+$/u.test(observedRustcVersionLine), 'An observed rustc version line is required')
  const root = resolve(repositoryRoot)
  assertNoCandidateResults(root)
  const snapshot = snapshotFiles(root, refreshInputPaths(root))
  for (const [path, expected] of Object.entries(FROZEN_ARCHIVES)) {
    assert.equal(snapshot.get(path).sha256, expected, `Historical archive changed: ${path}`)
  }
  const previousFingerprint = JSON.parse(ordinaryBytes(root, oldFingerprint))
  const previousPlan = JSON.parse(ordinaryBytes(root, oldPlan))
  assert.equal(previousPlan.executionProtocol.plannedWorkUnits, 4740)
  assert.equal(previousPlan.oracle.caseCount, 65)
  assert.equal(previousPlan.lifecycle.qualificationClaim, 'none')
  const immutableArtifacts = new Set([
    'frozen-oracle-manifest', 'frozen-reference-oracle', 'frozen-reference-direct-evaluator',
    'qualification-plan-schema', 'g1-runtime-browser-bindings',
  ])
  for (const artifact of previousPlan.bindings.artifacts) {
    assert.equal(artifact.sha256.state, 'frozen', `Unresolved binding: ${artifact.id}`)
    if (immutableArtifacts.has(artifact.id)) {
      assert.equal(snapshot.get(artifact.path).sha256, artifact.sha256.value,
        `Frozen oracle/schema/environment cannot be amended by this refresh: ${artifact.id}`)
    }
  }
  const channel = ordinaryBytes(root, 'rust-toolchain.toml').toString('utf8')
    .match(/^channel\s*=\s*"([^"]+)"/mu)?.[1]
  assert(channel, 'Pinned Rust toolchain channel is missing')
  const fingerprint = structuredClone(previousFingerprint)
  fingerprint.fingerprintId = 'g0-toolchain-fingerprints-v22'
  fingerprint.recordedAt = recordedAt
  fingerprint.previousFingerprintId = previousFingerprint.fingerprintId
  fingerprint.previousFingerprintSha256 = FROZEN_ARCHIVES[oldFingerprint]
  fingerprint.purpose = reason.trim()
  fingerprint.claimBoundary.excludes = [
    'Full SPDX SBOM', 'Playwright qualification locks',
    'Any rewrite of G0 v1-v21 or G1 v1 through v38',
    'G0 closure, G1 qualification, completed clean work, or production cutover',
  ]
  fingerprint.claimBoundary.includes = [
    ...new Set([
      ...fingerprint.claimBoundary.includes,
      'Exact SHA-256 of the Geometry Closure V3 own-Rust source bundle and rebuilt WASM',
      'Exact SHA-256 of the V39 GitHub workflow, evidence harness, selectors, and hosted-runner identity freeze',
    ]),
  ]
  fingerprint.toolchain.rustChannel = channel
  fingerprint.toolchain.observedRustcVersionLine = observedRustcVersionLine
  fingerprint.artifacts = previousFingerprint.artifacts.map(item => ({ ...item, ...snapshot.get(item.path) }))
  const ownRust = JSON.parse(ordinaryBytes(root, OWN_RUST_EVIDENCE))
  assert.deepEqual(ownRust.sourceBundle.paths, [...ownRust.sourceBundle.paths].sort(),
    'Own-Rust source bundle paths must be sorted')
  const wasmBytes = readFileSync(resolve(root, ownRust.wasm.path))
  assert.equal(digest(wasmBytes), ownRust.wasm.sha256,
    'Own-Rust WASM fingerprint does not match rebuilt bytes')
  assert.equal(wasmBytes.byteLength, ownRust.wasm.byteLength,
    'Own-Rust WASM byte length does not match rebuilt bytes')
  const sourceBundle = Buffer.concat(ownRust.sourceBundle.paths.flatMap(path => [
    Buffer.from(path), Buffer.from([0]), ordinaryBytes(root, path), Buffer.from('\n'),
  ]))
  assert.equal(digest(sourceBundle), ownRust.sourceBundle.sha256,
    'Own-Rust source fingerprint does not match final source bytes')
  const existingArtifactIds = new Set(fingerprint.artifacts.map(item => item.id))
  const existingArtifactPaths = new Set(fingerprint.artifacts.map(item => item.path))
  let nextGeometryIndex = Math.max(0, ...fingerprint.artifacts
    .map(item => /^geometry-v3-source-(\d+)$/u.exec(item.id)?.[1])
    .filter(Boolean).map(Number))
  const geometryArtifacts = [
    { id: 'own-rust-cad-v14-evidence', path: OWN_RUST_EVIDENCE },
    ...ownRust.sourceBundle.paths
      .filter(path => !existingArtifactPaths.has(path))
      .map(path => ({
        id: `geometry-v3-source-${String(++nextGeometryIndex).padStart(2, '0')}`,
        path,
      })),
  ]
  fingerprint.artifacts.push(...geometryArtifacts
    .filter(item => !existingArtifactIds.has(item.id))
    .map(item => ({ ...item, ...snapshot.get(item.path) })))
  const qualificationArtifacts = [
    { id: 'g1-v34-github-workflow', path: '.github/workflows/g1-qualification-clean.yml' },
    { id: 'g1-v34-github-harness', path: 'scripts/g1-github-actions.mjs' },
    { id: 'g1-v34-clean-fragment-selector', path: 'scripts/run-g1-candidate-clean-fragment.mjs' },
    { id: 'g1-v34-ubuntu-selector', path: 'scripts/run-g1-ubuntu-docker-fragment.mjs' },
    { id: 'g1-v34-github-environment', path: githubEnvironment },
  ]
  fingerprint.artifacts.push(...qualificationArtifacts
    .filter(item => !existingArtifactPaths.has(item.path))
    .map(item => ({ ...item, ...snapshot.get(item.path) })))
  fingerprint.knownDrift = previousFingerprint.artifacts
    .filter(item => item.sha256 !== snapshot.get(item.path).sha256)
    .map(item => ({ relativeTo: oldFingerprint, bindingId: item.id,
      note: `Current bytes are newly bound in v22; v21 remains archived. ${reason.trim()}` }))

  const plan = structuredClone(previousPlan)
  plan.planId = 'semantic-manifold-g1-plan-v39'
  plan.processAmendment = {
    kind: 'post-freeze-harness-amendment', previousPlanId: previousPlan.planId,
    previousPlanSha256: FROZEN_ARCHIVES[oldPlan], reason: reason.trim(),
    changes: [
      'Recompute artifact and canonical bundle digests from current source bytes.',
      'Bind the exact GitHub Actions workflow, evidence-producing harness, selectors, and hosted-runner image identities.',
      'Start semantic-manifold-g1-candidate-run-v39 with zero completed clean runs and zero completed work units; no prior result is imported.',
      'Retain G0 v1-v21 and G1 v1 through v38 byte-for-byte; G0 toolchain fingerprints advance separately to v22.',
    ],
    preserved: [
      'All 65 oracle cases, 18 comparator mutations, boundary cases, matrix rows, environment IDs, seeds, budgets, required clean runs and all 4740 work units.',
      'Scope, oracle and comparator bytes, claim boundary, reset rules, unresolved u07 and production cutover prohibition.',
      'Prior evidence remains discovery-only; qualificationClaim stays none and qualificationApproval stays not-approved.',
    ],
    priorEvidenceTreatment: 'discovery-only', qualificationClaim: 'none',
  }
  const candidateBundle = plan.bindings.bundles.find(bundle => bundle.id === 'g1-candidate-bundle')
  assert(candidateBundle, 'G1 candidate bundle is missing')
  if (!candidateBundle.paths.includes('src/core/ownRustCadEvidence.ts')) {
    candidateBundle.paths.push('src/core/ownRustCadEvidence.ts')
    candidateBundle.paths.sort()
  }
  const harnessBundle = plan.bindings.bundles.find(bundle => bundle.id === 'g1-qualification-harness-bundle')
  assert(harnessBundle, 'G1 qualification harness bundle is missing')
  for (const path of REFRESH_EXECUTORS) {
    if (!harnessBundle.paths.includes(path)) harnessBundle.paths.push(path)
  }
  harnessBundle.paths.sort()
  if (!plan.bindings.artifacts.some(artifact => artifact.id === 'g1-github-actions-environment-v34')) {
    plan.bindings.artifacts.push({
      id: 'g1-github-actions-environment-v34',
      path: githubEnvironment,
      purpose: 'Exact hosted runner labels/image versions, action commit pins, source-SHA protocol, and downloaded archive identity-only boundary for V34.',
      sha256: {
        state: 'frozen', algorithm: 'sha256',
        value: snapshot.get(githubEnvironment).sha256,
        byteLength: snapshot.get(githubEnvironment).byteLength,
        verifiedBy: 'qualification validator and GitHub preflight recompute exact bytes and enforce every identity',
      },
    })
  }
  const githubFreeze = JSON.parse(ordinaryBytes(root, githubEnvironment))
  for (const environment of plan.environments.filter(item => item.browser)) {
    const revision = githubFreeze.playwright.executedTree[environment.browser.engine]
    const tree = githubFreeze.playwright.browserTrees[revision]
    environment.browser.version = 'bound-by-frozen-binary-sha256'
    environment.browser.sha256 = {
      ...environment.browser.sha256,
      value: tree.treeSha256,
      byteLength: tree.manifestByteLength,
      verifiedBy: `V34 canonical Linux tree manifest for ${revision}; every required supporting tree is checked by the frozen GitHub harness`,
    }
  }
  for (const artifact of plan.bindings.artifacts) {
    const hash = snapshot.get(artifact.path)
    artifact.sha256 = { ...artifact.sha256, value: hash.sha256, byteLength: hash.byteLength }
  }
  for (const bundle of plan.bindings.bundles) {
    assert.deepEqual(bundle.paths, [...bundle.paths].sort(), `Unsorted bundle: ${bundle.id}`)
    const bytes = bundleBytes(bundle.paths, snapshot)
    bundle.sha256 = { ...bundle.sha256, value: digest(bytes), byteLength: bytes.byteLength }
  }
  plan.executionProtocol.candidateRunId = 'semantic-manifold-g1-candidate-run-v39'
  plan.executionProtocol.resultPath = 'output/qualification/semantic-manifold-g1-candidate-run-v39/result.json'
  plan.executionProtocol.cleanRunDefinition[0] = previousPlan.executionProtocol.cleanRunDefinition[0]
    .replace('exact v38 frozen artifact', 'exact v39 frozen artifact')
  assert.equal(plan.executionProtocol.priorResultsMayBeImported, false)
  assert.equal(plan.approvals.qualificationApproval, 'not-approved')
  assert(plan.matrix.every(row => row.evidenceState === 'not-executed-clean-post-freeze'))

  const artifactBytes = { fingerprint: jsonBytes(fingerprint), plan: jsonBytes(plan) }
  const changes = []
  for (const [kind, before, after] of [
    ['g0-artifact', previousFingerprint.artifacts, fingerprint.artifacts],
    ['g1-artifact', previousPlan.bindings.artifacts, plan.bindings.artifacts],
    ['g1-bundle', previousPlan.bindings.bundles, plan.bindings.bundles],
  ]) {
    before.forEach((item, index) => {
      const oldHash = typeof item.sha256 === 'string' ? item.sha256 : item.sha256.value
      const nextHash = typeof after[index].sha256 === 'string' ? after[index].sha256 : after[index].sha256.value
      if (oldHash !== nextHash) changes.push({ kind, id: item.id, previousSha256: oldHash, currentSha256: nextHash })
    })
  }
  for (const artifact of fingerprint.artifacts.slice(previousFingerprint.artifacts.length)) {
    changes.push({
      kind: 'g0-artifact',
      id: artifact.id,
      previousSha256: null,
      currentSha256: artifact.sha256,
    })
  }
  for (const artifact of plan.bindings.artifacts.slice(previousPlan.bindings.artifacts.length)) {
    changes.push({
      kind: 'g1-artifact',
      id: artifact.id,
      previousSha256: null,
      currentSha256: artifact.sha256.value,
    })
  }
  const status = {
    schema: 'open-scad-viewer/qualification-refreeze-status', schemaVersion: 1,
    statusId: 'g0-v22-g1-v39-refreeze-status-v1', recordedAt, reason: reason.trim(),
    qualificationClaim: 'none', qualificationApproval: 'not-approved', g0Closed: false,
    priorEvidenceTreatment: 'discovery-only', priorResultsMayBeImported: false,
    candidateRunId: plan.executionProtocol.candidateRunId,
    completedWorkUnits: 0, completedCleanRuns: 0, plannedWorkUnits: 4740,
    pendingRows: plan.matrix.map(row => ({
      id: row.id, environmentIds: row.executionEnvironmentIds,
      cleanRunsRequiredPerEnvironment: row.work.cleanRunsRequired,
      completedCleanRuns: 0, completedWorkUnits: 0, plannedWorkUnits: row.work.plannedUnits,
    })),
    archives: Object.entries(FROZEN_ARCHIVES).map(([path, sha256]) => ({ path, sha256 })),
    artifacts: Object.entries(artifactBytes).map(([id, bytes]) => ({ id, path: REFRESH_OUTPUTS[id], ...shaRecord(bytes) })),
    inputSnapshot: { canonicalization: 'UTF-8 records sorted by path: path + NUL + lowercase-file-sha256 + LF',
      ...shaRecord(bundleBytes([...snapshot.keys()], snapshot)),
      files: [...snapshot].map(([path, hash]) => ({ path, ...hash })),
    },
    changedBindings: changes,
    executionNotes: [
      'This artifact records a new freeze, not an execution result. All 4740 units remain mandatory.',
      'The workflow, GitHub harness, and two fragment helpers select v39 and are frozen inputs; no v38 fragment is imported.',
      'No result artifact, test invocation, clean-run approval or production authorization is created by this script.',
    ],
  }
  artifactBytes.status = jsonBytes(status)
  const prepared = {
    root, snapshot, fingerprint, plan, status, artifactBytes,
    ownRustWasm: { path: ownRust.wasm.path, ...shaRecord(wasmBytes) },
  }
  assertRefreshStable(prepared)
  return prepared
}

/** Publish new files exclusively; an existing version is never overwritten. */
export function publishQualificationRefresh(prepared) {
  publishExclusive({ ...prepared, outputs: REFRESH_OUTPUTS, assertStable: () => assertRefreshStable(prepared) })
}

function main(argv) {
  let mode = 'check'
  const values = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--check' || arg === '--write') { mode = arg.slice(2); continue }
    assert(['--recorded-at', '--reason'].includes(arg), `Unknown argument: ${arg}`)
    const value = argv[++index]
    assert(value && !value.startsWith('--') && !values.has(arg), `Missing/duplicate argument: ${arg}`)
    values.set(arg, value)
  }
  if (mode === 'write') {
    assert(values.has('--recorded-at') && values.has('--reason'), '--write requires --recorded-at and --reason')
  }
  const prepared = prepareQualificationRefresh({
    recordedAt: values.get('--recorded-at') ?? new Date().toISOString().slice(0, 10),
    reason: values.get('--reason') ?? 'Re-freeze current SVG/BRep dependency, notice and source changes; preserve the finite G1 contract and reset all clean-run work.',
    observedRustcVersionLine: execFileSync('rustc', ['--version'], { cwd: defaultRoot, encoding: 'utf8' }).trim(),
  })
  if (mode === 'write') publishQualificationRefresh(prepared)
  console.log(JSON.stringify({ mode, written: mode === 'write', qualificationClaim: 'none',
    completedWorkUnits: 0, plannedWorkUnits: 4740, changedBindings: prepared.status.changedBindings,
    artifacts: prepared.status.artifacts, statusPath: REFRESH_OUTPUTS.status }, null, 2))
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)) } catch (error) { console.error(error.message); process.exitCode = 1 }
}
