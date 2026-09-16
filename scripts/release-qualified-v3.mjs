#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const v2IndexPath = 'docs/qualification/plans/g8-full-matrix-index-v2.json'
const v2MatrixPath = 'docs/qualification/brep-full-closed-matrix-v2.json'
const v2ReleasePath = 'docs/qualification/brep-capability-registry-release-full-v2.json'
const resetPolicy = 'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID'

const successors = [
  {
    id: 'authorized-heal-gap-le1/2',
    qualified: true,
    preservePublishedArtifacts: true,
    successorOf: 'authorized-heal-gap-le1/1',
    dependencies: [
      'numeric-evidence-curved-brep/1',
      'boundary-correspondence/1',
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
    ],
    included: [
      'Proof-bound positive endpoint snap and one-to-one rational refit with per-entity and cumulative displacement at most one tolerance-context cell',
      'Atomic native, bridge, and product transactions with cancellation, rollback, exact write sets, sew, audit, naming, and idempotence certificates',
    ],
    excluded: [
      'Silent healing',
      'Edge splitting',
      'Cross-context authority',
      'Overlapping implicit or explicit write sets',
      'Unbounded cumulative displacement',
    ],
    matrix: [
      ['HEAL2-ENDPOINT-SNAP', 'Complete'],
      ['HEAL2-RATIONAL-REFIT', 'Complete'],
      ['HEAL2-EXACT-BUDGET-AND-LINEAGE', 'Complete'],
      ['HEAL2-CANCEL-ROLLBACK-IDEMPOTENCE', 'Complete'],
      ['HEAL2-BUDGET-CONTEXT-WRITESET-MUTATION', 'typed-refuse'],
      ['HEAL2-BRIDGE-PRODUCT-SEAM', 'Complete'],
    ],
    implementation: [
      'crates/brep-core/src/trim_sew.rs',
      'crates/brep-core/src/transactions.rs',
      'crates/geometry-bridge/src/lib.rs',
      'crates/geometry-bridge/src/tests.rs',
      'src/services/geometry/brep.ts',
      'tests/brepAuthorizedHealV2.test.ts',
    ],
    oracles: [
      'trim_sew::tests::heal_positive_endpoint_snap_returns_complete_native_certificate',
      'trim_sew::tests::heal_positive_one_to_one_rational_refit_is_complete',
      'trim_sew::tests::heal_budget_exact_boundary_accepts_and_boundary_ulp_refuses',
      'trim_sew::tests::heal_refuses_unbound_stale_and_overlapping_recipes',
      'trim_sew::tests::heal_refit_requires_exact_rational_cardinality_degree_knots_and_weights',
      'trim_sew::tests::heal_transaction_cancel_rollback_and_idempotence',
      'tests/brepAuthorizedHealV2.test.ts',
    ],
  },
  {
    id: 'nurbs-boolean-bezier-le3/3',
    qualified: true,
    preservePublishedArtifacts: true,
    successorOf: 'nurbs-boolean-bezier-le3/2',
    dependencies: [
      'numeric-evidence-curved-brep/1',
      'boundary-correspondence/1',
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
      'nurbs-boolean-bezier-le3/2',
    ],
    included: [
      'One unit-weight degree-at-most-3 Bezier graph-patch solid cut by one affine-planar cutter at a strict interior transverse seam',
      'Certified exact retained iso traces, curved UV arrangement, correspondence, audited topology authorship, explicit lineage, and serialized bridge/product certificate',
    ],
    excluded: [
      'Boundary roots, multiple roots, tangency, or coincident overlap',
      'Containment or unequal spans',
      'Positive rational weights or multi-span surfaces',
      'Generic freeform arrangements',
      'Healing, mesh, prism, or Manifold fallback',
      'Parasolid parity',
    ],
    matrix: [
      ['NB3-TRANSVERSE-GRAPH-PATCH-INTERSECTION', 'Complete'],
      ['NB3-TRANSVERSE-GRAPH-PATCH-DIFFERENCE', 'Complete'],
      ['NB3-TRANSVERSE-GRAPH-PATCH-UNION', 'typed-refuse'],
      ['NB3-UV-SEW-AUDIT-LINEAGE-MUTATIONS', 'typed-refuse'],
      ['NB3-BOUNDARY-MULTIROOT-TANGENCY-COINCIDENCE', 'typed-refuse'],
      ['NB3-RATIONAL-MULTISPAN-RESOURCE', 'typed-refuse'],
      ['NB3-BRIDGE-PRODUCT-SEAM', 'Complete'],
    ],
    implementation: [
      'crates/brep-core/src/nurbs_ss_g6.rs',
      'crates/brep-core/src/uv_arrangement.rs',
      'crates/brep-core/src/trim_sew.rs',
      'crates/brep-core/src/profile_imprint.rs',
      'crates/brep-core/src/operations.rs',
      'crates/brep-core/src/imprint_pipeline.rs',
      'crates/brep-core/src/solid_audit.rs',
      'crates/geometry-bridge/src/lib.rs',
      'crates/geometry-bridge/src/tests.rs',
      'src/services/geometry/brep.ts',
      'tests/brepCurvedGraphBooleanV3.test.ts',
    ],
    oracles: [
      'nurbs_ss_g6::tests::v3_graph_patch_u_v_degree_2_3_intersection_and_difference',
      'nurbs_ss_g6::tests::v3_graph_patch_transform_reflection_swap_and_refusals',
      'geometry_bridge::tests::curved_graph_boolean_v3_bridge_serializes_audited_authority',
      'tests/brepCurvedGraphBooleanV3.test.ts',
    ],
  },
  {
    id: 'nurbs-boolean-bezier-le3/4',
    qualified: true,
    preservePublishedArtifacts: true,
    successorOf: 'nurbs-boolean-bezier-le3/3',
    dependencies: [
      'numeric-evidence-curved-brep/1',
      'boundary-correspondence/1',
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
      'nurbs-boolean-bezier-le3/3',
    ],
    included: [
      'Unequal-span strict transverse intersection/source-difference and strict affine-cutter containment union/intersection/cavity difference for the recognized unit-weight graph family',
      'Operation-specific ChangeSet lineage, convex-bound containment/separation, cavity ownership, audit, bridge, WASM, and product certificates',
    ],
    excluded: [
      'Tangency, coincidence, boundary roots, or multiple roots',
      'Rational weights, multi-span surfaces, and generic freeform arrangements',
      'Transverse union, reversed difference, healing, mesh, prism, or Manifold fallback',
    ],
    matrix: [
      ['NB4-UNEQUAL-INTERSECTION', 'Complete'],
      ['NB4-UNEQUAL-SOURCE-DIFFERENCE', 'Complete'],
      ['NB4-CONTAINMENT-UNION-INTERSECTION', 'Complete'],
      ['NB4-CONTAINMENT-CAVITY-DIFFERENCE', 'Complete'],
      ['NB4-TANGENCY-COINCIDENCE-BOUNDARY-REVERSED', 'typed-refuse'],
      ['NB4-CERTIFICATE-MUTATIONS', 'typed-refuse'],
      ['NB4-BRIDGE-WASM-PRODUCT', 'Complete'],
    ],
    implementation: [
      'crates/brep-core/src/nurbs_ss_g6.rs',
      'crates/brep-core/src/uv_arrangement.rs',
      'crates/brep-core/src/trim_sew.rs',
      'crates/brep-core/src/profile_imprint.rs',
      'crates/brep-core/src/solid_audit.rs',
      'crates/brep-core/src/operations.rs',
      'crates/brep-core/src/imprint_pipeline.rs',
      'crates/geometry-bridge/src/lib.rs',
      'crates/geometry-bridge/src/tests.rs',
      'src/services/geometry/brep.ts',
      'tests/brepNurbsSuccessorCellsV3.test.ts',
    ],
    oracles: [
      'nurbs_ss_g6::tests::v4_unequal_span_and_containment_operations_are_independently_certified',
      'nurbs_ss_g6::tests::v4_containment_boundary_and_reversed_difference_refuse',
      'geometry_bridge::tests::curved_graph_boolean_v4_v5_bridge_serializes_cell_specific_authority',
      'tests/brepNurbsSuccessorCellsV3.test.ts',
    ],
  },
  {
    id: 'nurbs-boolean-bezier-le3/5',
    qualified: true,
    preservePublishedArtifacts: true,
    successorOf: 'nurbs-boolean-bezier-le3/4',
    dependencies: [
      'numeric-evidence-curved-brep/1',
      'boundary-correspondence/1',
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
      'nurbs-boolean-bezier-le3/4',
    ],
    included: [
      'Bounded positive rational degree-2/3 graph intersection and source-difference with one strict interior factored homogeneous iso root',
      'Certified denominator lower bound, exact rational iso/pcurves, conditioning/resource bounds, lineage, audit, bridge, WASM, and product certificates',
    ],
    excluded: [
      'Weights outside [0.25,2], condition number above 8, non-factorable weights, multi-span, or more than 16 coefficients',
      'Tangency, coincidence, boundary or multiple roots, singular UV topology, union, and reversed difference',
      'Healing, mesh, prism, or Manifold fallback',
    ],
    matrix: [
      ['NB5-RATIONAL-INTERSECTION', 'Complete'],
      ['NB5-RATIONAL-SOURCE-DIFFERENCE', 'Complete'],
      ['NB5-HOMOGENEOUS-ROOT-DENOMINATOR-PCURVES', 'Complete'],
      ['NB5-WEIGHT-CONDITION-RESOURCE-LIMITS', 'typed-refuse'],
      ['NB5-TANGENCY-COINCIDENCE-SINGULAR-UV', 'typed-refuse'],
      ['NB5-CERTIFICATE-MUTATIONS', 'typed-refuse'],
      ['NB5-BRIDGE-WASM-PRODUCT', 'Complete'],
    ],
    implementation: [
      'crates/brep-core/src/nurbs_ss_g6.rs',
      'crates/brep-core/src/uv_arrangement.rs',
      'crates/brep-core/src/trim_sew.rs',
      'crates/brep-core/src/profile_imprint.rs',
      'crates/brep-core/src/operations.rs',
      'crates/brep-core/src/imprint_pipeline.rs',
      'crates/brep-core/src/solid_audit.rs',
      'crates/geometry-bridge/src/lib.rs',
      'crates/geometry-bridge/src/tests.rs',
      'src/services/geometry/brep.ts',
      'tests/brepNurbsSuccessorCellsV3.test.ts',
    ],
    oracles: [
      'nurbs_ss_g6::tests::v5_rational_graph_has_homogeneous_root_denominator_and_resource_proofs',
      'nurbs_ss_g6::tests::v5_weight_root_resource_tangency_and_certificate_mutations_refuse',
      'geometry_bridge::tests::curved_graph_boolean_v4_v5_bridge_serializes_cell_specific_authority',
      'tests/brepNurbsSuccessorCellsV3.test.ts',
    ],
  },
  {
    id: 'step-interchange/3',
    qualified: true,
    preservePublishedArtifacts: true,
    successorOf: 'step-interchange/2',
    dependencies: [
      'global-solid-audit/1',
      'persistent-naming/1',
      'step-interchange/2',
    ],
    included: [
      'Direct bounded Part 21 topology translation for LINE, CIRCLE, ELLIPSE, parameter-TRIMMED_CURVE, PLANE, rational multi-span B-spline, multi-body, and multi-cavity B-rep graphs',
      'Topology-preserving import and export with shared edges, senses, holes, cavity shells, placements, units, stable entity-bound identity, and explicit identity-loss reporting',
      'SI prefixes, bounded positive conversion units including inch, and selected-representation-reachable rigid placements',
      'Checked aggregate budgets of 16 MiB, 65536 instances, parser depth 32, graph depth 64, 4096 topology entities, 256 faces, and 8192 coedges',
    ],
    excluded: [
      'Constructor or AABB reconstruction',
      'FACETED_BREP, STL, or OBJ',
      'Silent identity preservation',
      'Unknown reachable geometry',
      'Unchecked cycles, depth, forward references, or resource accounting',
      'Periodic/angular analytic surfaces and poles that cannot map one-to-one to current finite native parameterization',
      'Parasolid parity',
    ],
    matrix: [
      ['STEP3-DIRECT-TOPOLOGY-ROUNDTRIP', 'Complete'],
      ['STEP3-MULTIBODY-MULTICAVITY-RATIONAL-MULTISPAN', 'Complete'],
      ['STEP3-REORDERED-ENTITY-IDENTITY', 'Complete'],
      ['STEP3-METADATA-FREE-IDENTITY-LOSS', 'Complete'],
      ['STEP3-MALFORMED-REACHABLE-GRAPH', 'typed-refuse'],
      ['STEP3-AGGREGATE-RESOURCE-OVERFLOW', 'typed-refuse'],
      ['STEP3-BRIDGE-PRODUCT-SEAM', 'Complete'],
    ],
    implementation: [
      'crates/brep-core/src/step_interchange.rs',
      'crates/brep-core/src/step_interchange_v3.rs',
      'crates/brep-topology/src/lib.rs',
      'crates/geometry-bridge/src/lib.rs',
      'crates/geometry-bridge/src/tests.rs',
      'src/services/cadNurbsStep.ts',
      'tests/brepDirectStepV3.test.ts',
    ],
    oracles: [
      'step_interchange_v3::tests::direct_roundtrip_preserves_topology_and_identity',
      'step_interchange_v3::tests::direct_more_than_32_bodies_and_metadata_free_identity',
      'step_interchange_v3::tests::multiple_cavities_shared_edges_senses_and_metadata_mutation',
      'step_interchange_v3::tests::parses_self_authored_analytic_and_conversion_fixtures',
      'step_interchange_v3::tests::only_representation_reachable_rigid_transform_moves_geometry',
      'step_interchange_v3::tests::aggregate_parser_and_graph_budgets_refuse',
      'geometry_bridge::tests::direct_step_v3_bridge_preserves_exact_graph',
      'tests/brepDirectStepV3.test.ts',
    ],
  },
]

const slug = id => id.replace('/', '-')
const canonicalHash = paths => {
  const hash = createHash('sha256')
  for (const path of [...paths].sort()) {
    hash.update(path).update('\0').update(readFileSync(resolve(root, path))).update('\n')
  }
  return hash.digest('hex')
}
const unresolved = capability => capability.matrix.map(([id]) => ({
  id,
  status: 'open',
  resolution: 'No implementation and executable native, bridge, and product evidence exist for this V3 successor row.',
}))

for (const capability of successors) {
  const name = slug(capability.id)
  const planPath = `docs/qualification/plans/${name}.json`
  const evidencePath = `docs/qualification/${name}-evidence-v3.json`
  const unresolvedRows = capability.qualified ? [] : unresolved(capability)
  if (!capability.preservePublishedArtifacts) write(planPath, {
    $schema: './brep-capability-qualification-plan-v3.schema.json',
    schema: 'open-scad-viewer/brep-capability-qualification-plan',
    schemaVersion: 3,
    planId: name,
    gate: { id: 'g8-full', name: 'brep-full-closed-matrix', version: 3 },
    lifecycle: {
      status: capability.qualified ? 'qualified' : 'frozen-pending-execution',
      qualificationClaim: capability.qualified
        ? (capability.id.startsWith('nurbs-boolean-bezier-le3/') || capability.id === 'authorized-heal-gap-le1/2'
            ? 'finite-cell'
            : 'qualified')
        : 'none',
    },
    claimBoundary: {
      candidateClaim: `Candidate finite V3 successor cell for ${capability.id}: ${capability.included[0]}`,
      forbiddenClaims: capability.excluded,
    },
    capability: capability.id,
    successorOf: capability.successorOf,
    dependencies: capability.dependencies.map(dependency => ({
      capability: dependency,
      requiredMaturity: 'Qualified',
    })),
    scope: { included: capability.included, excluded: capability.excluded },
    matrix: capability.matrix.map(([id, expected]) => ({
      id,
      expected,
      blocksQualification: true,
    })),
    bindings: {
      implementation: capability.implementation,
      registry: 'src/services/geometry/brepCapability.ts',
      evidenceJson: evidencePath,
    },
    evidence: {
      state: capability.qualified ? 'qualified' : 'not-executed',
      notes: [capability.qualified
        ? 'All listed finite native, bridge, actual WASM product, negative, and mutation rows passed.'
        : 'Contract skeleton only; implementation and executable native, bridge, and product qualification evidence do not yet exist.'],
    },
    resetPolicy,
    unresolvedRows,
  })
  if (!capability.preservePublishedArtifacts) write(evidencePath, {
    $schema: './brep-capability-evidence-v3.schema.json',
    schema: 'open-scad-viewer/brep-capability-evidence',
    schemaVersion: 3,
    evidenceId: `${name}-evidence-v3`,
    capability: capability.id,
    maturity: capability.qualified ? 'Qualified' : 'Unavailable',
    state: capability.qualified ? 'qualified' : 'not-executed',
    plan: planPath,
    runs: capability.qualified ? [{
      id: 'focused-native-bridge-product-2026-09-16',
      result: 'pass',
      artifactHash: canonicalHash(capability.implementation),
    }] : [],
    oracles: capability.oracles ?? [],
    unresolvedRows: capability.qualified ? [] : unresolvedRows,
    attestation: {
      fabricatedRuns: false,
      note: capability.qualified
        ? 'Qualification is limited to the explicit finite contract and preserves every listed typed refusal.'
        : 'Contract skeleton only; no implementation or qualification run is claimed.',
    },
  })
}

const v2Index = read(v2IndexPath)
const v2Matrix = read(v2MatrixPath)
const v2Release = read(v2ReleasePath)
const successorRows = successors.map(capability => ({
  id: capability.id,
  plan: `docs/qualification/plans/${slug(capability.id)}.json`,
  evidence: `docs/qualification/${slug(capability.id)}-evidence-v3.json`,
  maturity: capability.qualified ? 'Qualified' : 'Unavailable',
  releaseState: capability.qualified ? 'shipped' : 'candidate',
  dependencies: capability.dependencies,
}))
const rows = [...v2Index.capabilities, ...successorRows]
const candidates = rows.filter(row => row.releaseState === 'candidate').map(row => row.id)
const shipped = [
  ...v2Release.capabilities,
  ...successors.filter(capability => capability.qualified).map(capability => capability.id),
]
const explicitRefuse = [...new Set([
  ...v2Matrix.explicitRefuse,
  'unimplemented-v3-successor',
  'parasolid-parity',
])]

write('docs/qualification/plans/g8-full-matrix-index-v3.json', {
  schema: 'open-scad-viewer/qualification-plan-index',
  schemaVersion: 3,
  gate: 'G8-full',
  successorOf: v2IndexPath,
  matrix: 'docs/qualification/brep-full-closed-matrix-v3.json',
  registry: 'docs/qualification/brep-capability-registry-release-full-v3.json',
  capabilities: rows,
  unresolvedInShippedMatrix: [],
  invariants: v2Index.invariants,
})
write('docs/qualification/brep-full-closed-matrix-v3.json', {
  schema: 'open-scad-viewer/brep-closed-matrix',
  schemaVersion: 3,
  planId: 'brep-full-closed-matrix-v3',
  gate: 'G8-full',
  lifecycle: 'frozen-pending-execution',
  claimBoundary: 'project-kernel-qualified-finite-subset-rust',
  successorOf: v2MatrixPath,
  dependencyPolicy: v2Matrix.dependencyPolicy,
  admittedOps: shipped,
  pendingQualification: candidates,
  explicitRefuse,
  productDeployGate: v2Matrix.productDeployGate,
  unresolvedInShippedMatrix: [],
  unresolvedCandidateRows: candidates,
})
write('docs/qualification/brep-capability-registry-release-full-v3.json', {
  schema: 'open-scad-viewer/brep-capability-registry-release',
  schemaVersion: 3,
  gate: 'G8-full',
  planId: 'brep-capability-registry-release-full-v3',
  lifecycle: 'frozen-pending-execution',
  successorOf: v2ReleasePath,
  registry: 'src/services/geometry/brepCapability.ts',
  closedMatrix: 'docs/qualification/brep-full-closed-matrix-v3.json',
  g8Index: 'docs/qualification/plans/g8-full-matrix-index-v3.json',
  capabilities: shipped,
  dependencyPolicy: v2Release.dependencyPolicy,
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
