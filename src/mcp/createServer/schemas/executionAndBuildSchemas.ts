import { z } from 'zod/v4'
import { LEGACY_MANIFOLD_EXECUTION } from '../../../core/geometryExecution'
import { PUBLIC_ERROR_CODES } from '../../publicError'
import { CURRENT_BREP_MANIFEST, CURRENT_MESH_MANIFEST } from '../../server/contracts'
import { qualitySchema, sha256Schema } from './commonSchemas'

const capabilityIdSchema = z
  .string()
  .max(128)
  .regex(/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)

export const executionCommonShape = {
  required_capabilities: z.array(capabilityIdSchema).max(32),
  purpose: z.enum(['preview', 'full', 'analysis', 'export']),
  quality: qualitySchema,
  automatic_fallback: z.literal(false),
}

export function meshExecutionSchemaForManifest(manifest: typeof CURRENT_MESH_MANIFEST) {
  return z.object({
    ...executionCommonShape,
    language_contract: z.literal('legacy/current'),
    engine_class: z.literal(manifest.engineClass),
    engine_key: z.literal(manifest.engineKey),
    kernel_fingerprint: z.literal(manifest.kernelFingerprint),
    semantic_program_version: z.literal(manifest.semanticProgramVersion),
    capability_manifest_version: z.literal(manifest.capabilityManifestVersion),
    manifest_digest: z.literal(manifest.manifestDigest),
    representation: z.literal('mesh'),
    evidence: z.enum(['planned', 'runtime']),
    effective_limits: z.object({
      sourceCharacters: z.literal(250_000),
      triangles: z.literal(750_000),
    }),
  })
}

const currentMeshExecutionSchema = meshExecutionSchemaForManifest(CURRENT_MESH_MANIFEST)

export const brepExecutionSchema = z.object({
  ...executionCommonShape,
  language_contract: z.literal('openscad-viewer/brep-1'),
  engine_class: z.literal('brep'),
  engine_key: z.literal(CURRENT_BREP_MANIFEST.engineKey),
  kernel_fingerprint: z.literal(CURRENT_BREP_MANIFEST.kernelFingerprint),
  semantic_program_version: z.literal(CURRENT_BREP_MANIFEST.semanticProgramVersion),
  capability_manifest_version: z.literal(CURRENT_BREP_MANIFEST.capabilityManifestVersion),
  manifest_digest: z.literal(CURRENT_BREP_MANIFEST.manifestDigest),
  representation: z.enum(['brep', 'mesh']),
  evidence: z.enum(['planned', 'runtime']),
  effective_limits: z.object({
    sourceCharacters: z.literal(250_000),
    triangles: z.literal(750_000),
  }),
})

const legacyBackfillCommonShape = {
  language_contract: z.literal('legacy/current'),
  required_capabilities: z.array(z.never()).length(0),
  engine_class: z.literal('mesh'),
  engine_key: z.literal(LEGACY_MANIFOLD_EXECUTION.engineKey),
  kernel_fingerprint: z.literal(LEGACY_MANIFOLD_EXECUTION.kernelFingerprint),
  semantic_program_version: z.literal(LEGACY_MANIFOLD_EXECUTION.semanticProgramVersion),
  capability_manifest_version: z.literal(LEGACY_MANIFOLD_EXECUTION.capabilityManifestVersion),
  manifest_digest: z.literal(LEGACY_MANIFOLD_EXECUTION.manifestDigest),
  representation: z.literal('mesh'),
  evidence: z.literal('legacy-backfill'),
  effective_limits: z.object({}).strict(),
  automatic_fallback: z.literal(false),
}

const legacyBackfillExecutionSchema = z.union([
  z
    .object({
      ...legacyBackfillCommonShape,
      purpose: z.literal('preview'),
      quality: z.literal('preview'),
    })
    .strict(),
  z
    .object({
      ...legacyBackfillCommonShape,
      purpose: z.literal('full'),
      quality: z.literal('full'),
    })
    .strict(),
])

export const executionSchema = z.union([
  z.union([currentMeshExecutionSchema, brepExecutionSchema]),
  legacyBackfillExecutionSchema,
])

export const runtimeExecutionSchema = z.union([
  currentMeshExecutionSchema.extend({ evidence: z.literal('runtime') }),
  brepExecutionSchema.extend({ evidence: z.literal('runtime') }),
])

const diagnosticCommonShape = {
  name: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.union([z.string(), z.number().finite(), z.boolean(), z.null()])).optional(),
  line: z.number().int().positive().optional(),
  column: z.number().int().positive().optional(),
}

const diagnosticSchema = z.union([
  z.object({
    ...diagnosticCommonShape,
    contractVersion: z.literal(1),
    code: z.enum(PUBLIC_ERROR_CODES),
    retryable: z.boolean(),
  }),
  z.object({
    ...diagnosticCommonShape,
    contractVersion: z.literal(0),
    evidence: z.literal('legacy-unattested'),
    code: z.string().optional(),
    retryable: z.boolean().optional(),
  }),
])

const buildCommonShape = {
  id: z.string(),
  model_id: z.string().nullable(),
  model_revision: z.number().int().nonnegative().nullable(),
  source_sha256: sha256Schema,
  source_attestation: z.enum(['model-revision', 'inline-snapshot', 'historical-unattested']),
  quality: qualitySchema,
  duration_ms: z.number().nonnegative(),
  warnings: z.array(z.string()),
  created_at: z.string(),
  resource_uri: z.string(),
}

export const sourceAttestationSchema = buildCommonShape.source_attestation

export const buildMetricsSchema = z.object({
  mesh_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number(),
  surface_area: z.number().nonnegative(),
  reduced: z.boolean(),
})

export const buildSchema = z.union([
  z.object({
    ...buildCommonShape,
    execution: z.union([runtimeExecutionSchema, legacyBackfillExecutionSchema]),
    status: z.literal('succeeded'),
    metrics: buildMetricsSchema,
    error: z.null(),
  }),
  z.object({
    ...buildCommonShape,
    execution: executionSchema,
    status: z.literal('failed'),
    metrics: z.null(),
    error: diagnosticSchema,
  }),
  z.object({
    ...buildCommonShape,
    execution: executionSchema,
    status: z.literal('cancelled'),
    metrics: z.null(),
    error: diagnosticSchema.nullable(),
  }),
])

export const artifactSchema = z.object({
  id: z.string(),
  resource_uri: z.string(),
  file_name: z.string(),
  format: z.enum(['stl', 'obj', 'step']),
  mime_type: z.string(),
  sha256: z.string(),
  byte_length: z.number().int().nonnegative(),
})

const engineManifestCommonShape = {
  display_name: z.string(),
  permanent: z.literal(true),
  availability: z.enum(['available', 'unavailable']),
  unavailable_reason: z.string().nullable(),
  manifest_digest: sha256Schema,
  manifest_resource_uri: z.string(),
  capabilities: z.array(capabilityIdSchema),
  planned_capabilities: z.array(capabilityIdSchema),
  qualities: z.tuple([z.literal('preview'), z.literal('full')]),
  export_formats: z.array(z.string()),
  planned_export_formats: z.array(z.string()),
  qualification: z.object({
    status: z.enum(['qualified', 'baseline-pending', 'not-qualified']),
    record_id: z.string().nullable(),
    corpus_version: z.string().nullable(),
    target: z.string(),
  }),
  dependency: z.object({
    package_name: z.string().nullable(),
    version: z.string().nullable(),
    license_expression: z.string().nullable(),
    sbom_ref: z.string().nullable(),
    sbom_sha256: sha256Schema.nullable(),
    lockfile_sha256: sha256Schema.nullable(),
  }),
  rollback_compatibility: z.object({
    disable_engine_capability: z.literal(true),
    source_contract_preserved: z.literal(true),
    cross_engine_fallback: z.literal(false),
    minimum_catalog_schema: z.number().int().positive(),
  }),
  automatic_fallback: z.literal(false),
}

export const engineManifestSchema = z.union([
  z.object({
    ...engineManifestCommonShape,
    engine_class: z.literal('mesh'),
    maturity: z.literal('production'),
    engine_key: z.literal(CURRENT_MESH_MANIFEST.engineKey),
    kernel_fingerprint: z.literal(CURRENT_MESH_MANIFEST.kernelFingerprint),
    semantic_program_version: z.literal(CURRENT_MESH_MANIFEST.semanticProgramVersion),
    capability_manifest_version: z.literal(CURRENT_MESH_MANIFEST.capabilityManifestVersion),
    manifest_digest: z.literal(CURRENT_MESH_MANIFEST.manifestDigest),
    manifest_resource_uri: z.literal(
      `openscad://engines/mesh/capabilities/${CURRENT_MESH_MANIFEST.capabilityManifestVersion}`,
    ),
    language_contracts: z.tuple([z.literal('legacy/current')]),
    input_contract: z.literal('legacy-source-direct'),
    representations: z.tuple([z.literal('mesh')]),
    planned_representations: z.array(z.never()).length(0),
    limits: z.object({
      sourceCharacters: z.literal(250_000),
      triangles: z.literal(750_000),
    }),
    isolation: z.literal('in-process-serialized'),
    deployment: z.literal('node-mcp'),
  }),
  z.object({
    ...engineManifestCommonShape,
    engine_class: z.literal('brep'),
    availability: z.literal('available'),
    maturity: z.literal('production'),
    engine_key: z.literal(CURRENT_BREP_MANIFEST.engineKey),
    kernel_fingerprint: z.literal(CURRENT_BREP_MANIFEST.kernelFingerprint),
    semantic_program_version: z.literal(CURRENT_BREP_MANIFEST.semanticProgramVersion),
    capability_manifest_version: z.literal(CURRENT_BREP_MANIFEST.capabilityManifestVersion),
    manifest_digest: z.literal(CURRENT_BREP_MANIFEST.manifestDigest),
    manifest_resource_uri: z.literal(
      `openscad://engines/brep/capabilities/${CURRENT_BREP_MANIFEST.capabilityManifestVersion}`,
    ),
    language_contracts: z.tuple([z.literal('openscad-viewer/brep-1')]),
    input_contract: z.literal('semantic-program-required'),
    capabilities: z.array(capabilityIdSchema).min(1),
    representations: z.tuple([z.literal('brep'), z.literal('mesh')]),
    planned_representations: z.array(z.never()).length(0),
    export_formats: z.tuple([z.literal('stl'), z.literal('obj'), z.literal('step')]),
    limits: z.object({
      sourceCharacters: z.literal(250_000),
      triangles: z.literal(750_000),
    }),
    isolation: z.literal('in-process-serialized'),
    deployment: z.literal('node-mcp'),
  }),
])

export const engineRegistrySchema = z.object({
  contract_version: z.literal(1),
  source_directed_routing: z.literal(true),
  automatic_fallback: z.literal(false),
  routes: z.tuple([
    z
      .object({
        language_contract: z.literal('legacy/current'),
        engine_class: z.literal('mesh'),
        fallback: z.literal('never'),
      })
      .strict(),
    z
      .object({
        language_contract: z.literal('openscad-viewer/brep-1'),
        engine_class: z.literal('brep'),
        fallback: z.literal('never'),
      })
      .strict(),
  ]),
  engines: z.tuple([
    engineManifestSchema.options[0],
    // The archived manifest is static; runtime readiness may refuse this provider.
    engineManifestSchema.options[1].extend({ availability: z.enum(['available', 'unavailable']) }),
  ]),
})

export const catalogStatsSchema = z.object({
  models: z.number().int().nonnegative(),
  revisions: z.number().int().nonnegative(),
  builds: z.number().int().nonnegative(),
  artifacts: z.number().int().nonnegative(),
  stored_source_bytes: z.number().int().nonnegative(),
  stored_artifact_bytes: z.number().int().nonnegative(),
  builds_by_status: z.object({
    succeeded: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
    cancelled: z.number().int().nonnegative(),
  }),
  limits: z.object({
    models: z.number().int().positive(),
    revisions: z.number().int().positive(),
    revisions_per_model: z.number().int().positive(),
    source_bytes: z.number().int().positive(),
    builds: z.number().int().positive(),
    artifacts: z.number().int().positive(),
    artifact_bytes: z.number().int().positive(),
  }),
})

export const toolErrorSchema = z.object({
  code: z.enum(PUBLIC_ERROR_CODES),
  message: z.string(),
  retryable: z.boolean(),
  next_action: z.string().optional(),
  correlation_id: z.string().optional(),
  details: z.record(z.string(), z.union([z.string(), z.number().finite(), z.boolean(), z.null()])).optional(),
  line: z.number().int().positive().optional(),
  column: z.number().int().positive().optional(),
})

export const toolErrorResultSchema = z.object({
  error: toolErrorSchema,
  execution: executionSchema.optional(),
  build: buildSchema.optional(),
})
