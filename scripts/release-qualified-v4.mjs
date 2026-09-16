#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const v3IndexPath = 'docs/qualification/plans/g8-full-matrix-index-v3.json'
const v3MatrixPath = 'docs/qualification/brep-full-closed-matrix-v3.json'
const v3ReleasePath = 'docs/qualification/brep-capability-registry-release-full-v3.json'
const capability = {
  id: 'nurbs-boolean-bezier-le3/6',
  successorOf: 'nurbs-boolean-bezier-le3/5',
  plan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-6.json',
  evidence: 'docs/qualification/nurbs-boolean-bezier-le3-6-evidence-v4.json',
  dependencies: [
    'numeric-evidence-curved-brep/1',
    'boundary-correspondence/1',
    'exact-sew/1',
    'global-solid-audit/1',
    'persistent-naming/1',
    'nurbs-boolean-bezier-le3/5',
  ],
  included: [
    'Independent general NURBS Boolean walking slice for bounded non-periodic positive-weight tensor-product surfaces with degree 1 through 3 in each parameter and finite non-degenerate knot spans',
    'Exact knot-insertion decomposition of every participating span to rational Bezier form, with source-span identity and no sampling, fitting, degree truncation, or mesh substitution',
    'Two or more disjoint regular transverse intersection branches, including branches crossing span boundaries and closed branches, with exact half-open [k_i,k_i+1) knot-cell ownership in each parameter and only the final non-periodic span closed at its terminal boundary',
    'Certified joining of per-cell branch fragments by exact rational boundary identity, orientation, and one-to-one incidence into globally disjoint branch chains or cycles',
    'Operand-local UV DCEL construction, trimming, exact correspondence, sew, global solid audit, persistent naming, and closed-solid union, intersection, source-minus-tool, and tool-minus-source authorship',
    'Checked finite resource accounting for knot spans, candidate span pairs, branch fragments, joined branches, UV vertices and half-edges, authored topology, and certificate bytes',
  ],
  excluded: [
    'Tangency, higher-order contact, coincidence, overlap intervals or regions, and branch junctions',
    'Singular poles, zero or negative weights, denominator lower bounds that cannot be certified positive, degenerate knot spans, and invalid knot multiplicities',
    'Periodic or closed parameter seams and seam-crossing ownership',
    'Degree above 3 in either parameter and implicit degree reduction or approximation',
    'Unbounded work, unchecked resource overflow, or ambiguous branch ownership or joining',
    'Open-shell output, healing, mesh, prism, or Manifold fallback, and Parasolid parity',
  ],
  matrix: [
    ['NB6-EXACT-PER-SPAN-BEZIER-DECOMPOSITION', 'Complete'],
    ['NB6-MULTISPAN-MULTIBRANCH-TRANSVERSE-WALK', 'Complete'],
    ['NB6-HALF-OPEN-KNOT-OWNERSHIP', 'Complete'],
    ['NB6-BRANCH-JOIN-CHAINS-AND-CYCLES', 'Complete'],
    ['NB6-OPERAND-UV-DCEL', 'Complete'],
    ['NB6-CLOSED-SOLID-BOOLEAN-AUTHORSHIP', 'Complete'],
    ['NB6-RESOURCE-AND-CERTIFICATE-MUTATIONS', 'typed-refuse'],
    ['NB6-TANGENCY-COINCIDENCE-SINGULAR-PERIODIC-HIGH-DEGREE', 'typed-refuse'],
    ['NB6-NATIVE-BRIDGE-WASM-PRODUCT', 'Complete'],
  ],
  implementation: [
    'crates/brep-core/src/nurbs_ss_g6.rs',
    'crates/brep-core/src/uv_arrangement.rs',
    'crates/brep-core/src/trim_sew.rs',
    'crates/brep-core/src/operations.rs',
    'crates/brep-core/src/solid_audit.rs',
    'crates/geometry-bridge/src/lib.rs',
    'src/services/geometry/brep.ts',
  ],
}
const resetPolicy = 'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID'
const unresolvedRows = capability.matrix.map(([id]) => ({
  id,
  status: 'open',
  resolution: 'No implementation and executable native, bridge, WASM, and product evidence exist for this V4 candidate row.',
}))

write(capability.plan, {
  $schema: './brep-capability-qualification-plan-v4.schema.json',
  schema: 'open-scad-viewer/brep-capability-qualification-plan',
  schemaVersion: 4,
  planId: 'nurbs-boolean-bezier-le3-6',
  gate: { id: 'g8-full', name: 'brep-full-closed-matrix', version: 4 },
  lifecycle: {
    status: 'frozen-pending-execution',
    qualificationClaim: 'none',
  },
  claimBoundary: {
    candidateClaim: 'Candidate independent general NURBS Boolean walking slice for exact multi-span decomposition and multiple disjoint transverse branches; no implementation is qualified.',
    forbiddenClaims: capability.excluded,
  },
  capability: capability.id,
  successorOf: capability.successorOf,
  dependencies: capability.dependencies.map(dependency => ({
    capability: dependency,
    requiredMaturity: 'Qualified',
  })),
  scope: {
    included: capability.included,
    excluded: capability.excluded,
  },
  matrix: capability.matrix.map(([id, expected]) => ({
    id,
    expected,
    blocksQualification: true,
  })),
  bindings: {
    implementation: capability.implementation,
    registry: 'src/services/geometry/brepCapability.ts',
    evidenceJson: capability.evidence,
  },
  evidence: {
    state: 'not-executed',
    notes: [
      'Contract skeleton only; implementation and executable native, bridge, WASM, product, negative, and mutation qualification evidence do not yet exist.',
      'All V3 Qualified claims remain preserved and are not evidence for this successor.',
    ],
  },
  resetPolicy,
  unresolvedRows,
})

write(capability.evidence, {
  $schema: './brep-capability-evidence-v4.schema.json',
  schema: 'open-scad-viewer/brep-capability-evidence',
  schemaVersion: 4,
  evidenceId: 'nurbs-boolean-bezier-le3-6-evidence-v4',
  capability: capability.id,
  maturity: 'Unavailable',
  state: 'not-executed',
  plan: capability.plan,
  runs: [],
  oracles: [],
  unresolvedRows,
  attestation: {
    fabricatedRuns: false,
    note: 'Append-only candidate skeleton only; no implementation, run, oracle, or qualification is claimed.',
  },
})

const v3Index = read(v3IndexPath)
const v3Matrix = read(v3MatrixPath)
const v3Release = read(v3ReleasePath)
const candidateRow = {
  id: capability.id,
  plan: capability.plan,
  evidence: capability.evidence,
  maturity: 'Unavailable',
  releaseState: 'candidate',
  dependencies: capability.dependencies,
}
const candidates = [
  ...v3Index.capabilities
    .filter(row => row.releaseState === 'candidate')
    .map(row => row.id),
  capability.id,
]
const explicitRefuse = [...new Set([
  ...v3Matrix.explicitRefuse,
  'unimplemented-v4-successor',
  'nurbs-tangency-or-coincidence',
  'nurbs-singular-poles',
  'nurbs-periodic-seams',
  'nurbs-degree-gt3',
])]

write('docs/qualification/plans/g8-full-matrix-index-v4.json', {
  schema: 'open-scad-viewer/qualification-plan-index',
  schemaVersion: 4,
  gate: 'G8-full',
  successorOf: v3IndexPath,
  matrix: 'docs/qualification/brep-full-closed-matrix-v4.json',
  registry: 'docs/qualification/brep-capability-registry-release-full-v4.json',
  capabilities: [...v3Index.capabilities, candidateRow],
  unresolvedInShippedMatrix: [],
  invariants: v3Index.invariants,
})
write('docs/qualification/brep-full-closed-matrix-v4.json', {
  schema: 'open-scad-viewer/brep-closed-matrix',
  schemaVersion: 4,
  planId: 'brep-full-closed-matrix-v4',
  gate: 'G8-full',
  lifecycle: 'frozen-pending-execution',
  claimBoundary: v3Matrix.claimBoundary,
  successorOf: v3MatrixPath,
  dependencyPolicy: v3Matrix.dependencyPolicy,
  admittedOps: v3Matrix.admittedOps,
  pendingQualification: candidates,
  explicitRefuse,
  productDeployGate: v3Matrix.productDeployGate,
  unresolvedInShippedMatrix: [],
  unresolvedCandidateRows: candidates,
})
write('docs/qualification/brep-capability-registry-release-full-v4.json', {
  schema: 'open-scad-viewer/brep-capability-registry-release',
  schemaVersion: 4,
  gate: 'G8-full',
  planId: 'brep-capability-registry-release-full-v4',
  lifecycle: 'frozen-pending-execution',
  successorOf: v3ReleasePath,
  registry: 'src/services/geometry/brepCapability.ts',
  closedMatrix: 'docs/qualification/brep-full-closed-matrix-v4.json',
  g8Index: 'docs/qualification/plans/g8-full-matrix-index-v4.json',
  capabilities: v3Release.capabilities,
  dependencyPolicy: v3Release.dependencyPolicy,
  excludedPendingQualification: candidates,
  explicitRefuse,
  unresolvedInShippedMatrix: [],
})

function read(path) {
  return JSON.parse(readFileSync(resolve(root, path), 'utf8'))
}

function write(path, value) {
  const absolute = resolve(root, path)
  mkdirSync(dirname(absolute), { recursive: true })
  writeFileSync(absolute, `${JSON.stringify(value, null, 2)}\n`)
}
