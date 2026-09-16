#!/usr/bin/env node
// Explicit append-only re-freeze. This script never executes a qualification row.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { REFREEZE_CORE, digest, jsonBytes, shaRecord, bundleBytes, ordinaryBytes, snapshotFiles, publishExclusive, assertNoCandidateResults as assertCandidateResultsAbsent } from './qualificationRefreezeCore.mjs'

const defaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
export const REFRESH_SCRIPT = 'scripts/refresh-qualification-fingerprints.mjs'
export const REFRESH_REVIEW = 'docs/qualification/g0-v7-g1-v24-refreeze-review.md'
export const REFRESH_EXECUTORS = Object.freeze([
  'scripts/run-g1-candidate-clean-fragment.mjs',
  'scripts/run-g1-ubuntu-docker-fragment.mjs',
])
export const REFRESH_OUTPUTS = Object.freeze({
  fingerprint: 'docs/qualification/g0-toolchain-fingerprints-v7.json',
  plan: 'docs/qualification/semantic-manifold-g1-plan-v24.json',
  status: 'docs/qualification/g0-v7-g1-v24-refreeze-status-v1.json',
})
const oldFingerprint = 'docs/qualification/g0-toolchain-fingerprints-v6.json'
const oldPlan = 'docs/qualification/semantic-manifold-g1-plan-v23.json'
const oldStatus = 'docs/qualification/g0-v6-g1-v23-refreeze-status-v1.json'
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
]
export const FROZEN_ARCHIVES = Object.freeze({
  'docs/qualification/g0-toolchain-fingerprints-v1.json': '4c2a286ea9567ddef66d2a0335a4960bddc94921b662c9e34af063b480cb8af3',
  'docs/qualification/g0-toolchain-fingerprints-v2.json': 'd040f252ce41177e24b02491beef59ccb57771e99fc0efaf9f0e7e4b2a081e76',
  'docs/qualification/g0-toolchain-fingerprints-v3.json': 'c8c74e76f7e6ce9e85c8f1c4267e678ca8c243b16ec4e6dd050584b489c0c171',
  'docs/qualification/g0-toolchain-fingerprints-v4.json': '38b9fac7b3e03a301b0dfdeadf514572f337feff37b186089d968f3afa9e5a46',
  'docs/qualification/g0-toolchain-fingerprints-v5.json': '9da6b4921492e2f6592194b58f144d6c7460f3c6787717f1b311cf7040428040',
  [oldFingerprint]: 'ef2f25017daec1d75fae2725a85a500fa59d36c448f8a5ba0b9407f90a291eaf',
  'docs/qualification/g0-v3-g1-v19-refreeze-status-v1.json': '1c22fe7e7a5451a067be0b6b99b62a4f2c3be2e8067518b10d06d04c51c68b25',
  'docs/qualification/g1-v20-refreeze-status-v1.json': 'ec3fc7f7a87ceda8f6269cb2271648d9418167227c31887bf2b9614961d705d9',
  'docs/qualification/g0-v4-g1-v21-refreeze-status-v1.json': '1c1ab4668ea6a47f57d0f3316688e182b1b87ae5c78bccea902ca57f528b586d',
  'docs/qualification/g0-v5-g1-v22-refreeze-status-v1.json': 'e3229477708ad67db3f632225ac89a6be5de91f7e2a07aaa8c3c06740a2cf84e',
  [oldStatus]: 'f3e9033d518d14f3fd317974cfb8bb1b4706ec5dda9661b9e011ff5ee2e593d7',
  'docs/qualification/g0-v6-g1-v23-refreeze-review.md': '182970eb33bcb479d6b0c4258f665d5023a13ee059a39644fce2a202b0f75666',
  ...Object.fromEntries(historicalPlanHashes.map((hash, index) => [
    `docs/qualification/semantic-manifold-g1-plan-v${index + 1}.json`, hash,
  ])),
})

export function refreshInputPaths(root = defaultRoot) {
  const fingerprint = JSON.parse(ordinaryBytes(root, oldFingerprint))
  const plan = JSON.parse(ordinaryBytes(root, oldPlan))
  return [...new Set([
    ...Object.keys(FROZEN_ARCHIVES), REFRESH_SCRIPT, REFRESH_REVIEW, REFREEZE_CORE, ...REFRESH_EXECUTORS,
    ...fingerprint.artifacts.map(item => item.path),
    ...plan.bindings.artifacts.map(item => item.path),
    ...plan.bindings.bundles.flatMap(item => item.paths),
  ])].sort()
}

export function assertRefreshStable(prepared) {
  assertNoCandidateResults(prepared.root)
  const current = snapshotFiles(prepared.root, [...prepared.snapshot.keys()])
  assert.deepEqual(current, prepared.snapshot, 'Bound sources or archived evidence changed during re-freeze')
}

function assertNoCandidateResults(root) {
  assertCandidateResultsAbsent(root, 'semantic-manifold-g1-candidate-run-v24')
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
  fingerprint.fingerprintId = 'g0-toolchain-fingerprints-v7'
  fingerprint.recordedAt = recordedAt
  fingerprint.previousFingerprintId = previousFingerprint.fingerprintId
  fingerprint.previousFingerprintSha256 = FROZEN_ARCHIVES[oldFingerprint]
  fingerprint.purpose = reason.trim()
  fingerprint.claimBoundary.excludes = [
    'Full SPDX SBOM', 'Playwright qualification locks',
    'Any rewrite of G0 v1-v6 or G1 v1 through v23',
    'G0 closure, G1 qualification, completed clean work, or production cutover',
  ]
  fingerprint.toolchain.rustChannel = channel
  fingerprint.toolchain.observedRustcVersionLine = observedRustcVersionLine
  fingerprint.artifacts = previousFingerprint.artifacts.map(item => ({ ...item, ...snapshot.get(item.path) }))
  fingerprint.knownDrift = previousFingerprint.artifacts
    .filter(item => item.sha256 !== snapshot.get(item.path).sha256)
    .map(item => ({ relativeTo: oldFingerprint, bindingId: item.id,
      note: `Current bytes are newly bound in v7; v6 remains archived. ${reason.trim()}` }))

  const plan = structuredClone(previousPlan)
  plan.planId = 'semantic-manifold-g1-plan-v24'
  plan.processAmendment = {
    kind: 'post-freeze-harness-amendment', previousPlanId: previousPlan.planId,
    previousPlanSha256: FROZEN_ARCHIVES[oldPlan], reason: reason.trim(),
    changes: [
      'Recompute the existing artifact and canonical bundle digests from current source bytes; membership is unchanged.',
      'Start semantic-manifold-g1-candidate-run-v24 with zero completed clean runs and zero completed work units; no prior result is imported.',
      'Retain G0 v1-v6 and G1 v1 through v23 byte-for-byte; G0 toolchain fingerprints advance separately to v7.',
    ],
    preserved: [
      'All 65 oracle cases, 18 comparator mutations, boundary cases, matrix rows, environments, seeds, budgets, required clean runs and all 4740 work units.',
      'Scope, oracle and comparator bytes, claim boundary, reset rules, unresolved u07 and production cutover prohibition.',
      'Prior evidence remains discovery-only; qualificationClaim stays none and qualificationApproval stays not-approved.',
    ],
    priorEvidenceTreatment: 'discovery-only', qualificationClaim: 'none',
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
  plan.executionProtocol.candidateRunId = 'semantic-manifold-g1-candidate-run-v24'
  plan.executionProtocol.resultPath = 'output/qualification/semantic-manifold-g1-candidate-run-v24/result.json'
  plan.executionProtocol.cleanRunDefinition[0] = previousPlan.executionProtocol.cleanRunDefinition[0]
    .replace('exact v23 frozen artifact', 'exact v24 frozen artifact')
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
  const status = {
    schema: 'open-scad-viewer/qualification-refreeze-status', schemaVersion: 1,
    statusId: 'g0-v7-g1-v24-refreeze-status-v1', recordedAt, reason: reason.trim(),
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
      'The two fragment helpers select v24 and its new result path. Their execution and classification requirements are unchanged; no v23 fragment is imported.',
      'No result artifact, test invocation, clean-run approval or production authorization is created by this script.',
    ],
  }
  artifactBytes.status = jsonBytes(status)
  const prepared = { root, snapshot, fingerprint, plan, status, artifactBytes }
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
