#!/usr/bin/env node
// Every supported target is an explicit reviewed amendment, never an implicit latest version.
import assert from 'node:assert/strict'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  REFREEZE_CORE, digest, jsonBytes, shaRecord, bundleBytes, ordinaryBytes,
  snapshotFiles, publishExclusive, assertNoCandidateResults,
} from './qualificationRefreezeCore.mjs'

const defaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
export const G1_REFRESH_SCRIPT = 'scripts/refresh-g1-plan.mjs'
export const G1_EXECUTORS = Object.freeze([
  'scripts/run-g1-candidate-clean-fragment.mjs',
  'scripts/run-g1-ubuntu-docker-fragment.mjs',
])
export const G1_AMENDMENTS = Object.freeze({
  20: Object.freeze({
    version: 20, previousVersion: 19,
    previousPlanSha256: '14fb0fdc79532dcd9a8bed3a93128f152fe3b68cc56529b6cc83561b5b899ed3',
    previousStatus: 'docs/qualification/g0-v3-g1-v19-refreeze-status-v1.json',
    previousStatusSha256: '1c22fe7e7a5451a067be0b6b99b62a4f2c3be2e8067518b10d06d04c51c68b25',
    g0Fingerprint: 'docs/qualification/g0-toolchain-fingerprints-v3.json',
    g0FingerprintSha256: 'c8c74e76f7e6ce9e85c8f1c4267e678ca8c243b16ec4e6dd050584b489c0c171',
    review: 'docs/qualification/g1-v20-refreeze-review.md',
    reason: 'Re-freeze the shared semanticProgramExecutor.ts change after v19; retain the existing G0 v3 fingerprint and restart all required G1 clean work.',
  }),
})

function amendment(version) {
  const definition = G1_AMENDMENTS[version]
  assert(definition, `Version ${version} requires an explicit reviewed amendment definition`)
  assert.equal(definition.version, definition.previousVersion + 1)
  return definition
}

export function g1RefreshOutputs(version) {
  amendment(version)
  return {
    plan: `docs/qualification/semantic-manifold-g1-plan-v${version}.json`,
    status: `docs/qualification/g1-v${version}-refreeze-status-v1.json`,
  }
}

function historicalSources(root, definition) {
  const previousPlan = `docs/qualification/semantic-manifold-g1-plan-v${definition.previousVersion}.json`
  const anchors = {
    [previousPlan]: definition.previousPlanSha256,
    [definition.previousStatus]: definition.previousStatusSha256,
    [definition.g0Fingerprint]: definition.g0FingerprintSha256,
  }
  for (const [path, hash] of Object.entries(anchors)) {
    assert.equal(digest(ordinaryBytes(root, path)), hash, `Historical archive changed: ${path}`)
  }
  const status = JSON.parse(ordinaryBytes(root, definition.previousStatus))
  const plan = JSON.parse(ordinaryBytes(root, previousPlan))
  const g0 = JSON.parse(ordinaryBytes(root, definition.g0Fingerprint))
  const archives = { ...Object.fromEntries(status.archives.map(item => [item.path, item.sha256])), ...anchors }
  return { status, plan, g0, archives }
}

export function g1RefreshInputPaths(version, repositoryRoot = defaultRoot) {
  const definition = amendment(version)
  const previous = historicalSources(repositoryRoot, definition)
  return [...new Set([
    ...Object.keys(previous.archives),
    ...previous.status.inputSnapshot.files.map(item => item.path),
    ...previous.g0.artifacts.map(item => item.path),
    ...previous.plan.bindings.artifacts.map(item => item.path),
    ...previous.plan.bindings.bundles.flatMap(item => item.paths),
    G1_REFRESH_SCRIPT, REFREEZE_CORE, definition.review, ...G1_EXECUTORS,
  ])].sort()
}

export function assertG1RefreshStable(prepared) {
  assertNoCandidateResults(prepared.root, prepared.plan.executionProtocol.candidateRunId)
  assert.deepEqual(snapshotFiles(prepared.root, [...prepared.snapshot.keys()]), prepared.snapshot,
    'Bound sources or archived evidence changed during re-freeze')
}

export function prepareG1PlanRefresh({ version, repositoryRoot = defaultRoot, recordedAt, reason }) {
  const definition = amendment(version)
  assert(/^\d{4}-\d{2}-\d{2}$/u.test(recordedAt ?? ''), 'recordedAt must be YYYY-MM-DD')
  assert.equal(new Date(`${recordedAt}T00:00:00Z`).toISOString().slice(0, 10), recordedAt)
  assert(typeof reason === 'string' && reason.trim().length >= 20, 'A concrete amendment reason is required')
  const root = resolve(repositoryRoot)
  const candidateRunId = `semantic-manifold-g1-candidate-run-v${version}`
  assertNoCandidateResults(root, candidateRunId)
  const previous = historicalSources(root, definition)
  const snapshot = snapshotFiles(root, g1RefreshInputPaths(version, root))
  for (const [path, hash] of Object.entries(previous.archives)) {
    assert.equal(snapshot.get(path).sha256, hash, `Historical archive changed: ${path}`)
  }
  for (const artifact of previous.g0.artifacts) {
    assert.deepEqual(snapshot.get(artifact.path), { sha256: artifact.sha256, byteLength: artifact.byteLength },
      `G0 binding drift requires a separate G0 amendment: ${artifact.path}`)
  }
  assert.equal(previous.plan.oracle.caseCount, 65)
  assert.equal(previous.plan.executionProtocol.plannedWorkUnits, 4740)
  assert.equal(previous.plan.lifecycle.qualificationClaim, 'none')
  assert.equal(previous.plan.approvals.qualificationApproval, 'not-approved')
  assert.equal(previous.plan.executionProtocol.priorResultsMayBeImported, false)
  const protectedArtifacts = new Set([
    'frozen-oracle-manifest', 'frozen-reference-oracle', 'frozen-reference-direct-evaluator',
    'qualification-plan-schema', 'g1-runtime-browser-bindings',
  ])
  for (const item of previous.plan.bindings.artifacts) {
    assert.equal(item.sha256.state, 'frozen')
    if (protectedArtifacts.has(item.id)) assert.equal(snapshot.get(item.path).sha256, item.sha256.value,
      `Frozen oracle/schema/environment cannot be amended by this refresh: ${item.id}`)
  }
  const plan = structuredClone(previous.plan)
  plan.planId = `semantic-manifold-g1-plan-v${version}`
  plan.processAmendment = {
    kind: 'post-freeze-harness-amendment', previousPlanId: previous.plan.planId,
    previousPlanSha256: definition.previousPlanSha256, reason: reason.trim(),
    changes: [
      'Recompute only the existing artifact and canonical bundle digests; preserve all binding membership.',
      `Start ${candidateRunId} with zero completed clean runs and work units; prior evidence is not imported.`,
      `Retain the previous plan, its source-snapshot status and G0 fingerprint as byte-immutable historical evidence; active source checks use v${version} only.`,
    ],
    preserved: [
      'The same 65 oracle cases, 18 comparator mutations, finite boundary matrix, environments, seeds, budgets, required clean runs and all 4740 work units.',
      'Scope, frozen oracle/reference/schema/environment bytes, claim boundary, reset rules, unresolved u07 and production cutover prohibition.',
      'Existing G0 v3 remains current; G0 stays open, qualificationClaim stays none and qualificationApproval stays not-approved.',
    ], priorEvidenceTreatment: 'discovery-only', qualificationClaim: 'none',
  }
  for (const artifact of plan.bindings.artifacts) {
    const hash = snapshot.get(artifact.path)
    artifact.sha256 = { ...artifact.sha256, value: hash.sha256, byteLength: hash.byteLength }
  }
  for (const bundle of plan.bindings.bundles) {
    assert.deepEqual(bundle.paths, [...bundle.paths].sort())
    assert.equal(bundle.sha256.state, 'frozen')
    const bytes = bundleBytes(bundle.paths, snapshot)
    bundle.sha256 = { ...bundle.sha256, value: digest(bytes), byteLength: bytes.byteLength }
  }
  plan.executionProtocol.candidateRunId = candidateRunId
  plan.executionProtocol.resultPath = `output/qualification/${candidateRunId}/result.json`
  plan.executionProtocol.cleanRunDefinition[0] = previous.plan.executionProtocol.cleanRunDefinition[0]
    .replace(`exact v${definition.previousVersion} frozen artifact`, `exact v${version} frozen artifact`)
  assert(plan.matrix.every(row => row.evidenceState === 'not-executed-clean-post-freeze'))
  const outputs = g1RefreshOutputs(version)
  const artifactBytes = { plan: jsonBytes(plan) }
  const previousInputs = new Map(previous.status.inputSnapshot.files.map(item => [item.path, item]))
  const status = {
    schema: 'open-scad-viewer/qualification-refreeze-status', schemaVersion: 1,
    statusId: `g1-v${version}-refreeze-status-v1`, recordedAt, reason: reason.trim(),
    qualificationClaim: 'none', qualificationApproval: 'not-approved', g0Closed: false,
    priorEvidenceTreatment: 'discovery-only', priorResultsMayBeImported: false,
    candidateRunId, completedWorkUnits: 0, completedCleanRuns: 0, plannedWorkUnits: 4740,
    pendingRows: plan.matrix.map(row => ({ id: row.id, environmentIds: row.executionEnvironmentIds,
      cleanRunsRequiredPerEnvironment: row.work.cleanRunsRequired,
      completedCleanRuns: 0, completedWorkUnits: 0, plannedWorkUnits: row.work.plannedUnits })),
    archives: Object.entries(previous.archives).map(([path, sha256]) => ({ path, sha256 })),
    artifacts: [{ id: 'plan', path: outputs.plan, ...shaRecord(artifactBytes.plan) }],
    retainedG0Fingerprint: { path: definition.g0Fingerprint, sha256: definition.g0FingerprintSha256 },
    previousSourceSnapshot: { path: definition.previousStatus, sha256: definition.previousStatusSha256,
      treatment: 'historical-only; its recorded source bytes are not assertions about current files' },
    inputSnapshot: { canonicalization: 'UTF-8 records sorted by path: path + NUL + lowercase-file-sha256 + LF',
      ...shaRecord(bundleBytes([...snapshot.keys()], snapshot)),
      files: [...snapshot].map(([path, hash]) => ({ path, ...hash })) },
    changedInputs: [...snapshot].filter(([path, hash]) => previousInputs.get(path)?.sha256 !== hash.sha256)
      .map(([path, hash]) => ({ path, previousSha256: previousInputs.get(path)?.sha256 ?? null,
        currentSha256: hash.sha256, previousByteLength: previousInputs.get(path)?.byteLength ?? null,
        currentByteLength: hash.byteLength })),
    executionNotes: [
      'This is a new source freeze, not an execution result. All 4740 work units remain mandatory.',
      `The active fragment helpers select v${version}; no earlier fragments or counters are imported.`,
      'No qualification invocation, execution approval, qualification approval or production authorization is created.',
    ],
  }
  artifactBytes.status = jsonBytes(status)
  const prepared = { root, snapshot, outputs, artifactBytes, plan, status }
  assertG1RefreshStable(prepared)
  return prepared
}

export function publishG1PlanRefresh(prepared) {
  publishExclusive({ ...prepared, assertStable: () => assertG1RefreshStable(prepared) })
}

function main(argv) {
  let mode = 'check'
  const values = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--check' || arg === '--write') { mode = arg.slice(2); continue }
    assert(['--version', '--recorded-at', '--reason'].includes(arg), `Unknown argument: ${arg}`)
    const value = argv[++index]
    assert(value && !value.startsWith('--') && !values.has(arg), `Missing/duplicate argument: ${arg}`)
    values.set(arg, value)
  }
  const version = Number(values.get('--version'))
  const definition = amendment(version)
  if (mode === 'write') assert(values.has('--recorded-at') && values.has('--reason'), '--write requires --recorded-at and --reason')
  const prepared = prepareG1PlanRefresh({ version,
    recordedAt: values.get('--recorded-at') ?? new Date().toISOString().slice(0, 10),
    reason: values.get('--reason') ?? definition.reason,
  })
  if (mode === 'write') publishG1PlanRefresh(prepared)
  console.log(JSON.stringify({ mode, written: mode === 'write', qualificationClaim: 'none',
    candidateRunId: prepared.status.candidateRunId, completedWorkUnits: 0, plannedWorkUnits: 4740,
    changedInputs: prepared.status.changedInputs, artifacts: prepared.status.artifacts,
    retainedG0Fingerprint: prepared.status.retainedG0Fingerprint, statusPath: prepared.outputs.status }, null, 2))
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)) } catch (error) { console.error(error.message); process.exitCode = 1 }
}
