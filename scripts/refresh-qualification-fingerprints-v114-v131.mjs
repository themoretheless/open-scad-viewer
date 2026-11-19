#!/usr/bin/env node
// Explicit append-only re-freeze. This script never executes a qualification row.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { readFileSync, existsSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { REFREEZE_CORE, digest, jsonBytes, shaRecord, bundleBytes, ordinaryBytes, snapshotFiles, publishExclusive, assertNoCandidateResults as assertCandidateResultsAbsent } from './qualificationRefreezeCore.mjs'

const defaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
export const REFRESH_SCRIPT = 'scripts/refresh-qualification-fingerprints.mjs'
export const REFRESH_REVIEW = 'docs/qualification/g0-v114-g1-v131-refreeze-review.md'
export const OWN_RUST_EVIDENCE = 'docs/qualification/own-rust-cad-v86.json'
const RUNTIME_EVIDENCE_PATHS = [
  'docs/qualification/runtime-provider-7702e12173f8-2026-11-18.json',
  'docs/qualification/runtime-direct-api-7702e1-2026-11-18/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-7702e1-2026-11-18/runtime-binding.json',
  'docs/qualification/runtime-direct-api-7702e1-2026-11-18/node.json',
  'docs/qualification/runtime-direct-api-7702e1-2026-11-18/report.json',
  'docs/qualification/runtime-provider-a0f18dd409f7-2026-11-18.json',
  'docs/qualification/runtime-direct-api-a0f18d-2026-11-18/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-a0f18d-2026-11-18/runtime-binding.json',
  'docs/qualification/runtime-direct-api-a0f18d-2026-11-18/node.json',
  'docs/qualification/runtime-direct-api-a0f18d-2026-11-18/report.json',
  'docs/qualification/runtime-provider-71294e3370d7-2026-11-18.json',
  'docs/qualification/runtime-direct-api-71294e-2026-11-18/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-71294e-2026-11-18/runtime-binding.json',
  'docs/qualification/runtime-direct-api-71294e-2026-11-18/node.json',
  'docs/qualification/runtime-direct-api-71294e-2026-11-18/report.json',
  'src/core/currentRuntimeGeometryEvidence.ts',
  'src/core/runtimeGeometryEvidenceV13.ts',
  'src/core/runtimeGeometryEvidenceV14.ts',
  'docs/qualification/runtime-provider-8a82d7afc51a-2026-11-18.json',
  'docs/qualification/runtime-direct-api-8a82d7-2026-11-18/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-8a82d7-2026-11-18/runtime-binding.json',
  'docs/qualification/runtime-direct-api-8a82d7-2026-11-18/node.json',
  'docs/qualification/runtime-direct-api-8a82d7-2026-11-18/report.json',
  'docs/qualification/runtime-provider-516fa1bfeb90-2026-11-18.json',
  'docs/qualification/runtime-direct-api-516fa1-2026-11-18/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-516fa1-2026-11-18/runtime-binding.json',
  'docs/qualification/runtime-direct-api-516fa1-2026-11-18/node.json',
  'docs/qualification/runtime-direct-api-516fa1-2026-11-18/report.json',
  'docs/qualification/runtime-provider-7467ec8b0d7e-2026-11-17.json',
  'docs/qualification/runtime-direct-api-7467ec-2026-11-17/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-7467ec-2026-11-17/runtime-binding.json',
  'docs/qualification/runtime-direct-api-7467ec-2026-11-17/node.json',
  'docs/qualification/runtime-direct-api-7467ec-2026-11-17/report.json',
  'docs/qualification/runtime-provider-5c5f9b414d6c-2026-11-18.json',
  'docs/qualification/runtime-direct-api-5c5f9b-2026-11-18/runtime-api-baseline.json',
  'docs/qualification/runtime-direct-api-5c5f9b-2026-11-18/runtime-binding.json',
  'docs/qualification/runtime-direct-api-5c5f9b-2026-11-18/node.json',
  'docs/qualification/runtime-direct-api-5c5f9b-2026-11-18/report.json',
]
const INTEGRATION_INPUT_PATHS = [
  'crates/brep-core/src/contact/shared_boundary/jordan_partition.rs',
  'crates/nurbs-core/src/analysis/surface_injectivity/rational_jordan.rs',
  'crates/nurbs-core/src/editing/exact_knot_insertion/strict_box.rs',
  'tests/directModelerUi.test.ts',
  'crates/brep-core/src/contact/sweep_smoothness/complete/graph_jets.rs',
  'crates/brep-core/src/contact/sweep_smoothness/complete/graph_jets/refinement.rs',
  'crates/brep-core/src/contact/sweep_smoothness/complete.rs',
  'crates/brep-core/src/contact/sweep_smoothness/complete/periodic_uses.rs',
  'crates/brep-core/src/contact/sweep_smoothness/complete/interiors.rs',
  'crates/brep-core/src/contact/sweep_smoothness/complete/interiors/geometric.rs',
  'crates/brep-core/src/parts/analytic/loft.rs',
  'crates/geometry-bridge/src/dispatch/nurbs/lofts_and_sweeps.rs',
  'scripts/check-loft-generalization-browser.mjs',
  'scripts/export-loft-generalization-step.mts',
  'scripts/verify-loft-generalization-step.py',
  'crates/brep-core/src/parts/analytic/mod.rs',
  'crates/geometry-bridge/src/cad_sweep_boundary/rational_closed.rs',
  'crates/geometry-bridge/src/cad_sweep_boundary/smooth_closed.rs',
  'crates/geometry-bridge/src/cad_sweep_smoothness/complete.rs',
  'tests/brepRuledLoft.test.ts',
  'crates/brep-core/src/boolean/face_cut/open_ss.rs',
  'crates/brep-core/src/boolean/face_cut/open_ss/automatic.rs',
  'crates/brep-core/src/contact/shared_boundary/jordan_partition.rs',
  'crates/brep-core/tests/open_ss_partition.rs',
  'crates/brep-core/tests/original_frenet_material.rs',
  'crates/nurbs-core/src/analysis/surface_injectivity/rational_jordan.rs',
  'crates/nurbs-core/src/intersection/ss_intersection/fiber_coverage/physical_branches/trace.rs',
  'crates/nurbs-core/src/intersection/ss_intersection/fiber_coverage/physical_branches/trace/replay.rs',
  'crates/nurbs-core/src/intersection/ss_intersection/fiber_coverage/physical_branches/trace/tests.rs',
  'crates/nurbs-core/src/sweeps/progressive_miter/vector_certificate/span_fourth.rs',
  'crates/nurbs-core/src/sweeps/progressive_miter/vector_certificate/span_jets.rs',
  'crates/nurbs-core/src/sweeps/progressive_sweep/ideal_volume/arc_frames/frenet_rates.rs',
  'crates/nurbs-core/src/sweeps/progressive_sweep/ideal_volume/arc_frames/frenet_spans.rs',
  'crates/nurbs-core/src/sweeps/progressive_sweep/ideal_volume/fixed_rank.rs',
  'crates/nurbs-core/src/sweeps/progressive_sweep/ideal_volume/frenet_clock.rs',
  'src/features/DirectModeler.svt',
  'tests/directModelerUi.test.ts',
]
const MODULE_DELIVERY_PATHS = [
  'src/kernel/geometry/kernel.ts',
  'src/kernel/geometry/kernelCompilation.ts',
  'src/kernel/geometry/kernelCompilationRequired.ts',
  'src/kernel/wasmArtifact.ts',
  'vite.qualification.config.ts',
]
export const REFRESH_EXECUTORS = Object.freeze([
  '.gitattributes',
  'scripts/qualificationFrozenArchives.mjs',
  'scripts/qualificationLegacyArchives.mjs',
  'scripts/qualificationFrontendRecoveryArchives.mjs',
  'scripts/qualificationDeliveryArchives.mjs',
  'scripts/qualificationNativeCiRecoveryArchives.mjs',
  'tools/browser-qualification/candidate-direct-api.mjs',
  'tools/browser-qualification/candidate-direct-api-fixtures.ts',
  'tools/browser-qualification/candidate-direct-api-worker.ts',
  'scripts/qualificationPr58Archives.mjs',
  'scripts/qualificationFiberedSourceArchives.mjs',
  'scripts/qualificationIncomingMainArchives.mjs',
  'scripts/qualificationFrameCapArchives.mjs',
  'scripts/qualificationGeneralCertificateMainArchives.mjs',
  'scripts/browserMemoryErrors.mjs',
  'scripts/check-sweep-miter-matrix-browser.mjs',
  'scripts/sweep-browser-surface-evidence.mjs',
  'scripts/sweep-browser-worker-probe.mjs',
  'scripts/export-sweep-step-oracle.mts',
  'scripts/sweep-step-miter-fixtures.mts',
  'scripts/sweep-step-profile-fixtures.mts',
  '.github/workflows/g1-qualification-clean.yml',
  'scripts/materialize-qualification-kernels.mjs',
  'scripts/pack-geometry-kernel.mjs',
  'scripts/pack-language-kernel.mjs',
  'scripts/pack-photogrammetry.mjs',
  'scripts/build-wasm-brotli.mjs',
  'scripts/build-vr.mjs',
  'scripts/pack-harfbuzz.mjs',
  'scripts/browserPayloadTree.mjs',
  'scripts/g1RuntimeIdentity.mjs',
  'scripts/diagnose-g1-webkit-memory.mjs',
  'scripts/qualificationKernelModuleDelivery.mjs',
  'tests/geometryKernelModuleDelivery.test.ts',
  'tests/profileSolidAdmission.test.ts',
  'tests/profileSolidRush.test.ts',
  'tests/profileFramePremises.test.ts',
  'tests/spatialBishopSurface.test.ts',
  'examples/rush/spatial-bishop-g2-surface-6.r',
  'scripts/check-profile-solid-browser.mjs',
  'examples/rush/closed-profile-planar-10.r',
  'examples/rush/closed-profile-spatial-6.r',
  'scripts/export-profile-solid-step.mts',
  'scripts/verify-profile-solid-step.py',
  'scripts/g1-github-actions.mjs',
  'scripts/run-g1-candidate-clean-fragment.mjs',
  'scripts/run-g1-ubuntu-docker-fragment.mjs',
  'scripts/publish-runtime-direct-api.mjs',
  'scripts/qualificationPathRenamesV106.mjs',
])
export const REFRESH_OUTPUTS = Object.freeze({
  fingerprint: 'docs/qualification/g0-toolchain-fingerprints-v114.json',
  plan: 'docs/qualification/semantic-manifold-g1-plan-v131.json',
  status: 'docs/qualification/g0-v114-g1-v131-refreeze-status-v1.json',
})
const oldFingerprint = 'docs/qualification/g0-toolchain-fingerprints-v113.json'
const oldPlan = 'docs/qualification/semantic-manifold-g1-plan-v130.json'
const oldStatus = 'docs/qualification/g0-v113-g1-v130-refreeze-status-v1.json'
const oldReview = 'docs/qualification/g0-v113-g1-v130-refreeze-review.md'
const githubEnvironment = 'docs/qualification/environment-freeze/g1-github-actions-v41.json'
import {FROZEN_ARCHIVES} from './qualificationFrozenArchives.mjs'
import {PATH_RENAMES_SINCE_V105} from './qualificationPathRenamesV106.mjs'

/** A bound path that no longer exists is rebound to its recorded git rename target. */
export function followPathRename(root, path) {
  if (existsSync(resolve(root, path))) return path
  const renamed = Object.hasOwn(PATH_RENAMES_SINCE_V105, path) ? PATH_RENAMES_SINCE_V105[path] : null
  return renamed && existsSync(resolve(root, renamed)) ? renamed : path
}
export {FROZEN_ARCHIVES}

export function artifactBindingChanges(kind, before, after) {
  const changes=[]
    const nextById = new Map(after.map(item => [item.id, item]))
    for (const item of before) {
      const next = nextById.get(item.id)
      const oldHash = typeof item.sha256 === 'string' ? item.sha256 : item.sha256.value
      const nextHash = next ? typeof next.sha256 === 'string' ? next.sha256 : next.sha256.value : null
      if (oldHash !== nextHash) changes.push({ kind, id: item.id, previousSha256: oldHash, currentSha256: nextHash })
    }
    const previousIds = new Set(before.map(item => item.id))
    for (const item of after.filter(item => !previousIds.has(item.id))) {
      changes.push({kind, id:item.id, previousSha256:null,
        currentSha256:typeof item.sha256 === 'string' ? item.sha256 : item.sha256.value})
    }
  return changes
}

export function refreshInputPaths(root = defaultRoot) {
  const fingerprint = JSON.parse(ordinaryBytes(root, oldFingerprint))
  const plan = JSON.parse(ordinaryBytes(root, oldPlan))
  const ownRust = JSON.parse(ordinaryBytes(root, OWN_RUST_EVIDENCE))
  return [...new Set([
    ...Object.keys(FROZEN_ARCHIVES), REFRESH_SCRIPT, REFRESH_REVIEW, REFREEZE_CORE,
    githubEnvironment, ...REFRESH_EXECUTORS, ...MODULE_DELIVERY_PATHS,
    OWN_RUST_EVIDENCE, ...RUNTIME_EVIDENCE_PATHS, ...INTEGRATION_INPUT_PATHS,
    ...ownRust.sourceBundle.paths.map(path => followPathRename(root, path)),
    ...fingerprint.artifacts.map(item => followPathRename(root, item.path)),
    ...plan.bindings.artifacts.map(item => followPathRename(root, item.path)),
    ...plan.bindings.bundles.flatMap(item => item.paths).map(path => followPathRename(root, path)),
  ])].filter(path => existsSync(resolve(root, path))).sort()
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
  assertCandidateResultsAbsent(root, 'semantic-manifold-g1-candidate-run-v131')
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
  fingerprint.fingerprintId = 'g0-toolchain-fingerprints-v114'
  fingerprint.recordedAt = recordedAt
  fingerprint.previousFingerprintId = previousFingerprint.fingerprintId
  fingerprint.previousFingerprintSha256 = FROZEN_ARCHIVES[oldFingerprint]
  fingerprint.purpose = reason.trim()
  fingerprint.claimBoundary.excludes = [
    'Full SPDX SBOM', 'Playwright qualification locks',
    'Any rewrite of G0 v1-v113 or G1 v1 through v130',
    'G0 closure, G1 qualification, completed clean work, or production cutover',
  ]
  fingerprint.claimBoundary.includes = [
    ...new Set([
      ...fingerprint.claimBoundary.includes,
      'Exact SHA-256 of the source snapshot and independently recorded published WASM; compilation correspondence is not certified',
      'Exact SHA-256 of the V40 GitHub workflow, evidence harness, selectors, and hosted-runner identity freeze',
    ]),
  ]
  fingerprint.toolchain.rustChannel = channel
  fingerprint.toolchain.observedRustcVersionLine = observedRustcVersionLine
  fingerprint.artifacts = previousFingerprint.artifacts.map(item => ({ ...item, path: followPathRename(root, item.path) }))
    .filter(item => snapshot.has(item.path)).map(item => ({ ...item, ...snapshot.get(item.path) }))
  const ownRust = JSON.parse(ordinaryBytes(root, OWN_RUST_EVIDENCE))
  assert.deepEqual(ownRust.sourceBundle.paths, [...ownRust.sourceBundle.paths].sort(),
    'Own-Rust source bundle paths must be sorted')
  const wasmBytes = readFileSync(resolve(root, ownRust.wasm.path))
  const observedWasmSha256 = digest(wasmBytes)
  const recordedWasm = (ownRust.wasm.variants ?? [ownRust.wasm])
    .find(variant => variant.sha256 === observedWasmSha256)
  assert(recordedWasm, 'Own-Rust WASM fingerprint does not match a recorded build artifact')
  assert(wasmBytes.byteLength > 0 && wasmBytes.byteLength <= 16 * 1024 * 1024,
    'Own-Rust WASM exceeds the bounded artifact size')
  if (recordedWasm.byteLength !== null) assert.equal(wasmBytes.byteLength, recordedWasm.byteLength,
    'Own-Rust WASM byte length does not match recorded bytes')
  const survivingOwnRustPaths = ownRust.sourceBundle.paths.filter(path => snapshot.has(path))
  if (survivingOwnRustPaths.length === ownRust.sourceBundle.paths.length) {
    const sourceBundle = Buffer.concat(ownRust.sourceBundle.paths.flatMap(path => [
      Buffer.from(path), Buffer.from([0]), ordinaryBytes(root, path), Buffer.from('\n'),
    ]))
    assert.equal(digest(sourceBundle), ownRust.sourceBundle.sha256,
      'Own-Rust source fingerprint does not match final source bytes')
  }
  // With retired local paths, the historical whole-bundle digest cannot be reproduced.
  // OwnRust86 remains immutable historical evidence; every surviving path is rebound separately below.
  const existingArtifactIds = new Set(fingerprint.artifacts.map(item => item.id))
  const existingArtifactPaths = new Set(fingerprint.artifacts.map(item => item.path))
  let nextGeometryIndex = Math.max(0, ...fingerprint.artifacts
    .map(item => /^geometry-v3-source-(\d+)$/u.exec(item.id)?.[1])
    .filter(Boolean).map(Number))
  const geometryArtifacts = [
    { id: 'own-rust-cad-v86-evidence', path: OWN_RUST_EVIDENCE },
    ...[...ownRust.sourceBundle.paths, ...INTEGRATION_INPUT_PATHS]
      .map(path => followPathRename(root, path))
      .filter(path => snapshot.has(path) && !existingArtifactPaths.has(path))
      .map(path => ({
        id: `geometry-v3-source-${String(++nextGeometryIndex).padStart(2, '0')}`,
        path,
      })),
  ]
  fingerprint.artifacts.push(...geometryArtifacts
    .filter(item => !existingArtifactIds.has(item.id))
    .map(item => ({ ...item, ...snapshot.get(item.path) })))
  const qualificationArtifacts = [
    { id: 'g1-v131-github-workflow', path: '.github/workflows/g1-qualification-clean.yml' },
    { id: 'g1-v131-github-harness', path: 'scripts/g1-github-actions.mjs' },
    { id: 'g1-v131-clean-fragment-selector', path: 'scripts/run-g1-candidate-clean-fragment.mjs' },
    { id: 'g1-v131-ubuntu-selector', path: 'scripts/run-g1-ubuntu-docker-fragment.mjs' },
    { id: 'g1-v131-github-environment', path: githubEnvironment },
  ]
  fingerprint.artifacts.push(...qualificationArtifacts
    .filter(item => !existingArtifactPaths.has(item.path))
    .map(item => ({ ...item, ...snapshot.get(item.path) })))
  fingerprint.knownDrift = previousFingerprint.artifacts
    .filter(item => item.sha256 !== snapshot.get(followPathRename(root, item.path))?.sha256)
    .map(item => ({ relativeTo: oldFingerprint, bindingId: item.id,
      note: `Current bytes are newly bound in v114; v113 remains archived. ${reason.trim()}` }))

  const plan = structuredClone(previousPlan)
  for (const artifact of plan.bindings.artifacts) artifact.path = followPathRename(root, artifact.path)
  for (const bundle of plan.bindings.bundles) {
    bundle.paths = [...new Set(bundle.paths.map(path => followPathRename(root, path)))].sort()
  }
  plan.planId = 'semantic-manifold-g1-plan-v131'
  plan.processAmendment = {
    kind: 'post-freeze-harness-amendment', previousPlanId: previousPlan.planId,
    previousPlanSha256: FROZEN_ARCHIVES[oldPlan], reason: reason.trim(),
    changes: [
      'Recompute artifact and canonical bundle digests from current source bytes.',
      'Bind the exact GitHub Actions workflow, evidence-producing harness, selectors, and hosted-runner image identities.',
      'Start semantic-manifold-g1-candidate-run-v131 with zero completed clean runs and zero completed work units; no prior result is imported.',
      'Retain G0 v1-v113 and G1 v1 through v130 byte-for-byte; G0 toolchain fingerprints advance separately to v114.',
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
  for (const path of ['src/core/ownRustCadEvidence.ts', OWN_RUST_EVIDENCE, ...RUNTIME_EVIDENCE_PATHS, ...INTEGRATION_INPUT_PATHS, ...MODULE_DELIVERY_PATHS]) {
    if (!candidateBundle.paths.includes(path)) candidateBundle.paths.push(path)
  }
  candidateBundle.paths.sort()
  const harnessBundle = plan.bindings.bundles.find(bundle => bundle.id === 'g1-qualification-harness-bundle')
  assert(harnessBundle, 'G1 qualification harness bundle is missing')
  for (const path of REFRESH_EXECUTORS) {
    if (!harnessBundle.paths.includes(path)) harnessBundle.paths.push(path)
  }
  harnessBundle.paths.sort()
  if (!plan.bindings.artifacts.some(artifact => artifact.id === 'g1-github-actions-environment-v41')) {
    plan.bindings.artifacts.push({
      id: 'g1-github-actions-environment-v41',
      path: githubEnvironment,
      purpose: 'Exact hosted runner labels/image versions, action commit pins, source-SHA protocol, and downloaded archive identity-only boundary for V41 (observed hosted images and downloaded browser payloads).',
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
      verifiedBy: `V41 canonical Linux tree manifest for ${revision}; every required supporting tree is checked by the frozen GitHub harness`,
    }
  }
  plan.bindings.artifacts = plan.bindings.artifacts.filter(artifact => snapshot.has(artifact.path))
  for (const bundle of plan.bindings.bundles) bundle.paths = bundle.paths.filter(path => snapshot.has(path))
  for (const artifact of plan.bindings.artifacts) {
    const hash = snapshot.get(artifact.path)
    artifact.sha256 = { ...artifact.sha256, value: hash.sha256, byteLength: hash.byteLength }
  }
  for (const bundle of plan.bindings.bundles) {
    assert.deepEqual(bundle.paths, [...bundle.paths].sort(), `Unsorted bundle: ${bundle.id}`)
    const bytes = bundleBytes(bundle.paths, snapshot)
    bundle.sha256 = { ...bundle.sha256, value: digest(bytes), byteLength: bytes.byteLength }
  }
  plan.executionProtocol.candidateRunId = 'semantic-manifold-g1-candidate-run-v131'
  plan.executionProtocol.resultPath = 'output/qualification/semantic-manifold-g1-candidate-run-v131/result.json'
  plan.executionProtocol.cleanRunDefinition[0] = previousPlan.executionProtocol.cleanRunDefinition[0]
    .replace('exact v130 frozen artifact', 'exact v131 frozen artifact')
  assert.equal(plan.executionProtocol.priorResultsMayBeImported, false)
  assert.equal(plan.approvals.qualificationApproval, 'not-approved')
  assert(plan.matrix.every(row => row.evidenceState === 'not-executed-clean-post-freeze'))

  const artifactBytes = { fingerprint: jsonBytes(fingerprint), plan: jsonBytes(plan) }
  const changes = [
    ['g0-artifact', previousFingerprint.artifacts, fingerprint.artifacts],
    ['g1-artifact', previousPlan.bindings.artifacts, plan.bindings.artifacts],
    ['g1-bundle', previousPlan.bindings.bundles, plan.bindings.bundles],
  ].flatMap(([kind,before,after])=>artifactBindingChanges(kind,before,after))
  const status = {
    schema: 'open-scad-viewer/qualification-refreeze-status', schemaVersion: 1,
    statusId: 'g0-v114-g1-v131-refreeze-status-v1', recordedAt, reason: reason.trim(),
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
      'The workflow, GitHub harness, and two fragment helpers select v131 and are frozen inputs; no earlier fragment is imported.',
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
