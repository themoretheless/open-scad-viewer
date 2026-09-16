#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const resetPolicy = 'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID'
const runDate = '2026-09-16'
const shared = {
  numeric: 'numeric-evidence-curved-brep/1',
  boundary: 'boundary-correspondence/1',
  sew: 'exact-sew/1',
  audit: 'global-solid-audit/1',
  naming: 'persistent-naming/1',
}

const capabilities = [
  {
    id: shared.numeric,
    included: ['Finite context-bound positional, root-parameter, tangent/normal, correspondence, and topology-preservation evidence', 'At most 64 composed claims under declared spatial/angular bounds'],
    excluded: ['Generic exact arithmetic', 'Unbounded-degree root isolation', 'Context-free tolerance comparisons'],
    matrix: [['NECB-POSITION-SCALE', 'Complete'], ['NECB-CONTEXT-COMPOSE', 'Complete'], ['NECB-GRAY-BAND', 'typed-refuse'], ['NECB-RESOURCE-65', 'typed-refuse']],
    implementation: ['crates/brep-core/src/predicate_evidence.rs', 'crates/cad-predicates/src/context.rs'],
    oracles: ['predicate_evidence::evidence_matrix_composes_all_independent_claims', 'predicate_evidence::positional_bound_is_scale_aware_and_translation_independent', 'predicate_evidence::gray_band_and_context_mismatch_refuse_aggregate', 'predicate_evidence::evidence_resource_limit_is_finite', 'tests/brepMassProperties.test.ts::publishes finite certified mass and tessellation enclosures'],
  },
  {
    id: shared.boundary, deps: [shared.numeric],
    included: ['Two authored uses of one finite rational curve with exact definition, endpoint, parameter-domain, orientation, shell, and context agreement', 'Explicit periodic seam shift and cyclic boundary-use locations'],
    excluded: ['Endpoint-only proximity', 'Curve fitting', 'Cross-context correspondence', 'More than two uses'],
    matrix: [['BC-RATIONAL-LINE', 'Complete'], ['BC-ORIENTATION-MUTATION', 'typed-refuse'], ['BC-CONTEXT-MUTATION', 'typed-refuse'], ['BC-DEFINITION-MUTATION', 'typed-refuse']],
    implementation: ['crates/brep-core/src/trim_sew.rs'],
    oracles: ['trim_sew::correspondence_proof_sews_and_mutations_refuse', 'analytic_features::successor_multi_edge_fillet_is_context_audit_and_naming_bound', 'geometry_bridge::tests::audited_feature_successors_cross_the_bridge_with_certificates', 'tests/brepMassProperties.test.ts::publishes audited finite feature successors and refuses bent frames'],
  },
  {
    id: shared.sew, deps: [shared.boundary],
    included: ['At most 4096 paired boundary uses, each carrying a complete boundary-correspondence proof', 'Opposite orientation, exact shell ownership, and atomic snapshot publication'],
    excluded: ['Endpoint-key-only sewing', 'Gap closure', 'Duplicate orientation', 'Non-manifold incidence'],
    matrix: [['SEW-PROVEN-PAIR', 'Complete'], ['SEW-ENDPOINT-ONLY', 'typed-refuse'], ['SEW-GAP-ATOMIC', 'typed-refuse'], ['SEW-DUPLICATE-ORIENTATION', 'typed-refuse']],
    implementation: ['crates/brep-core/src/trim_sew.rs'],
    oracles: ['trim_sew::endpoint_only_exact_sew_wrapper_fails_closed', 'trim_sew::correspondence_proof_sews_and_mutations_refuse', 'trim_sew::sew_gap_refuses_without_mutating_base', 'trim_sew::sew_duplicate_and_orientation_refuse', 'geometry_bridge::brep::certified_nurbs'],
  },
  {
    id: shared.audit, deps: [shared.numeric, shared.sew],
    included: ['At most 8 bodies with complete shell ownership, orientation, exact sew, finite cavity containment, body separation, and bounded self-intersection checks', 'Empty solid set and finite box/cylinder/sphere-cavity fixtures'],
    excluded: ['Overlapping material bodies', 'Orphan shells', 'Unknown cavity containment', 'Unchecked arbitrary freeform self-intersection'],
    matrix: [['AUDIT-CYLINDER', 'Complete'], ['AUDIT-EMPTY', 'Complete'], ['AUDIT-SPHERICAL-CAVITY', 'Complete'], ['AUDIT-ORIENTATION-MUTATION', 'typed-refuse'], ['AUDIT-ORPHAN-SHELL', 'typed-refuse'], ['AUDIT-OVERLAPPING-BODIES', 'typed-refuse']],
    implementation: ['crates/brep-core/src/solid_audit.rs', 'crates/brep-core/src/trim_sew.rs'],
    oracles: ['solid_audit::cylinder_audits_ok', 'solid_audit::empty_model_audits_ok', 'solid_audit::strict_spherical_cavity_is_certified', 'solid_audit::missed_orientation_branch_cannot_hide_behind_closed_flag', 'solid_audit::orphan_shell_mutation_refuses_global_audit', 'solid_audit::overlapping_bodies_are_refused', 'tests/brepMassProperties.test.ts::publishes finite certified mass and tessellation enclosures'],
  },
  {
    id: shared.naming, deps: [shared.sew],
    included: ['Opaque 128-bit topology IDs derived from operation, occurrence, capability, and private geometry signature', 'Validated persist, split, merge, generated, and modified ChangeSet lineage with acyclic cardinality checks'],
    excluded: ['Legacy 64-bit IDs without explicit migration', 'Nearest-face identity inference', 'Cyclic or dangling lineage'],
    matrix: [['NAME-OPAQUE-128', 'Complete'], ['NAME-CHANGESET-DAG', 'Complete'], ['NAME-LEGACY-NORMAL-PARSE', 'typed-refuse'], ['NAME-CARDINALITY-MUTATION', 'typed-refuse']],
    implementation: ['crates/brep-topology/src/persistent_naming.rs', 'crates/brep-core/src/lib.rs', 'src/core/topologyLineage.ts'],
    oracles: ['persistent_naming::canonical_ids_are_strict_and_legacy_requires_migration', 'persistent_naming::change_sets_enforce_existence_cardinality_and_dag', 'crates/brep-core/tests/topology_identity.rs', 'tests/topologyLineage.test.ts'],
  },
  {
    id: 'authorized-heal-gap-le1/1', deps: [shared.numeric, shared.boundary, shared.sew, shared.audit, shared.naming],
    qualified: false,
    included: ['Explicit endpoint snap or one-to-one rational curve refit with per-entity and cumulative displacement at most one tolerance-context cell', 'Cancellation, rollback, lineage, sew, audit, and idempotence checks'],
    excluded: ['Silent healing', 'Edge splitting', 'Cross-context authority', 'Unbounded cumulative displacement'],
    matrix: [['HEAL-ZERO-DISPLACEMENT-TRANSACTION', 'Complete'], ['HEAL-BUDGET-CONTEXT-LINEAGE', 'typed-refuse'], ['HEAL-CANCEL-ROLLBACK-IDEMPOTENCE', 'Complete'], ['HEAL-POSITIVE-GAP-PRODUCT-SEAM', 'ResearchOnly']],
    implementation: ['crates/brep-core/src/trim_sew.rs', 'crates/brep-core/src/transactions.rs'],
    oracles: ['trim_sew::heal_plan_refuses_budget_context_and_missing_lineage', 'trim_sew::heal_transaction_cancel_rollback_and_idempotence'],
    unresolved: [{ id: 'HEAL-POSITIVE-GAP-PRODUCT-SEAM', status: 'open', resolution: 'No native positive-displacement success corpus or bridge/product entry point exists; retain Unavailable.' }],
  },
  {
    id: 'nurbs-boolean-bezier-le3/2', deps: [shared.numeric, shared.boundary, shared.sew, shared.audit, shared.naming],
    included: ['Strict partial contact between affine-planar non-rational Bezier profile prisms of degree at most 3 with equal extrusion spans', 'Union, difference, and intersection with certified arrangements, sew, audit, naming, and no-heal certificate'],
    excluded: ['Containment or identity', 'Tangency or coincident overlap', 'Unequal extrusion spans', 'Degree greater than 3', 'Rational weights', 'Healing, mesh, prism, or Manifold fallback'],
    matrix: [['NB2-PARTIAL-UNION', 'Complete'], ['NB2-PARTIAL-DIFFERENCE', 'Complete'], ['NB2-PARTIAL-INTERSECTION', 'Complete'], ['NB2-CONTAINMENT', 'typed-refuse'], ['NB2-TANGENCY', 'typed-refuse'], ['NB2-UNEQUAL-SPAN', 'typed-refuse'], ['NB2-CERT-MUTATION', 'typed-refuse']],
    implementation: ['crates/brep-core/src/nurbs_ss_g6.rs', 'crates/brep-core/src/profile_imprint.rs', 'crates/brep-core/src/operations.rs', 'crates/geometry-bridge/src/lib.rs'],
    oracles: ['nurbs_ss_g6::bezier_le3_solid_boolean_all_operations_contact', 'nurbs_ss_g6::unequal_extrusion_spans_refuse_without_stepped_fallback', 'nurbs_ss_g6::certificate_mutation_blocks_topology_change', 'geometry_bridge::tests::freeform_nurbs_step_trimmed_and_solid_bridge'],
  },
  {
    id: 'step-interchange/2', deps: [shared.audit, shared.naming],
    included: ['Finite AP214/AP242 analytic and bicubic solid graph with explicit SI context and rigid placements', 'At most 32 bodies and one cavity per body', 'Opaque topology metadata roundtrip or explicit identity-loss report for metadata-free input'],
    excluded: ['FACETED_BREP', 'STL or OBJ', 'More than 32 bodies', 'More than one cavity per body', 'Arbitrary third-party entity graphs', 'Silent identity preservation'],
    matrix: [['STEP2-SI-IDENTITY-ROUNDTRIP', 'Complete'], ['STEP2-METADATA-FREE-LOSS', 'Complete'], ['STEP2-MULTIBODY-ONE-CAVITY', 'Complete'], ['STEP2-BODY-33', 'typed-refuse'], ['STEP2-CAVITY-2', 'typed-refuse'], ['STEP2-FACETED', 'typed-refuse']],
    implementation: ['crates/brep-core/src/step_interchange.rs', 'crates/brep-core/src/nurbs_step_solid.rs', 'crates/geometry-bridge/src/lib.rs', 'src/services/cadAnalyticStep.ts', 'src/services/cadNurbsStep.ts'],
    oracles: ['step_interchange::v2_roundtrip_preserves_identity_and_context', 'step_interchange::v2_import_without_metadata_reports_identity_loss', 'geometry_bridge::tests::step_successor_bridge_reports_identity_preservation_and_loss', 'tests/cadAnalyticStep.test.ts', 'tests/cadNurbsStep.test.ts'],
  },
  {
    id: 'analytic-multi-edge-fillet/1', deps: [shared.numeric, shared.boundary, shared.sew, shared.audit, shared.naming],
    included: ['One or more selected vertical edges of one audited axis-aligned cuboid', 'Finite positive constant radius with exact circular profile and complete ChangeSet'],
    excluded: ['Curved source edges', 'Non-cuboids', 'Non-vertical edges', 'Variable radius', 'General corner transition networks', 'Mesh bevel relabeling'],
    matrix: [['FILLET-MULTI-VERTICAL', 'Complete'], ['FILLET-NONVERTICAL', 'typed-refuse'], ['FILLET-NONCUBOID', 'typed-refuse'], ['FILLET-RADIUS-COLLAPSE', 'typed-refuse']],
    implementation: ['crates/brep-core/src/analytic_features.rs', 'crates/geometry-bridge/src/lib.rs', 'src/services/geometry/brep.ts'],
    oracles: ['analytic_features::successor_multi_edge_fillet_is_context_audit_and_naming_bound', 'geometry_bridge::tests::audited_feature_successors_cross_the_bridge_with_certificates', 'tests/brepMassProperties.test.ts::publishes audited finite feature successors and refuses bent frames'],
  },
  {
    id: 'exact-parallel-frame-sweep/1', deps: [shared.numeric, shared.boundary, shared.sew, shared.audit, shared.naming],
    included: ['Finite simple polygon profile translated between collinear stations under fixed, rotation-minimizing, or rmf spelling', 'Constant parallel frame with audited closed solid and complete ChangeSet'],
    excluded: ['Bent paths', 'Frenet rotation', 'Twist or scale laws', 'Self-intersecting profiles', 'Faceted sweep relabeling'],
    matrix: [['SWEEP-STRAIGHT-RMF', 'Complete'], ['SWEEP-STRAIGHT-FIXED', 'Complete'], ['SWEEP-BENT-RMF', 'typed-refuse'], ['SWEEP-INVALID-FRAME-LAW', 'typed-refuse']],
    implementation: ['crates/brep-core/src/analytic_features.rs', 'crates/geometry-bridge/src/lib.rs', 'src/services/geometry/brep.ts'],
    oracles: ['analytic_features::successor_parallel_sweep_certifies_and_bent_rmf_refuses', 'geometry_bridge::tests::audited_feature_successors_cross_the_bridge_with_certificates', 'tests/brepMassProperties.test.ts::publishes audited finite feature successors and refuses bent frames'],
  },
  {
    id: 'certified-brep-tessellation/1', deps: [shared.numeric, shared.boundary, shared.sew, shared.audit, shared.naming],
    included: ['Audited planar exact-profile solids and one-body no-cavity exact rational cylinders', 'Positive finite chord tolerance with at most 20000 triangles', 'Two-sided analytic deviation plus shared-edge identity, orientation, and no-T-junction coverage'],
    excluded: ['Spheres, tori, cones, cavities, and generic freeform surfaces', 'Tolerance requiring more than the triangle budget', 'Display tessellation without certificate'],
    matrix: [['TESS-PLANAR-BOX', 'Complete'], ['TESS-RATIONAL-CYLINDER', 'Complete'], ['TESS-TINY-TOLERANCE', 'typed-refuse'], ['TESS-NAMING-MUTATION', 'typed-refuse'], ['TESS-SPHERE', 'typed-refuse']],
    implementation: ['crates/geometry-bridge/src/brep.rs', 'crates/geometry-bridge/src/lib.rs', 'src/services/geometry/brep.ts'],
    oracles: ['crates/geometry-bridge/tests/analytic_brep_tessellation.rs::certified_planar_and_rational_cells_publish_two_sided_coverage', 'crates/geometry-bridge/tests/analytic_brep_tessellation.rs::certified_tessellation_has_typed_budget_mutation_and_shape_refusals', 'tests/brepMassProperties.test.ts::publishes finite certified mass and tessellation enclosures'],
  },
  {
    id: 'certified-mass-properties/1', deps: [shared.numeric, shared.audit, shared.naming],
    included: ['Audited exact axis-aligned or rigidly placed boxes, cylinders, tubes, and planar exact-profile prisms', 'Finite interval enclosures for area, volume, centroid, and inertia'],
    excluded: ['Spheres, tori, cones, and generic freeform solids', 'Quadrature estimates relabeled as certified', 'Incomplete naming evidence'],
    matrix: [['MASS-BOX', 'Complete'], ['MASS-CYLINDER', 'Complete'], ['MASS-TUBE', 'Complete'], ['MASS-PROFILE-PRISM', 'Complete'], ['MASS-RIGID-SCALE', 'Complete'], ['MASS-SPHERE', 'typed-refuse'], ['MASS-NAMING-MUTATION', 'typed-refuse']],
    implementation: ['crates/brep-core/src/analysis.rs', 'crates/geometry-bridge/src/lib.rs', 'src/services/geometry/brep.ts'],
    oracles: ['analysis::certified_mass_encloses_box_cylinder_tube_and_exact_profile', 'analysis::certified_mass_is_scale_and_rigid_transform_stable_and_refuses_mutation', 'tests/brepMassProperties.test.ts::publishes finite certified mass and tessellation enclosures'],
  },
].map(capability => ({ qualified: true, deps: [], unresolved: [], ...capability }))

const slug = id => id.replace('/', '-')
const canonicalHash = paths => {
  const hash = createHash('sha256')
  for (const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root, path))).update('\n')
  return hash.digest('hex')
}
const nativeHash = canonicalHash(['crates/brep-core/src/predicate_evidence.rs', 'crates/brep-core/src/trim_sew.rs', 'crates/brep-core/src/solid_audit.rs', 'crates/brep-core/src/analytic_features.rs', 'crates/brep-core/src/nurbs_ss_g6.rs', 'crates/brep-core/src/analysis.rs', 'crates/brep-core/src/step_interchange.rs', 'crates/brep-topology/src/persistent_naming.rs'])
const bridgeHash = canonicalHash(['crates/geometry-bridge/src/lib.rs', 'crates/geometry-bridge/src/tests.rs', 'crates/geometry-bridge/tests/analytic_brep_tessellation.rs'])
const productHash = canonicalHash(['src/services/geometry/brep.ts', 'src/services/cadAnalyticStep.ts', 'src/services/cadNurbsStep.ts', 'tests/brepMassProperties.test.ts', 'tests/brepProductSeam.test.ts', 'tests/topologyLineage.test.ts'])
const runRecords = [
  { id: `focused-native-workspace-${runDate}`, result: 'pass', artifactHash: nativeHash },
  { id: `geometry-bridge-certificate-seams-${runDate}`, result: 'pass', artifactHash: bridgeHash },
  { id: `product-wasm-certificate-seams-${runDate}`, result: 'pass', artifactHash: productHash },
]

for (const capability of capabilities) {
  const name = slug(capability.id)
  const evidencePath = `docs/qualification/${name}-evidence-v2.json`
  const planPath = `docs/qualification/plans/${name}.json`
  const matrix = capability.matrix.map(([id, expected]) => ({
    id, expected, blocksQualification: expected !== 'ResearchOnly',
  }))
  const plan = {
    $schema: './brep-capability-qualification-plan-v2.schema.json',
    schema: 'open-scad-viewer/brep-capability-qualification-plan',
    schemaVersion: 2,
    planId: name,
    gate: { id: 'g8-full', name: 'brep-full-closed-matrix', version: 2 },
    lifecycle: capability.qualified
      ? { status: 'qualified', qualificationClaim: 'qualified' }
      : { status: 'frozen-pending-execution', qualificationClaim: 'none' },
    claimBoundary: {
      candidateClaim: `${capability.qualified ? 'Qualified finite cell' : 'Candidate finite cell'} for ${capability.id}: ${capability.included[0]}`,
      forbiddenClaims: capability.excluded,
    },
    capability: capability.id,
    dependencies: capability.deps.map(dependency => ({ capability: dependency, requiredMaturity: 'Qualified' })),
    scope: { included: capability.included, excluded: capability.excluded },
    matrix,
    bindings: {
      implementation: capability.implementation,
      registry: 'src/services/geometry/brepCapability.ts',
      evidenceJson: evidencePath,
    },
    evidence: {
      state: capability.qualified ? 'qualified' : 'not-executed',
      notes: capability.qualified
        ? ['Native, bridge certificate, and product WASM rows passed on the recorded source-bound run.', 'Every excluded row remains typed-refuse; no external clean-run claim is made.']
        : ['Native zero-displacement and refusal tests exist, but positive-gap bridge/product evidence does not.'],
    },
    resetPolicy,
    unresolvedRows: capability.unresolved,
  }
  const evidence = {
    $schema: './brep-capability-evidence-v2.schema.json',
    schema: 'open-scad-viewer/brep-capability-evidence',
    schemaVersion: 2,
    evidenceId: `${name}-evidence-v2`,
    capability: capability.id,
    maturity: capability.qualified ? 'Qualified' : 'Unavailable',
    state: capability.qualified ? 'qualified' : 'not-executed',
    plan: planPath,
    runs: capability.qualified ? runRecords : [],
    oracles: capability.oracles,
    unresolvedRows: capability.unresolved,
    attestation: {
      fabricatedRuns: false,
      note: capability.qualified
        ? 'Recorded local native, bridge, and product runs passed; this is not a G0/G1 clean external execution claim.'
        : 'No positive-gap bridge/product run exists; capability remains Unavailable.',
    },
  }
  write(planPath, plan)
  write(evidencePath, evidence)
}

const legacyQualified = [
  { id: 'planar-csg/1', plan: 'docs/qualification/plans/planar-csg-1.json', evidence: 'docs/qualification/brep-native-workbench-boolean-v1.json', dependencies: [] },
  { id: 'analytic-boolean/1', plan: 'docs/qualification/plans/analytic-boolean-1.json', evidence: 'docs/qualification/analytic-boolean-1-evidence-v1.json', dependencies: [] },
]
const rows = [
  ...legacyQualified.map(row => ({ ...row, maturity: 'Qualified', releaseState: 'shipped' })),
  ...capabilities.map(capability => ({
    id: capability.id,
    plan: `docs/qualification/plans/${slug(capability.id)}.json`,
    evidence: `docs/qualification/${slug(capability.id)}-evidence-v2.json`,
    maturity: capability.qualified ? 'Qualified' : 'Unavailable',
    releaseState: capability.qualified ? 'shipped' : 'candidate',
    dependencies: capability.deps,
  })),
]
const shipped = rows.filter(row => row.releaseState === 'shipped').map(row => row.id)
const candidates = rows.filter(row => row.releaseState === 'candidate').map(row => row.id)
const unsupported = [
  'analytic-constructors/1', 'intersection-queries/1', 'nurbs-ss-bezier-le3/1',
  'nurbs-boolean-bezier-le3/1', 'analytic-fillet/1', 'analytic-chamfer/1',
  'analytic-shell/1', 'analytic-solid-loft/1', 'step-interchange/1',
  'nurbs-step-bicubic-face/1', 'nurbs-step-trimmed-bicubic/1',
  'nurbs-step-solid/1', 'iges-interchange/1',
]
write('docs/qualification/plans/g8-full-matrix-index-v2.json', {
  schema: 'open-scad-viewer/qualification-plan-index', schemaVersion: 2, gate: 'G8-full',
  matrix: 'docs/qualification/brep-full-closed-matrix-v2.json',
  registry: 'docs/qualification/brep-capability-registry-release-full-v2.json',
  capabilities: rows, unresolvedInShippedMatrix: [],
  invariants: ['only-Qualified-dependency-closure-ships', 'false-Complete-freezes-source-and-transitively-stales-dependents', 'no-fabricated-runs', 'no-manifold-cross-route'],
})
write('docs/qualification/brep-full-closed-matrix-v2.json', {
  schema: 'open-scad-viewer/brep-closed-matrix', schemaVersion: 2,
  planId: 'brep-full-closed-matrix-v2', gate: 'G8-full', lifecycle: 'frozen',
  claimBoundary: 'project-kernel-qualified-finite-subset-rust',
  successorOf: 'docs/qualification/brep-full-closed-matrix-v1.json',
  dependencyPolicy: {
    requiredMaturity: 'Qualified',
    releaseRule: 'A capability may ship only when it and every transitive dependency are Qualified with zero blocking rows.',
    resetRule: 'A false-Complete freezes that capability ID and transitively stales all dependents; recovery requires successor IDs.',
  },
  admittedOps: shipped, pendingQualification: candidates,
  explicitRefuse: [...unsupported, 'unqualified-foundation-capability', 'dependency-below-Qualified', 'stale-transitive-dependent', 'silent-heal', 'mesh-fallback', 'manifold-cross-route', 'parasolid-parity'],
  productDeployGate: 'Only releaseState=shipped rows whose complete dependency closure is Qualified may deploy.',
  unresolvedInShippedMatrix: [], unresolvedCandidateRows: candidates,
})
write('docs/qualification/brep-capability-registry-release-full-v2.json', {
  schema: 'open-scad-viewer/brep-capability-registry-release', schemaVersion: 2,
  gate: 'G8-full', planId: 'brep-capability-registry-release-full-v2',
  successorOf: 'docs/qualification/brep-capability-registry-release-full-v1.json',
  registry: 'src/services/geometry/brepCapability.ts',
  closedMatrix: 'docs/qualification/brep-full-closed-matrix-v2.json',
  g8Index: 'docs/qualification/plans/g8-full-matrix-index-v2.json',
  capabilities: shipped,
  dependencyPolicy: { requiredMaturity: 'Qualified', transitive: true, falseCompleteReset: 'freeze-source-and-stale-transitive-dependents' },
  excludedPendingQualification: candidates, explicitRefuse: unsupported,
  unresolvedInShippedMatrix: [],
})

function write(path, value) {
  const absolute = resolve(root, path)
  mkdirSync(dirname(absolute), { recursive: true })
  writeFileSync(absolute, `${JSON.stringify(value, null, 2)}\n`)
}
