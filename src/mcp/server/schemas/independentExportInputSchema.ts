// schemas: independentExportInputSchema.
import { z } from 'zod/v4'

import { LEGACY_MANIFOLD_EXECUTION } from '../../../core/geometryExecution'

import { OPENSCAD_2021_01_CONTRACT } from '../../../core/openScad2021Contract'

import {
  OPENSCAD_PROJECT_MAX_BLOB_BYTES,
  OPENSCAD_PROJECT_MAX_FILES,
  OPENSCAD_PROJECT_MAX_PATH_BYTES,
  OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
  OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
  normalizeOpenScadProjectPath,
} from '../../../services/openScadProject'

import {
  DEFAULT_MAX_ARTIFACT_BYTES,
  MAX_ARTIFACT_BYTES,
} from '../../geometryService'

import {
  MAX_MODEL_ID_LENGTH,
  MAX_MODEL_SOURCE_LENGTH,
  isWellFormedUnicode,
} from '../../modelStore'

import { PUBLIC_ERROR_CODES } from '../../publicError'

import {
  OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS,
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES,
  OFFICIAL_OPENSCAD_MAX_DEFINES,
  OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
  OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
  OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  normalizeOfficialOpenScadProjectPath,
  officialOpenScadProjectPathsConflict,
} from '../../officialOpenScadRuntimeProtocol'

import {
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
} from '../../officialOpenScadRuntimePatch'

import {
  CURRENT_BREP_MANIFEST,
  CURRENT_MESH_MANIFEST,
  INDEPENDENT_ENGINE_ID,
  OFFICIAL_LANGUAGE_RESOURCE_URI,
} from '.././contracts'
import { independentProjectInputShape, independentExportFormatSchema, independentExportFileNameSchema, validateIndependentProjectInput, persistedRuntimeExecutionSchema, legacyBackfillExecutionSchema, executionSchema, runtimeExecutionSchema, boundsSchema, pointSchema, topologySchema } from './meshExecutionSchemaForManifest'
import { sha256Schema, qualitySchema, idSchema, officialLogsSchema, sourceSchema } from './idSchema'

export const independentExportInputSchema = z.object({
  ...independentProjectInputShape,
  format: independentExportFormatSchema,
  file_name: independentExportFileNameSchema.optional(),
  max_bytes: z.number().int().positive().max(MAX_ARTIFACT_BYTES).default(DEFAULT_MAX_ARTIFACT_BYTES),
  quality: z.literal('full').default('full'),
  time: z.number().finite().min(0).max(1).default(0),
}).strict().superRefine(validateIndependentProjectInput)

export const independentEngineAttestationSchema = z.object({
  id: z.literal(INDEPENDENT_ENGINE_ID),
  stage: z.literal('development'),
  language_contract: z.literal(OPENSCAD_2021_01_CONTRACT.id),
  upstream_runtime_used: z.literal(false),
  production_authoritative: z.literal(false),
  complete_language_claim: z.literal(false),
  stable_function_inventory: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.functions.length),
  stable_module_inventory: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.modules.length),
}).strict()

export const independentMetricsSchema = z.object({
  mesh_count: z.number().int().nonnegative(),
  vertex_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number().finite().nonnegative(),
  surface_area: z.number().finite().nonnegative(),
}).strict()

export const independentCheckSchema = z.object({
  engine: independentEngineAttestationSchema,
  source_sha256: sha256Schema,
  quality: qualitySchema,
  duration_ms: z.number().finite().nonnegative(),
  ...independentMetricsSchema.shape,
  warnings: z.array(z.string()),
}).strict()

export const independentExportSchema = z.object({
  engine: independentEngineAttestationSchema,
  source_sha256: sha256Schema,
  quality: z.literal('full'),
  format: independentExportFormatSchema,
  duration_ms: z.number().finite().nonnegative(),
  metrics: independentMetricsSchema,
  warnings: z.array(z.string()),
}).strict()

export const independentArtifactSchema = z.object({
  id: sha256Schema,
  resource_uri: z.string(),
  file_name: independentExportFileNameSchema,
  format: independentExportFormatSchema,
  mime_type: z.enum(['model/stl', 'model/obj']),
  sha256: sha256Schema,
  byte_length: z.number().int().nonnegative(),
}).strict()

export const modelMetadataSchema = z.object({
  id: z.string(),
  name: z.string(),
  revision: z.number().int().nonnegative(),
  created_at: z.string(),
  updated_at: z.string(),
  resource_uri: z.string(),
})

export const diagnosticCommonShape = {
  name: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.union([
    z.string(), z.number().finite(), z.boolean(), z.null(),
  ])).optional(),
  line: z.number().int().positive().optional(),
  column: z.number().int().positive().optional(),
}

export const diagnosticSchema = z.union([
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

export const buildCommonShape = {
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
    execution: z.union([persistedRuntimeExecutionSchema, legacyBackfillExecutionSchema]),
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

export const comparisonSideSchema = z.object({
  source_sha256: sha256Schema,
  model_id: idSchema.nullable(),
  model_revision: z.number().int().nonnegative().nullable(),
  execution: runtimeExecutionSchema,
  mesh_count: z.number().nonnegative().int(),
  triangle_count: z.number().nonnegative().int(),
  volume: z.number().nonnegative(),
  surface_area: z.number().nonnegative(),
  bounds: boundsSchema,
  dimensions: pointSchema,
  topology: topologySchema,
  warnings: z.array(z.string()),
})

export const comparisonDeltaSchema = z.object({
  mesh_count: z.number(),
  triangle_count: z.number(),
  volume: z.number(),
  surface_area: z.number(),
  dimensions: pointSchema,
  topology: z.object({
    boundary: z.number(),
    crease: z.number(),
    non_manifold: z.number(),
    degenerate: z.number(),
  }),
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

export const publicErrorSchema = z.object({
  code: z.enum(PUBLIC_ERROR_CODES),
  message: z.string(),
  retryable: z.boolean(),
  next_action: z.string().optional(),
  correlation_id: z.string().optional(),
  details: z.record(z.string(), z.union([
    z.string(),
    z.number().finite(),
    z.boolean(),
    z.null(),
  ])).optional(),
  line: z.number().int().positive().optional(),
  column: z.number().int().positive().optional(),
})

export const toolErrorResultSchema = z.object({
  error: publicErrorSchema,
  execution: executionSchema.optional(),
  build: buildSchema.optional(),
})


export const comparisonSuccessSchema = z.object({
  quality: qualitySchema,
  comparison_mode: z.enum(['same_engine_metrics', 'cross_engine_metrics_only']),
  geometric_equivalence: z.literal(false),
  left: comparisonSideSchema,
  right: comparisonSideSchema,
  delta: comparisonDeltaSchema,
})


export const comparisonErrorResultSchema = toolErrorResultSchema.extend({
  comparison_failure: z.object({
    failed_side: z.enum(['left', 'right']),
    completed_left: comparisonSideSchema.optional(),
  }),
})


export function toolOutputSchema<T extends z.ZodType>(success: T) {
  return z.union([success, toolErrorResultSchema])
}


export const officialToolErrorResultSchema = toolErrorResultSchema.extend({
  official_logs: officialLogsSchema.optional(),
  duration_ms: z.number().finite().nonnegative().optional(),
}).strict()


export function officialToolOutputSchema<T extends z.ZodType>(success: T) {
  return z.union([success, officialToolErrorResultSchema])
}


export const sourceSelectorFields = {
  source: sourceSchema.describe('Inline OpenSCAD source; mutually exclusive with model_id.').optional(),
  model_id: idSchema.describe('Saved DuckDB model id; mutually exclusive with source.').optional(),
  revision: z.number().int().nonnegative()
    .describe('Immutable saved-model revision; valid only with model_id. Defaults to the current revision.')
    .optional(),
}


export const sourceReferenceSchema = z.union([
  z.object({
    source: sourceSelectorFields.source.unwrap(),
    model_id: z.never().optional(),
    revision: z.never().optional(),
  }).strict(),
  z.object({
    source: z.never().optional(),
    model_id: sourceSelectorFields.model_id.unwrap(),
    revision: sourceSelectorFields.revision,
  }).strict(),
])


export function selectorSchema<T extends z.ZodRawShape>(extra: T) {
  return z.union([
    z.object({
      ...extra,
      source: sourceSelectorFields.source.unwrap(),
      model_id: z.never().optional(),
      revision: z.never().optional(),
    }).strict(),
    z.object({
      ...extra,
      source: z.never().optional(),
      model_id: sourceSelectorFields.model_id.unwrap(),
      revision: sourceSelectorFields.revision,
    }).strict(),
  ])
}
