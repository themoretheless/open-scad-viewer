import { createHash } from 'node:crypto'
import {
  McpServer,
  ResourceNotFoundError,
  ResourceTemplate,
  SUPPORTED_PROTOCOL_VERSIONS,
  type RequestId,
} from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type {
  GeometryBuildPurpose,
  GeometryEngineRegistrySnapshot,
  GeometryExecutionDescriptor,
} from '../core/geometryExecution'
import { GEOMETRY_MANIFEST_ARCHIVE } from '../core/geometryExecution'
import { EXAMPLE_CATALOG, EXAMPLES } from '../data/examples'
import {
  attachGeometryExecutionToError,
  geometryExecutionForError,
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  GeometryLanguageContractError,
} from '../services/geometryBuildEngine'
import type { CustomizerValue } from '../services/scadCustomizer'
import {
  DEFAULT_MAX_IN_FLIGHT_REQUESTS,
  DEFAULT_MAX_PENDING_CONTROL_MESSAGES,
  DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES,
  MAX_MCP_SUBSCRIPTIONS,
  MAX_MCP_STRING_REQUEST_ID_LENGTH,
} from './boundedTransport'
import {
  DEFAULT_MAX_ARTIFACT_BYTES,
  GeometryBusyError,
  HeadlessGeometryService,
  MAX_ARTIFACT_BYTES,
  MAX_PENDING_GEOMETRY_JOBS,
  type GeometryAnalysis,
  type McpGeometryService,
} from './geometryService'
import {
  assertGeometryManifestArchive,
  canonicalJson,
  engineManifestUri,
  immutableEngineManifestToWire,
  staticEngineManifestToWire,
} from './engineManifest'
import {
  MAX_MODEL_ID_LENGTH,
  MAX_MODEL_NAME_LENGTH,
  MAX_MODEL_SOURCE_LENGTH,
  ModelRevisionConflictError,
  isWellFormedUnicode,
  validateModelId,
  type ModelStore,
  type StoredBuild,
  type StoredModel,
  type StoredModelRevision,
} from './modelStore'
import {
  buildDiagnostic,
  isAbortError,
  ModelNotFoundError,
  PUBLIC_ERROR_CODES,
  publicToolError,
} from './publicError'

export interface CreateOpenScadMcpServerOptions {
  store: ModelStore
  geometry?: McpGeometryService
  /** Releases transport admission only after an SDK request handler settles. */
  onRequestSettled?: (requestId: RequestId) => void
  /** Mirrors the SDK handler AbortSignal after protocol validation. */
  onRequestCancelled?: (requestId: RequestId) => void
}

const SERVER_INSTRUCTIONS = `Use openscad_check for read-only validation and fast iteration; it does not write build history.
Use openscad_analyze when a durable build record is useful, and openscad_export only when an STL or OBJ artifact is required.
Use openscad_compare for bounded metric/topology deltas; it is not an exact geometric boolean diff.
Geometry is routed by the source's leading @language contract: legacy/current uses Manifold and openscad-viewer/brep-1 uses the Rust B-rep/NURBS engine. Callers cannot override that route. Both engine classes are permanent, and an unavailable engine returns a typed error without cross-engine fallback.
For saved models, list or get the model first and pass expected_revision when saving changes. Use openscad_customize_model for an atomic Customizer update. Pass revision to check, analyze, customize, or export an immutable historical snapshot.
Customizer operations only replace declared top-level parameters. The compiler intentionally rejects unsupported OpenSCAD features such as include, use, import, text, surface, Minkowski, and user-defined functions.
Preview geometry can be reduced; use full quality for authoritative measurements and exports. DuckDB is independent from the browser IndexedDB workspace, so no browser draft is modified implicitly.`

const CACHE_FIVE_MINUTES = 5 * 60 * 1_000
const CACHE_ONE_DAY = 24 * 60 * 60 * 1_000

const idSchema = z.string().min(1).max(MAX_MODEL_ID_LENGTH)
  .regex(/^[A-Za-z0-9][A-Za-z0-9._:-]*$/)
const sourceSchema = z.string().max(MAX_MODEL_SOURCE_LENGTH)
  .refine(isWellFormedUnicode, 'Source must contain well-formed Unicode')
const qualitySchema = z.enum(['preview', 'full'])
const sha256Schema = z.string().regex(/^[a-f0-9]{64}$/)
const capabilityIdSchema = z.string().max(128)
  .regex(/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
const executionCommonShape = {
  required_capabilities: z.array(capabilityIdSchema).max(32),
  purpose: z.enum(['preview', 'full', 'analysis', 'export']),
  quality: qualitySchema,
  automatic_fallback: z.literal(false),
}
const manifoldExecutionSchema = z.object({
    ...executionCommonShape,
    language_contract: z.literal('legacy/current'),
    engine_class: z.literal('manifold'),
    engine_key: z.literal('manifold-wasm-v1'),
    kernel_fingerprint: z.literal('manifold-wasm-v1'),
    semantic_program_version: z.literal('legacy-direct-evaluator-v1'),
    capability_manifest_version: z.literal('manifold-node-v1'),
    manifest_digest: z.literal('117bc5f8792f5d9d3b70ed80f66419070289c1ea4757f9fe1a7f268ab2b58645'),
    representation: z.literal('mesh'),
    evidence: z.enum(['planned', 'runtime']),
    effective_limits: z.object({
      sourceCharacters: z.literal(250_000),
      triangles: z.literal(750_000),
    }),
  })
const brepExecutionSchema = z.object({
    ...executionCommonShape,
    language_contract: z.literal('openscad-viewer/brep-1'),
    engine_class: z.literal('brep'),
    engine_key: z.literal('rust-brep-reserved-v1'),
    kernel_fingerprint: z.literal('not-deployed'),
    semantic_program_version: z.literal('semantic-program-contract-v1'),
    capability_manifest_version: z.literal('brep-contract-v1'),
    manifest_digest: z.literal('c4eefdbf1ff0add1e1705413d6ab766774487db0cb5a1d1434ab6c25f4ef1964'),
    representation: z.enum(['brep', 'mesh']),
    evidence: z.literal('planned'),
    effective_limits: z.object({ sourceCharacters: z.literal(250_000) }),
  })
const currentExecutionSchema = z.union([manifoldExecutionSchema, brepExecutionSchema])
const legacyBackfillCommonShape = {
  language_contract: z.literal('legacy/current'),
  required_capabilities: z.array(z.never()).length(0),
  engine_class: z.literal('manifold'),
  engine_key: z.literal('manifold-wasm-v1'),
  kernel_fingerprint: z.literal('manifold-wasm-v1'),
  semantic_program_version: z.literal('legacy-direct-evaluator-v1'),
  capability_manifest_version: z.literal('manifold-node-v1'),
  manifest_digest: z.literal('117bc5f8792f5d9d3b70ed80f66419070289c1ea4757f9fe1a7f268ab2b58645'),
  representation: z.literal('mesh'),
  evidence: z.literal('legacy-backfill'),
  effective_limits: z.object({}).strict(),
  automatic_fallback: z.literal(false),
}
const legacyBackfillExecutionSchema = z.union([
  z.object({
    ...legacyBackfillCommonShape,
    purpose: z.literal('preview'),
    quality: z.literal('preview'),
  }).strict(),
  z.object({
    ...legacyBackfillCommonShape,
    purpose: z.literal('full'),
    quality: z.literal('full'),
  }).strict(),
])
const executionSchema = z.union([currentExecutionSchema, legacyBackfillExecutionSchema])
const runtimeExecutionSchema = manifoldExecutionSchema.extend({ evidence: z.literal('runtime') })
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
const manifoldEngineManifestSchema = z.object({
  ...engineManifestCommonShape,
  engine_class: z.literal('manifold'),
  maturity: z.literal('production'),
  engine_key: z.literal('manifold-wasm-v1'),
  kernel_fingerprint: z.literal('manifold-wasm-v1'),
  semantic_program_version: z.literal('legacy-direct-evaluator-v1'),
  capability_manifest_version: z.literal('manifold-node-v1'),
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
})
const brepEngineManifestSchema = z.object({
  ...engineManifestCommonShape,
  engine_class: z.literal('brep'),
  availability: z.literal('unavailable'),
  maturity: z.literal('contract'),
  engine_key: z.literal('rust-brep-reserved-v1'),
  kernel_fingerprint: z.literal('not-deployed'),
  semantic_program_version: z.literal('semantic-program-contract-v1'),
  capability_manifest_version: z.literal('brep-contract-v1'),
  language_contracts: z.tuple([z.literal('openscad-viewer/brep-1')]),
  input_contract: z.literal('semantic-program-required'),
  capabilities: z.array(z.never()).length(0),
  representations: z.array(z.never()).length(0),
  planned_representations: z.tuple([z.literal('brep'), z.literal('mesh')]),
  export_formats: z.array(z.never()).length(0),
  limits: z.object({ sourceCharacters: z.literal(250_000) }),
  isolation: z.literal('not-deployed'),
  deployment: z.literal('not-deployed'),
})
const engineRegistrySchema = z.object({
  contract_version: z.literal(1),
  source_directed_routing: z.literal(true),
  automatic_fallback: z.literal(false),
  routes: z.tuple([
    z.object({
      language_contract: z.literal('legacy/current'),
      engine_class: z.literal('manifold'),
      fallback: z.literal('never'),
    }).strict(),
    z.object({
      language_contract: z.literal('openscad-viewer/brep-1'),
      engine_class: z.literal('brep'),
      fallback: z.literal('never'),
    }).strict(),
  ]),
  engines: z.tuple([manifoldEngineManifestSchema, brepEngineManifestSchema]),
})
const wireStringSchema = z.string().max(64 * 1024)
  .refine(isWellFormedUnicode, 'String must contain well-formed Unicode')
const promptGoalSchema = z.string().max(2_048)
  .refine(isWellFormedUnicode, 'Goal must contain well-formed Unicode')
const scalarSchema = z.union([z.number().finite(), z.boolean(), wireStringSchema])
const pointSchema = z.array(z.number().finite()).length(3)
const topologySchema = z.object({
  boundary: z.number().int().nonnegative(),
  crease: z.number().int().nonnegative(),
  non_manifold: z.number().int().nonnegative(),
  degenerate: z.number().int().nonnegative(),
})
const boundsSchema = z.object({ min: pointSchema, max: pointSchema }).nullable()
const analysisSchema = z.object({
  execution: runtimeExecutionSchema,
  quality: qualitySchema,
  reduced: z.boolean(),
  duration_ms: z.number().nonnegative(),
  mesh_count: z.number().int().nonnegative(),
  vertex_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number(),
  surface_area: z.number().nonnegative(),
  bounds: boundsSchema,
  topology: topologySchema,
  warnings: z.array(z.string()),
  parameters: z.array(z.object({
    name: z.string(),
    label: z.string(),
    value: scalarSchema,
    min: z.number().nullable(),
    max: z.number().nullable(),
    step: z.number().nullable(),
    options: z.array(scalarSchema).nullable(),
  })),
  parameters_truncated: z.boolean(),
  objects: z.array(z.object({
    index: z.number().int().nonnegative(),
    entity_id: z.string().nullable(),
    vertices: z.number().int().nonnegative(),
    triangles: z.number().int().nonnegative(),
    bounds: boundsSchema,
    dimensions: pointSchema,
    center: pointSchema.nullable(),
    color: z.array(z.number().finite()).length(4),
    topology: topologySchema,
    sources: z.array(z.object({
      label: z.string(),
      start: z.number().int().nonnegative(),
      end: z.number().int().nonnegative(),
      operation_id: z.string().nullable(),
      instance_id: z.string().nullable(),
      triangles: z.number().int().nonnegative(),
    })),
    sources_truncated: z.boolean(),
  })),
  objects_truncated: z.boolean(),
  details_truncated: z.boolean(),
})
const modelMetadataSchema = z.object({
  id: z.string(),
  name: z.string(),
  revision: z.number().int().nonnegative(),
  created_at: z.string(),
  updated_at: z.string(),
  resource_uri: z.string(),
})
const diagnosticCommonShape = {
  name: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.union([
    z.string(), z.number().finite(), z.boolean(), z.null(),
  ])).optional(),
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
const buildMetricsSchema = z.object({
  mesh_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number(),
  surface_area: z.number().nonnegative(),
  reduced: z.boolean(),
})
const buildSchema = z.union([
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
const comparisonSideSchema = z.object({
  source_sha256: sha256Schema,
  model_id: idSchema.nullable(),
  model_revision: z.number().int().nonnegative().nullable(),
  execution: runtimeExecutionSchema,
  mesh_count: z.number().int().nonnegative(),
  triangle_count: z.number().int().nonnegative(),
  volume: z.number().nonnegative(),
  surface_area: z.number().nonnegative(),
  bounds: boundsSchema,
  dimensions: pointSchema,
  topology: topologySchema,
  warnings: z.array(z.string()),
})
const comparisonDeltaSchema = z.object({
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
const catalogStatsSchema = z.object({
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
const publicErrorSchema = z.object({
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
const toolErrorResultSchema = z.object({
  error: publicErrorSchema,
  execution: executionSchema.optional(),
  build: buildSchema.optional(),
})

const comparisonSuccessSchema = z.object({
  quality: qualitySchema,
  comparison_mode: z.enum(['same_engine_metrics', 'cross_engine_metrics_only']),
  geometric_equivalence: z.literal(false),
  left: comparisonSideSchema,
  right: comparisonSideSchema,
  delta: comparisonDeltaSchema,
})

const comparisonErrorResultSchema = toolErrorResultSchema.extend({
  comparison_failure: z.object({
    failed_side: z.enum(['left', 'right']),
    completed_left: comparisonSideSchema.optional(),
  }),
})

function toolOutputSchema<T extends z.ZodType>(success: T) {
  return z.union([success, toolErrorResultSchema])
}

const sourceSelectorFields = {
  source: sourceSchema.describe('Inline OpenSCAD source; mutually exclusive with model_id.').optional(),
  model_id: idSchema.describe('Saved DuckDB model id; mutually exclusive with source.').optional(),
  revision: z.number().int().nonnegative()
    .describe('Immutable saved-model revision; valid only with model_id. Defaults to the current revision.')
    .optional(),
}

const sourceReferenceSchema = z.union([
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

function selectorSchema<T extends z.ZodRawShape>(extra: T) {
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

function sha256(data: string | Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

function truncateWellFormed(value: string, maxLength: number): string {
  if (value.length <= maxLength) return value
  let end = maxLength
  const last = value.charCodeAt(end - 1)
  if (last >= 0xD800 && last <= 0xDBFF) end--
  return value.slice(0, end)
}

function modelUri(id: string): string {
  return `openscad://models/${encodeURIComponent(id)}`
}

function modelRevisionUri(id: string, revision: number): string {
  return `${modelUri(id)}/revisions/${revision}`
}

function buildUri(id: string): string {
  return `openscad://builds/${encodeURIComponent(id)}`
}

function artifactUri(id: string): string {
  return `openscad://artifacts/${encodeURIComponent(id)}`
}

function resourceVariable(uri: URL, value: string | string[] | undefined): string {
  if (typeof value !== 'string') throw new ResourceNotFoundError(uri.href)
  try {
    return decodeURIComponent(value)
  } catch {
    throw new ResourceNotFoundError(uri.href)
  }
}

function resourceId(uri: URL, value: string | string[] | undefined): string {
  const id = resourceVariable(uri, value)
  try { validateModelId(id) } catch { throw new ResourceNotFoundError(uri.href) }
  return id
}

function modelToWire(model: StoredModel) {
  return {
    id: model.id,
    name: model.name,
    revision: model.revision,
    created_at: model.createdAt,
    updated_at: model.updatedAt,
    resource_uri: modelUri(model.id),
  }
}

function modelRevisionToWire(model: StoredModelRevision) {
  return {
    id: model.id,
    name: model.name,
    revision: model.revision,
    created_at: model.createdAt,
    updated_at: model.createdAt,
    resource_uri: modelRevisionUri(model.id, model.revision),
  }
}

function executionToWire(execution: GeometryExecutionDescriptor) {
  return {
    language_contract: execution.languageContract,
    required_capabilities: execution.requiredCapabilities,
    engine_class: execution.engineClass,
    engine_key: execution.engineKey,
    kernel_fingerprint: execution.kernelFingerprint,
    semantic_program_version: execution.semanticProgramVersion,
    capability_manifest_version: execution.capabilityManifestVersion,
    manifest_digest: execution.manifestDigest,
    purpose: execution.purpose,
    quality: execution.quality,
    representation: execution.representation,
    evidence: execution.evidence,
    effective_limits: execution.effectiveLimits,
    automatic_fallback: execution.automaticFallback,
  }
}

function engineRegistryToWire(registry: GeometryEngineRegistrySnapshot) {
  return {
    contract_version: registry.contractVersion,
    source_directed_routing: registry.sourceDirectedRouting,
    automatic_fallback: registry.automaticFallback,
    routes: registry.routes.map(route => ({
      language_contract: route.languageContract,
      engine_class: route.engineClass,
      fallback: route.fallback,
    })),
    engines: registry.engines.map(engine => ({
      ...staticEngineManifestToWire(engine),
      availability: engine.availability,
      unavailable_reason: engine.unavailableReason,
    })),
  }
}

function analysisToWire(analysis: GeometryAnalysis) {
  return {
    execution: executionToWire(analysis.execution),
    quality: analysis.quality,
    reduced: analysis.reduced,
    duration_ms: analysis.durationMs,
    mesh_count: analysis.meshCount,
    vertex_count: analysis.vertexCount,
    triangle_count: analysis.triangleCount,
    volume: analysis.volume,
    surface_area: analysis.surfaceArea,
    bounds: analysis.bounds,
    topology: {
      boundary: analysis.topology.boundary,
      crease: analysis.topology.crease,
      non_manifold: analysis.topology.nonManifold,
      degenerate: analysis.topology.degenerate,
    },
    warnings: analysis.warnings,
    parameters: analysis.parameters,
    parameters_truncated: analysis.parametersTruncated,
    objects: analysis.objects.map(object => ({
      index: object.index,
      entity_id: object.entityId,
      vertices: object.vertices,
      triangles: object.triangles,
      bounds: object.bounds,
      dimensions: object.dimensions,
      center: object.center,
      color: object.color,
      topology: {
        boundary: object.topology.boundary,
        crease: object.topology.crease,
        non_manifold: object.topology.nonManifold,
        degenerate: object.topology.degenerate,
      },
      sources: object.sources.map(source => ({
        label: source.label,
        start: source.start,
        end: source.end,
        operation_id: source.operationId,
        instance_id: source.instanceId,
        triangles: source.triangles,
      })),
      sources_truncated: object.sourcesTruncated,
    })),
    objects_truncated: analysis.objectsTruncated,
    details_truncated: analysis.detailsTruncated,
  }
}

function buildToWire(build: StoredBuild) {
  return {
    id: build.id,
    model_id: build.modelId,
    model_revision: build.modelRevision,
    source_sha256: build.sourceSha256,
    source_attestation: build.sourceAttestation,
    execution: executionToWire(build.execution),
    quality: build.quality,
    status: build.status,
    duration_ms: build.durationMs,
    warnings: build.warnings,
    metrics: build.metrics ? {
      mesh_count: build.metrics.meshCount,
      triangle_count: build.metrics.triangleCount,
      volume: build.metrics.volume,
      surface_area: build.metrics.surfaceArea,
      reduced: build.metrics.reduced,
    } : null,
    error: build.error,
    created_at: build.createdAt,
    resource_uri: buildUri(build.id),
  }
}

function textResult<T extends object>(structuredContent: T) {
  const serialized = JSON.stringify(structuredContent)
  return {
    content: [{ type: 'text' as const, text: serialized }],
    structuredContent: structuredContent as Record<string, unknown>,
  }
}

function executionForFailure(error: unknown): GeometryExecutionDescriptor | undefined {
  return geometryExecutionForError(error)
    ?? (error instanceof GeometryEngineUnavailableError
    || error instanceof GeometryCapabilityUnavailableError
    ? error.execution
    : undefined)
}

function errorResult(error: unknown, extra: Record<string, unknown> = {}) {
  const problem = publicToolError(error)
  if (problem.internal) {
    console.error(`MCP tool failure (${problem.error.correlation_id}):`, error)
  }
  const execution = executionForFailure(error)
  const structured = {
    ...extra,
    ...(execution && extra.execution === undefined
      ? { execution: executionToWire(execution) }
      : {}),
    error: problem.error,
  }
  return {
    content: [{ type: 'text' as const, text: JSON.stringify(structured, null, 2) }],
    structuredContent: structured,
    isError: true as const,
  }
}

interface SelectedModel {
  id: string
  name: string
  source: string
  revision: number
}

async function resolveSource(
  store: ModelStore,
  input: { source?: string; model_id?: string; revision?: number },
): Promise<{ source: string; model: SelectedModel | null }> {
  if (input.source !== undefined) return { source: input.source, model: null }
  const model = input.revision === undefined
    ? await store.getModel(input.model_id!)
    : await store.getModelRevision(input.model_id!, input.revision)
  if (!model) throw new ModelNotFoundError(input.model_id!, input.revision)
  return { source: model.source, model }
}

function analysisDimensions(analysis: GeometryAnalysis): [number, number, number] {
  if (!analysis.bounds) return [0, 0, 0]
  return analysis.bounds.max.map((value, index) => (
    value - analysis.bounds!.min[index]
  )) as [number, number, number]
}

function comparisonSide(
  selected: Awaited<ReturnType<typeof resolveSource>>,
  analysis: GeometryAnalysis,
) {
  return {
    source_sha256: sha256(selected.source),
    model_id: selected.model?.id ?? null,
    model_revision: selected.model?.revision ?? null,
    execution: executionToWire(analysis.execution),
    mesh_count: analysis.meshCount,
    triangle_count: analysis.triangleCount,
    volume: analysis.volume,
    surface_area: analysis.surfaceArea,
    bounds: analysis.bounds,
    dimensions: analysisDimensions(analysis),
    topology: {
      boundary: analysis.topology.boundary,
      crease: analysis.topology.crease,
      non_manifold: analysis.topology.nonManifold,
      degenerate: analysis.topology.degenerate,
    },
    warnings: analysis.warnings,
  }
}

function comparisonDelta(
  left: ReturnType<typeof comparisonSide>,
  right: ReturnType<typeof comparisonSide>,
) {
  return {
    mesh_count: right.mesh_count - left.mesh_count,
    triangle_count: right.triangle_count - left.triangle_count,
    volume: right.volume - left.volume,
    surface_area: right.surface_area - left.surface_area,
    dimensions: right.dimensions.map((value, index) => (
      value - left.dimensions[index]
    )) as [number, number, number],
    topology: {
      boundary: right.topology.boundary - left.topology.boundary,
      crease: right.topology.crease - left.topology.crease,
      non_manifold: right.topology.non_manifold - left.topology.non_manifold,
      degenerate: right.topology.degenerate - left.topology.degenerate,
    },
  }
}

function sameExecutionContext(
  left: GeometryExecutionDescriptor,
  right: GeometryExecutionDescriptor,
): boolean {
  const stableLimits = (limits: Record<string, number>) => JSON.stringify(
    Object.entries(limits).sort(([leftKey], [rightKey]) => leftKey.localeCompare(rightKey)),
  )
  return left.languageContract === right.languageContract
    && left.engineClass === right.engineClass
    && left.engineKey === right.engineKey
    && left.kernelFingerprint === right.kernelFingerprint
    && left.semanticProgramVersion === right.semanticProgramVersion
    && left.capabilityManifestVersion === right.capabilityManifestVersion
    && left.manifestDigest === right.manifestDigest
    && left.requiredCapabilities.join('\0') === right.requiredCapabilities.join('\0')
    && left.purpose === right.purpose
    && left.quality === right.quality
    && left.representation === right.representation
    && left.automaticFallback === right.automaticFallback
    && stableLimits(left.effectiveLimits) === stableLimits(right.effectiveLimits)
}

function safeFileName(name: string, format: 'stl' | 'obj'): string {
  const extension = `.${format}`
  const trimmed = name.trim().replace(/[/\\\0]/g, '_') || `model${extension}`
  const base = trimmed.toLowerCase().endsWith(extension) ? trimmed.slice(0, -extension.length) : trimmed
  return `${truncateWellFormed(base, 255 - extension.length)}${extension}`
}

async function recordFailure(
  store: ModelStore,
  geometry: McpGeometryService,
  input: {
    model: Pick<SelectedModel, 'id' | 'revision'> | null
    source: string
    quality: 'preview' | 'full'
    purpose: GeometryBuildPurpose
    durationMs: number
    error: unknown
    execution?: GeometryExecutionDescriptor
  },
): Promise<StoredBuild | null> {
  try {
    if (input.error instanceof GeometryLanguageContractError) return null
    const execution = input.execution
      ?? executionForFailure(input.error)
      ?? geometry.planSource(input.source, input.quality, input.purpose)
    return await store.recordBuild({
      modelId: input.model?.id,
      modelRevision: input.model?.revision,
      source: input.source,
      sourceSha256: sha256(input.source),
      execution,
      quality: input.quality,
      durationMs: input.durationMs,
      warnings: [],
      status: persistedFailureStatus(input.error),
      error: buildDiagnostic(input.error),
    })
  } catch (storeError) {
    console.error('Could not record failed MCP build:', storeError)
    return null
  }
}

/** Persisted causality depends on the settled error, never a later signal state. */
export function persistedFailureStatus(error: unknown): 'cancelled' | 'failed' {
  return isAbortError(error) ? 'cancelled' : 'failed'
}

export function createOpenScadMcpServer(options: CreateOpenScadMcpServerOptions): McpServer {
  assertGeometryManifestArchive()
  const { store } = options
  const geometry = options.geometry ?? new HeadlessGeometryService()
  const server = new McpServer(
    {
      name: 'open-scad-viewer',
      version: '0.1.0',
      description: 'Bounded local OpenSCAD analysis, customization, revision storage, and STL/OBJ export.',
    },
    {
      supportedProtocolVersions: ['2026-07-28', ...SUPPORTED_PROTOCOL_VERSIONS],
      instructions: SERVER_INSTRUCTIONS,
      cacheHints: {
        'server/discover': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'tools/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'prompts/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
        'resources/templates/list': { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
      },
    },
  )
  const notifyResourceListChanged = () => {
    void server.server.sendResourceListChanged().catch(error => {
      console.error('Could not send MCP resource-list notification:', error)
    })
  }

  server.registerTool('openscad_save_model', {
    title: 'Save OpenSCAD model',
    description: 'Create or update a versioned OpenSCAD model in the local DuckDB catalog.',
    inputSchema: z.object({
      id: idSchema.optional(),
      name: z.string().min(1).max(MAX_MODEL_NAME_LENGTH)
        .refine(isWellFormedUnicode, 'Name must contain well-formed Unicode')
        .refine(name => name.trim().length > 0, 'Name must contain a non-whitespace character'),
      source: sourceSchema,
      expected_revision: z.number().int().nonnegative().optional(),
    }).strict(),
    outputSchema: toolOutputSchema(z.object({ model: modelMetadataSchema })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async input => {
    try {
      geometry.planSource(input.source, 'preview', 'preview')
      const model = await store.saveModel({
        id: input.id,
        name: input.name,
        source: input.source,
        expectedRevision: input.expected_revision,
      })
      notifyResourceListChanged()
      return textResult({ model: modelToWire(model) })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_get_model', {
    title: 'Get OpenSCAD model',
    description: 'Read a saved OpenSCAD model and its current revision from DuckDB.',
    inputSchema: z.object({
      id: idSchema,
      revision: z.number().int().nonnegative().optional(),
    }).strict(),
    outputSchema: toolOutputSchema(z.object({ model: modelMetadataSchema.extend({ source: z.string() }) })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ id, revision }) => {
    try {
      const model = revision === undefined
        ? await store.getModel(id)
        : await store.getModelRevision(id, revision)
      if (!model) return errorResult(new ModelNotFoundError(id, revision))
      const metadata = revision === undefined
        ? modelToWire(model as StoredModel)
        : modelRevisionToWire(model as StoredModelRevision)
      return textResult({ model: { ...metadata, source: model.source } })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_list_models', {
    title: 'List OpenSCAD models',
    description: 'List saved OpenSCAD models from the local DuckDB catalog.',
    inputSchema: z.object({ limit: z.number().int().min(1).max(500).default(100) }),
    outputSchema: toolOutputSchema(z.object({
      models: z.array(modelMetadataSchema.extend({ source_length: z.number().int().nonnegative() })),
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ limit }) => {
    try {
      const models = await store.listModels(limit)
      return textResult({
        models: models.map(model => ({
          ...modelToWire({ ...model, source: '' }),
          source_length: model.sourceLength,
        })),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_catalog_stats', {
    title: 'OpenSCAD catalog statistics',
    description: 'Return bounded DuckDB catalog usage, build outcomes, and quota limits without exposing SQL.',
    inputSchema: z.object({}),
    outputSchema: toolOutputSchema(z.object({ stats: catalogStatsSchema })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      const stats = await store.getCatalogStats()
      return textResult({
        stats: {
          models: stats.modelCount,
          revisions: stats.revisionCount,
          builds: stats.buildCount,
          artifacts: stats.artifactCount,
          stored_source_bytes: stats.storedSourceBytes,
          stored_artifact_bytes: stats.storedArtifactBytes,
          builds_by_status: stats.buildsByStatus,
          limits: {
            models: stats.limits.models,
            revisions: stats.limits.revisions,
            revisions_per_model: stats.limits.revisionsPerModel,
            source_bytes: stats.limits.sourceBytes,
            builds: stats.limits.builds,
            artifacts: stats.limits.artifacts,
            artifact_bytes: stats.limits.artifactBytes,
          },
        },
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_list_engines', {
    title: 'List geometry engines',
    description: 'Return both permanent geometry engine classes, their source-language routes, runtime availability, manifests, and the no-fallback policy.',
    inputSchema: z.object({}),
    outputSchema: toolOutputSchema(z.object({ geometry_engines: engineRegistrySchema })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => textResult({
    geometry_engines: engineRegistryToWire(await geometry.capabilities()),
  }))

  server.registerTool('openscad_list_model_revisions', {
    title: 'List OpenSCAD model revisions',
    description: 'List immutable source snapshots for one saved DuckDB model.',
    inputSchema: z.object({
      id: idSchema,
      limit: z.number().int().min(1).max(500).default(100),
    }),
    outputSchema: toolOutputSchema(z.object({
      revisions: z.array(z.object({
        id: z.string(),
        name: z.string(),
        revision: z.number().int().nonnegative(),
        created_at: z.string(),
        source_length: z.number().int().nonnegative(),
        resource_uri: z.string(),
      })),
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ id, limit }) => {
    try {
      const revisions = await store.listModelRevisions(id, limit)
      return textResult({
        revisions: revisions.map(revision => ({
          id: revision.id,
          name: revision.name,
          revision: revision.revision,
          created_at: revision.createdAt,
          source_length: revision.sourceLength,
          resource_uri: modelRevisionUri(revision.id, revision.revision),
        })),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_check', {
    title: 'Check OpenSCAD geometry',
    description: 'Compile inline source or an exact saved-model revision without writing build history. Use preview for fast iteration and full for authoritative metrics.',
    inputSchema: selectorSchema({ quality: qualitySchema.default('preview') }),
    outputSchema: toolOutputSchema(z.object({
      source_sha256: sha256Schema,
      model_id: idSchema.nullable(),
      model_revision: z.number().int().nonnegative().nullable(),
      analysis: analysisSchema,
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const selected = await resolveSource(store, input)
      const analysis = await geometry.analyze(selected.source, input.quality, context.mcpReq.signal)
      return textResult({
        source_sha256: sha256(selected.source),
        model_id: selected.model?.id ?? null,
        model_revision: selected.model?.revision ?? null,
        analysis: analysisToWire(analysis),
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_compare', {
    title: 'Compare OpenSCAD geometry',
    description: 'Compile two inline or saved-revision sources without persistence and return bounded metric, dimension, and topology deltas. This is not an exact geometric boolean diff.',
    inputSchema: z.object({
      left: sourceReferenceSchema,
      right: sourceReferenceSchema,
      quality: qualitySchema.default('full'),
    }).strict(),
    outputSchema: z.union([comparisonSuccessSchema, comparisonErrorResultSchema]),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async ({ left, right, quality }, context) => {
    let leftSource: Awaited<ReturnType<typeof resolveSource>>
    let rightSource: Awaited<ReturnType<typeof resolveSource>>
    let leftAnalysis: GeometryAnalysis
    try {
      leftSource = await resolveSource(store, left)
    } catch (error) {
      return errorResult(error, { comparison_failure: { failed_side: 'left' } })
    }
    try {
      rightSource = await resolveSource(store, right)
    } catch (error) {
      return errorResult(error, { comparison_failure: { failed_side: 'right' } })
    }
    try {
      leftAnalysis = await geometry.analyze(leftSource.source, quality, context.mcpReq.signal)
    } catch (error) {
      return errorResult(error, { comparison_failure: { failed_side: 'left' } })
    }
    let rightAnalysis: GeometryAnalysis
    try {
      rightAnalysis = leftSource.source === rightSource.source
        ? leftAnalysis
        : await geometry.analyze(rightSource.source, quality, context.mcpReq.signal)
    } catch (error) {
      return errorResult(error, {
        comparison_failure: {
          failed_side: 'right',
          completed_left: comparisonSide(leftSource, leftAnalysis),
        },
      })
    }
    try {
      const leftSummary = comparisonSide(leftSource, leftAnalysis)
      const rightSummary = comparisonSide(rightSource, rightAnalysis)
      return textResult({
        quality,
        comparison_mode: sameExecutionContext(leftAnalysis.execution, rightAnalysis.execution)
          ? 'same_engine_metrics' as const
          : 'cross_engine_metrics_only' as const,
        geometric_equivalence: false as const,
        left: leftSummary,
        right: rightSummary,
        delta: comparisonDelta(leftSummary, rightSummary),
      })
    } catch (error) {
      return errorResult(error, {
        comparison_failure: {
          failed_side: 'right',
          completed_left: comparisonSide(leftSource, leftAnalysis),
        },
      })
    }
  })

  server.registerTool('openscad_analyze', {
    title: 'Analyze OpenSCAD geometry',
    description: 'Compile OpenSCAD source or a saved model and return bounded geometry, topology, provenance, and Customizer facts. The build is recorded in DuckDB.',
    inputSchema: selectorSchema({ quality: qualitySchema.default('full') }),
    outputSchema: toolOutputSchema(z.object({ build: buildSchema, analysis: analysisSchema })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async (input, context) => {
    const startedAt = performance.now()
    let completedExecution: GeometryExecutionDescriptor | undefined
    let selected: Awaited<ReturnType<typeof resolveSource>>
    try {
      selected = await resolveSource(store, input)
    } catch (error) {
      return errorResult(error)
    }
    try {
      const analysis = await geometry.analyze(selected.source, input.quality, context.mcpReq.signal)
      completedExecution = analysis.execution
      let build: StoredBuild
      try {
        build = await store.recordBuild({
          modelId: selected.model?.id,
          modelRevision: selected.model?.revision,
          source: selected.source,
          sourceSha256: sha256(selected.source),
          execution: analysis.execution,
          quality: input.quality,
          durationMs: analysis.durationMs,
          warnings: analysis.warnings,
          status: 'succeeded',
          metrics: {
            meshCount: analysis.meshCount,
            triangleCount: analysis.triangleCount,
            volume: analysis.volume,
            surfaceArea: analysis.surfaceArea,
            reduced: analysis.reduced,
          },
        })
      } catch (error) {
        attachGeometryExecutionToError(error, analysis.execution)
        throw error
      }
      notifyResourceListChanged()
      return textResult({ build: buildToWire(build), analysis: analysisToWire(analysis) })
    } catch (error) {
      if (error instanceof GeometryBusyError) return errorResult(error)
      const build = await recordFailure(store, geometry, {
        ...selected,
        quality: input.quality,
        purpose: 'analysis',
        durationMs: performance.now() - startedAt,
        error,
        execution: completedExecution,
      })
      if (build) notifyResourceListChanged()
      return errorResult(error, build ? { build: buildToWire(build) } : {})
    }
  })

  server.registerTool('openscad_customize', {
    title: 'Customize OpenSCAD source',
    description: 'Apply validated values to top-level OpenSCAD Customizer parameters and return updated source without saving it.',
    inputSchema: selectorSchema({ values: z.record(z.string(), scalarSchema) }),
    outputSchema: toolOutputSchema(z.object({
      source: z.string(),
      applied: z.array(z.string()),
      parameters: analysisSchema.shape.parameters,
      parameters_truncated: z.boolean(),
    })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async input => {
    try {
      const selected = await resolveSource(store, input)
      const result = geometry.customize(selected.source, input.values as Record<string, CustomizerValue>)
      return textResult({
        source: result.source,
        applied: result.applied,
        parameters: result.parameters,
        parameters_truncated: result.parametersTruncated,
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_customize_model', {
    title: 'Customize and save an OpenSCAD model',
    description: 'Atomically apply validated top-level Customizer values to the current saved model and create a revision guarded by expected_revision.',
    inputSchema: z.object({
      model_id: idSchema,
      expected_revision: z.number().int().nonnegative(),
      values: z.record(z.string(), scalarSchema),
    }).strict(),
    outputSchema: toolOutputSchema(z.object({
      model: modelMetadataSchema,
      applied: z.array(z.string()),
      parameters: analysisSchema.shape.parameters,
      parameters_truncated: z.boolean(),
    })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async input => {
    try {
      const model = await store.getModel(input.model_id)
      if (!model) throw new ModelNotFoundError(input.model_id)
      if (model.revision !== input.expected_revision) {
        throw new ModelRevisionConflictError(model.id, input.expected_revision, model.revision)
      }
      const result = geometry.customize(model.source, input.values as Record<string, CustomizerValue>)
      const saved = await store.saveModelSource({
        id: model.id,
        source: result.source,
        expectedRevision: input.expected_revision,
      })
      notifyResourceListChanged()
      return textResult({
        model: modelToWire(saved),
        applied: result.applied,
        parameters: result.parameters,
        parameters_truncated: result.parametersTruncated,
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_export', {
    title: 'Export OpenSCAD geometry',
    description: 'Compile a full-quality model, store a bounded STL or OBJ artifact in DuckDB, and return an MCP resource link.',
    inputSchema: selectorSchema({
      format: z.enum(['stl', 'obj']),
      file_name: z.string().min(1).max(255)
        .refine(isWellFormedUnicode, 'File name must contain well-formed Unicode').optional(),
      max_bytes: z.number().int().positive().max(MAX_ARTIFACT_BYTES).default(DEFAULT_MAX_ARTIFACT_BYTES),
    }),
    outputSchema: toolOutputSchema(z.object({
      build: buildSchema,
      analysis: analysisSchema,
      artifact: z.object({
        id: z.string(),
        resource_uri: z.string(),
        file_name: z.string(),
        format: z.enum(['stl', 'obj']),
        mime_type: z.string(),
        sha256: z.string(),
        byte_length: z.number().int().nonnegative(),
      }),
    })),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
  }, async (input, context) => {
    const startedAt = performance.now()
    let completedBuild: StoredBuild | null = null
    let completedExecution: GeometryExecutionDescriptor | undefined
    let selected: Awaited<ReturnType<typeof resolveSource>>
    try {
      selected = await resolveSource(store, input)
    } catch (error) {
      return errorResult(error)
    }
    try {
      const exported = await geometry.export(selected.source, input.format, {
        maxBytes: input.max_bytes,
        signal: context.mcpReq.signal,
        name: input.file_name ?? selected.model?.name,
      })
      const analysis = exported.analysis
      completedExecution = analysis.execution
      let build: StoredBuild
      try {
        build = await store.recordBuild({
          modelId: selected.model?.id,
          modelRevision: selected.model?.revision,
          source: selected.source,
          sourceSha256: sha256(selected.source),
          execution: analysis.execution,
          quality: 'full',
          durationMs: analysis.durationMs,
          warnings: analysis.warnings,
          status: 'succeeded',
          metrics: {
            meshCount: analysis.meshCount,
            triangleCount: analysis.triangleCount,
            volume: analysis.volume,
            surfaceArea: analysis.surfaceArea,
            reduced: analysis.reduced,
          },
        })
      } catch (error) {
        attachGeometryExecutionToError(error, analysis.execution)
        throw error
      }
      completedBuild = build
      const fileName = safeFileName(input.file_name ?? selected.model?.name ?? 'model', input.format)
      const artifact = await store.storeArtifact({
        buildId: build.id,
        modelId: selected.model?.id,
        format: input.format,
        fileName,
        mimeType: exported.mimeType,
        sha256: sha256(exported.data),
        data: exported.data,
      })
      notifyResourceListChanged()
      const structured = {
        build: buildToWire(build),
        analysis: analysisToWire(analysis),
        artifact: {
          id: artifact.id,
          resource_uri: artifactUri(artifact.id),
          file_name: artifact.fileName,
          format: artifact.format,
          mime_type: artifact.mimeType,
          sha256: artifact.sha256,
          byte_length: artifact.byteLength,
        },
      }
      const result = textResult(structured)
      return {
        ...result,
        content: [
          result.content[0],
          {
            type: 'resource_link' as const,
            uri: artifactUri(artifact.id),
            name: artifact.fileName,
            mimeType: artifact.mimeType,
            description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes)`,
          },
        ],
      }
    } catch (error) {
      if (completedBuild) {
        notifyResourceListChanged()
        return errorResult(error, { build: buildToWire(completedBuild) })
      }
      if (error instanceof GeometryBusyError) return errorResult(error)
      const build = await recordFailure(store, geometry, {
        ...selected,
        quality: 'full',
        purpose: 'export',
        durationMs: performance.now() - startedAt,
        error,
        execution: completedExecution,
      })
      if (build) notifyResourceListChanged()
      return errorResult(error, build ? { build: buildToWire(build) } : {})
    }
  })

  server.registerTool('openscad_build_history', {
    title: 'OpenSCAD build history',
    description: 'List recent MCP build results recorded in DuckDB, optionally for one saved model.',
    inputSchema: z.object({
      model_id: idSchema.optional(),
      limit: z.number().int().min(1).max(100).default(100),
    }),
    outputSchema: toolOutputSchema(z.object({ builds: z.array(buildSchema) })),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async input => {
    try {
      const builds = await store.listBuilds({ modelId: input.model_id, limit: input.limit })
      return textResult({ builds: builds.map(buildToWire) })
    } catch (error) {
      return errorResult(error)
    }
  })

  const promptRevisionSchema = z.string().max(16).regex(/^\d+$/)
    .refine(value => Number.isSafeInteger(Number(value)), 'Revision must be a safe integer')
    .describe('Optional immutable model revision written as a decimal integer.')

  server.registerPrompt('openscad_review_model', {
    title: 'Review a saved OpenSCAD model',
    description: 'Guide a reproducible geometry and print-readiness review without modifying the catalog.',
    argsSchema: z.object({
      model_id: idSchema,
      revision: promptRevisionSchema.optional(),
      goal: promptGoalSchema.optional(),
    }),
  }, ({ model_id, revision, goal }) => {
    const selector = revision === undefined
      ? { model_id }
      : { model_id, revision: Number(revision) }
    return {
      messages: [{
        role: 'user' as const,
        content: {
          type: 'text' as const,
          text: `Review the saved OpenSCAD model selected by ${JSON.stringify(selector)}.${goal ? ` Goal: ${goal}` : ''}
Call openscad_check with full quality. Report dimensions, volume, surface area, mesh and triangle counts, boundary/non-manifold/degenerate topology, warnings, and relevant source provenance. Distinguish proven facts from printability assumptions. Do not save or export unless I explicitly ask.`,
        },
      }],
    }
  })

  server.registerPrompt('openscad_customize_workflow', {
    title: 'Customize a saved OpenSCAD model',
    description: 'Guide safe Customizer exploration with an explicit revision check before any save.',
    argsSchema: z.object({
      model_id: idSchema,
      revision: promptRevisionSchema.optional(),
      goal: promptGoalSchema,
    }),
  }, ({ model_id, revision, goal }) => {
    const selector = revision === undefined
      ? { model_id }
      : { model_id, revision: Number(revision) }
    return {
      messages: [{
        role: 'user' as const,
        content: {
          type: 'text' as const,
          text: `Prepare a safe customization of ${JSON.stringify(selector)} for this goal: ${goal}
First call openscad_check in preview quality and inspect its declared top-level Customizer parameters. Explore only valid declared values with openscad_customize and check the returned source. Do not save automatically. If I approve a save, read the current model first and call openscad_customize_model with its revision as expected_revision so concurrent changes cannot be overwritten.`,
        },
      }],
    }
  })

  server.registerResource('capabilities', 'openscad://capabilities', {
    title: 'OpenSCAD MCP capabilities',
    description: 'Machine-readable engine registry, source routes, compiler subset, safety limits, protocol support, and persistence boundaries.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => {
    const geometryEngines = engineRegistryToWire(await geometry.capabilities())
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({
        server: { name: 'open-scad-viewer', version: '0.1.0' },
        protocol_versions: ['2026-07-28', ...SUPPORTED_PROTOCOL_VERSIONS],
        geometry_engines: geometryEngines,
        parity_resource_uri: 'openscad://parity',
        compiler: {
          language_scope: 'strict OpenSCAD subset',
          supported: [
            'cube/sphere/cylinder/polyhedron',
            'square/circle/polygon',
            'union/difference/intersection/hull',
            'transforms/color',
            'modules/children/for/if/let',
            'linear_extrude/rotate_extrude/projection/offset',
            'top-level Customizer parameters',
          ],
          unsupported: [
            'include/use/import',
            'text/surface/Minkowski',
            'user-defined functions',
          ],
        },
        limits: {
          source_characters: MAX_MODEL_SOURCE_LENGTH,
          artifact_bytes: MAX_ARTIFACT_BYTES,
          pending_geometry_jobs: MAX_PENDING_GEOMETRY_JOBS,
          stdio_in_flight_requests: DEFAULT_MAX_IN_FLIGHT_REQUESTS,
          stdio_pending_output: DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES,
          stdio_pending_overload_replies: DEFAULT_MAX_PENDING_CONTROL_MESSAGES,
          stdio_subscriptions: MAX_MCP_SUBSCRIPTIONS,
          stdio_string_request_id_characters: MAX_MCP_STRING_REQUEST_ID_LENGTH,
        },
        persistence: {
          catalog: 'DuckDB',
          arbitrary_sql: false,
          browser_indexeddb_synchronized: false,
        },
        }, null, 2),
      }],
    }
  })

  const engineManifestTemplate = new ResourceTemplate(
    'openscad://engines/{engine_class}/capabilities/{manifest_version}',
    {
      list: async () => ({
        resources: Object.values(GEOMETRY_MANIFEST_ARCHIVE).map(staticEngineManifestToWire).map(engine => ({
          uri: engine.manifest_resource_uri,
          name: `${engine.display_name} capability manifest`,
          description: `${engine.maturity} immutable manifest; permanent engine class`,
          mimeType: 'application/json',
        })),
      }),
    },
  )
  server.registerResource('geometry-engine-manifest', engineManifestTemplate, {
    title: 'Immutable geometry engine capability manifest',
    description: 'Versioned capabilities, deployment maturity, fingerprint, and isolation facts for one permanent engine class.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const engineClass = resourceVariable(uri, variables.engine_class)
    const manifestVersion = resourceVariable(uri, variables.manifest_version)
    if (engineClass !== 'manifold' && engineClass !== 'brep') {
      throw new ResourceNotFoundError(uri.href)
    }
    const manifest = immutableEngineManifestToWire(
      engineClass,
      manifestVersion,
    )
    if (!manifest) throw new ResourceNotFoundError(uri.href)
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify(manifest, null, 2),
      }],
    }
  })

  server.registerResource('geometry-parity', 'openscad://parity', {
    title: 'Geometry engine browser-to-MCP parity status',
    description: 'Honest qualification state by engine; cross-engine metric comparison is never proof of geometric equivalence.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => {
    const registry = await geometry.capabilities()
    const policyPayload = {
      source_directed_routing: registry.sourceDirectedRouting,
      automatic_fallback: registry.automaticFallback,
      routes: registry.routes.map(route => ({
        language_contract: route.languageContract,
        engine_class: route.engineClass,
        fallback: route.fallback,
      })),
      manifests: registry.engines.map(engine => ({
        engine_class: engine.engineClass,
        capability_manifest_version: engine.capabilityManifestVersion,
        manifest_digest: engine.manifestDigest,
        kernel_fingerprint: engine.kernelFingerprint,
        limits: engine.limits,
      })),
      comparison: {
        mode: 'metrics-and-topology-only',
        cross_engine_geometric_equivalence: false,
        numeric_tolerances: null,
        comparator_qualification: 'pending',
      },
    }
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({
        contract_version: 1,
        definition: 'Same source revision, language contract, engine class and fingerprint, policy, limits, and declared tolerances across browser and MCP.',
        cross_engine_geometric_equivalence: false,
        policy: {
          id: 'engine-routing-contract-v1',
          digest_algorithm: 'sha256',
          canonicalization: 'recursive-codepoint-key-sort-v1',
          payload: policyPayload,
          sha256: sha256(canonicalJson(policyPayload)),
        },
        corpus: {
          id: 'browser-mcp-engine-parity',
          version: null,
          sha256: null,
          result_sha256: null,
          qualified_at: null,
        },
        engines: registry.engines.map(engine => ({
          engine_class: engine.engineClass,
          capability_manifest_version: engine.capabilityManifestVersion,
          manifest_digest: engine.manifestDigest,
          kernel_fingerprint: engine.kernelFingerprint,
          runtime_availability: engine.availability,
          browser_target: engine.deployment === 'not-deployed' ? 'not-deployed' : 'wasm-worker',
          mcp_target: engine.deployment === 'not-deployed' ? 'not-deployed' : 'node-wasm-in-process',
          browser_mcp_status: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : engine.qualification.status === 'qualified'
              ? 'qualified'
              : 'qualification-pending',
          mcp_isolation: engine.isolation,
          qualification: {
            status: engine.qualification.status,
            record_id: engine.qualification.recordId,
            corpus_version: engine.qualification.corpusVersion,
            target: engine.qualification.target,
          },
          result_sha256: null,
        })),
        }, null, 2),
      }],
    }
  })

  const modelTemplate = new ResourceTemplate('openscad://models/{id}', {
    list: async () => ({
      resources: (await store.listModels(500)).map(model => ({
        uri: modelUri(model.id),
        name: model.name,
        description: `OpenSCAD source, revision ${model.revision}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { id: async value => (await store.listModels(500)).map(model => model.id).filter(id => id.startsWith(value)) },
  })
  server.registerResource('saved-model', modelTemplate, {
    title: 'Saved OpenSCAD model',
    description: 'Versioned OpenSCAD source stored in DuckDB.',
    mimeType: 'text/x-openscad',
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const model = await store.getModel(id)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })

  const modelRevisionTemplate = new ResourceTemplate('openscad://models/{id}/revisions/{revision}', {
    list: undefined,
  })
  server.registerResource('saved-model-revision', modelRevisionTemplate, {
    title: 'Saved OpenSCAD model revision',
    description: 'An immutable OpenSCAD source snapshot stored in DuckDB.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const revisionValue = resourceVariable(uri, variables.revision)
    if (!/^\d+$/.test(revisionValue)) throw new ResourceNotFoundError(uri.href)
    const revision = Number(revisionValue)
    if (!Number.isSafeInteger(revision)) throw new ResourceNotFoundError(uri.href)
    const model = await store.getModelRevision(id, revision)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })

  const buildTemplate = new ResourceTemplate('openscad://builds/{id}', {
    list: async () => ({
      resources: (await store.listBuilds({ limit: 500 })).map(build => ({
        uri: buildUri(build.id),
        name: `Build ${build.id}`,
        description: `${build.status} ${build.quality} build`,
        mimeType: 'application/json',
      })),
    }),
  })
  server.registerResource('build-result', buildTemplate, {
    title: 'OpenSCAD build result',
    description: 'A compact build result recorded in DuckDB.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const build = await store.getBuild(id)
    if (!build) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(buildToWire(build), null, 2) }] }
  })

  const artifactTemplate = new ResourceTemplate('openscad://artifacts/{id}', {
    list: async () => ({
      resources: (await store.listArtifacts(100)).map(artifact => ({
        uri: artifactUri(artifact.id),
        name: artifact.fileName,
        description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes)`,
        mimeType: artifact.mimeType,
      })),
    }),
  })
  server.registerResource('export-artifact', artifactTemplate, {
    title: 'OpenSCAD export artifact',
    description: 'A bounded STL or OBJ artifact stored in DuckDB.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const artifact = await store.getArtifact(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return artifact.format === 'obj'
      ? { contents: [{ uri: uri.href, mimeType: artifact.mimeType, text: new TextDecoder().decode(artifact.data) }] }
      : { contents: [{ uri: uri.href, mimeType: artifact.mimeType, blob: Buffer.from(artifact.data).toString('base64') }] }
  })

  const exampleNames = EXAMPLE_CATALOG.map(example => example.id).sort()
  const examplesById = new Map(EXAMPLE_CATALOG.map(example => [example.id, example]))
  const exampleTemplate = new ResourceTemplate('openscad://examples/{name}', {
    list: async () => ({
      resources: exampleNames.map(name => ({
        uri: `openscad://examples/${name}`,
        name: `${name}.scad`,
        description: examplesById.get(name)?.description.en ?? `Bundled OpenSCAD example: ${name}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { name: value => exampleNames.filter(name => name.startsWith(value)) },
  })
  server.registerResource('example', exampleTemplate, {
    title: 'Bundled OpenSCAD example',
    description: 'An example from the browser workspace.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const name = resourceVariable(uri, variables.name)
    const source = EXAMPLES[name]
    if (source === undefined) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: source }] }
  })

  if (options.onRequestSettled) {
    type DispatchHandler = (...args: unknown[]) => unknown
    const protocolServer = server.server as unknown as {
      setRequestHandler: (...args: unknown[]) => void
      _requestHandlers: Map<string, DispatchHandler>
    }
    const wrappedHandlers = new WeakSet<DispatchHandler>()
    const wrapDispatchHandler = (method: string) => {
      const handler = protocolServer._requestHandlers.get(method)
      if (!handler || wrappedHandlers.has(handler)) return
      const wrapped: DispatchHandler = async (...args: unknown[]) => {
        const context = args.at(-1) as {
          mcpReq?: { id?: RequestId; signal?: AbortSignal }
        } | undefined
        const requestId = context?.mcpReq?.id
        const signal = context?.mcpReq?.signal
        if (requestId !== undefined && signal) {
          const cancelled = () => options.onRequestCancelled?.(requestId)
          if (signal.aborted) cancelled()
          else signal.addEventListener('abort', cancelled, { once: true })
        }
        try {
          return await handler(...args)
        } finally {
          if (requestId !== undefined) options.onRequestSettled?.(requestId)
        }
      }
      wrappedHandlers.add(wrapped)
      protocolServer._requestHandlers.set(method, wrapped)
    }

    // MCP SDK 2.0 has no public outer-dispatch middleware. Wrap finalized
    // registry entries so lifecycle covers SDK validation and application
    // callbacks. serveStdio installs a fresh modern discover handler after the
    // factory returns, so future registrations must pass through the same seam.
    const originalSetRequestHandler = protocolServer.setRequestHandler.bind(protocolServer)
    protocolServer.setRequestHandler = (...registration: unknown[]) => {
      originalSetRequestHandler(...registration)
      if (typeof registration[0] === 'string') wrapDispatchHandler(registration[0])
    }
    for (const method of protocolServer._requestHandlers.keys()) wrapDispatchHandler(method)
  }

  return server
}
