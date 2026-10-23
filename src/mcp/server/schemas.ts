import { z } from 'zod/v4'
import { LEGACY_MANIFOLD_EXECUTION } from '../../core/geometryExecution'
import { OPENSCAD_2021_01_CONTRACT } from '../../core/openScad2021Contract'
import {
  OPENSCAD_PROJECT_MAX_BLOB_BYTES,
  OPENSCAD_PROJECT_MAX_FILES,
  OPENSCAD_PROJECT_MAX_PATH_BYTES,
  OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
  OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
  normalizeOpenScadProjectPath,
} from '../../services/openScadProject'
import {
  DEFAULT_MAX_ARTIFACT_BYTES,
  MAX_ARTIFACT_BYTES,
} from '../geometryService'
import {
  MAX_MODEL_ID_LENGTH,
  MAX_MODEL_SOURCE_LENGTH,
  isWellFormedUnicode,
} from '../modelStore'
import { PUBLIC_ERROR_CODES } from '../publicError'
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
} from '../officialOpenScadRuntimeProtocol'
import {
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
} from '../officialOpenScadRuntimePatch'
import {
  CURRENT_BREP_MANIFEST,
  CURRENT_MESH_MANIFEST,
  INDEPENDENT_ENGINE_ID,
  OFFICIAL_LANGUAGE_RESOURCE_URI,
} from './contracts'

export const idSchema = z.string().min(1).max(MAX_MODEL_ID_LENGTH)
  .regex(/^[A-Za-z0-9][A-Za-z0-9._:-]*$/)
export const sourceSchema = z.string().max(MAX_MODEL_SOURCE_LENGTH)
  .refine(isWellFormedUnicode, 'Source must contain well-formed Unicode')
export const qualitySchema = z.enum(['preview', 'full'])
export const sha256Schema = z.string().regex(/^[a-f0-9]{64}$/)
export const officialFormatSchema = z.enum(OFFICIAL_OPENSCAD_EXPORT_FORMATS)
export const officialExperimentalFeatureSchema = z.enum(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)
const utf8Encoder = new TextEncoder()
export const officialSourceSchema = z.string().max(OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES)
  .refine(isWellFormedUnicode, 'Source must contain well-formed Unicode')
  .refine(
    value => utf8Encoder.encode(value).byteLength <= OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
    `Source must not exceed ${OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES} UTF-8 bytes`,
  )
export const officialProjectPathSchema = z.string().min(1).max(1_024)
  .refine(value => {
    try {
      return normalizeOfficialOpenScadProjectPath(value) === value
    } catch {
      return false
    }
  }, 'Path must be a normalized relative POSIX project path')

export function base64DecodedLength(value: string): number {
  if (value.length === 0) return 0
  const padding = value.endsWith('==') ? 2 : value.endsWith('=') ? 1 : 0
  return (value.length / 4) * 3 - padding
}

export const strictBase64Schema = z.string()
  .max(Math.ceil(OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES / 3) * 4)
  .regex(/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/)
  .refine(value => Buffer.from(value, 'base64').toString('base64') === value, 'Data must use canonical base64')
  .refine(
    value => base64DecodedLength(value) <= OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
    `Decoded data must not exceed ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES} bytes`,
  )
export const officialProjectFileSchema = z.union([
  z.object({
    path: officialProjectPathSchema,
    text: z.string().max(OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES)
      .refine(isWellFormedUnicode, 'Text file must contain well-formed Unicode')
      .refine(
        value => utf8Encoder.encode(value).byteLength <= OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
        `Text file must not exceed ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES} UTF-8 bytes`,
      ),
    data_base64: z.never().optional(),
  }).strict(),
  z.object({
    path: officialProjectPathSchema,
    text: z.never().optional(),
    data_base64: strictBase64Schema,
  }).strict(),
])
export const independentProjectPathSchema = z.string().min(1).max(OPENSCAD_PROJECT_MAX_PATH_BYTES)
  .refine(value => {
    try {
      return normalizeOpenScadProjectPath(value) === value
    } catch {
      return false
    }
  }, 'Path must be a normalized relative POSIX project path')
export const independentProjectTextSchema = z.string().max(OPENSCAD_PROJECT_MAX_SOURCE_BYTES)
  .refine(isWellFormedUnicode, 'Text file must contain well-formed Unicode')
  .refine(
    value => utf8Encoder.encode(value).byteLength <= OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
    `Text file must not exceed ${OPENSCAD_PROJECT_MAX_SOURCE_BYTES} UTF-8 bytes`,
  )
export const independentProjectBase64Schema = z.string()
  .max(Math.ceil(OPENSCAD_PROJECT_MAX_BLOB_BYTES / 3) * 4)
  .regex(/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/)
  .refine(value => Buffer.from(value, 'base64').toString('base64') === value, 'Data must use canonical base64')
  .refine(
    value => base64DecodedLength(value) <= OPENSCAD_PROJECT_MAX_BLOB_BYTES,
    `Decoded data must not exceed ${OPENSCAD_PROJECT_MAX_BLOB_BYTES} bytes`,
  )
export const independentProjectFileSchema = z.union([
  z.object({
    path: independentProjectPathSchema,
    text: independentProjectTextSchema,
    data_base64: z.never().optional(),
  }).strict(),
  z.object({
    path: independentProjectPathSchema,
    text: z.never().optional(),
    data_base64: independentProjectBase64Schema,
  }).strict(),
])
export const officialDefinesSchema = z.array(
  z.string().max(OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES)
    .refine(isWellFormedUnicode, 'Definition must contain well-formed Unicode')
    .regex(/^[$A-Za-z_][$A-Za-z0-9_]*\s*=[^\r\n\0]*$/, 'Definition must be a single-line name=value expression'),
).max(OFFICIAL_OPENSCAD_MAX_DEFINES)
  .refine(
    values => values.reduce((bytes, value) => bytes + utf8Encoder.encode(value).byteLength, 0)
      <= OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES,
    `Definitions must not exceed ${OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES} UTF-8 bytes in total`,
  )
export const officialRunInputShape = {
  source: officialSourceSchema.describe('OpenSCAD source mounted as /project/main.scad in isolated MEMFS.'),
  files: z.array(officialProjectFileSchema).max(OFFICIAL_OPENSCAD_MAX_PROJECT_FILES).default([])
    .describe('Relative project files mounted in MEMFS; each file contains exactly one of text or data_base64.'),
  experimental_features: z.array(officialExperimentalFeatureSchema)
    .max(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES.length)
    .refine(values => new Set(values).size === values.length, 'Experimental features must be unique')
    .default([]),
  defines: officialDefinesSchema.default([]),
  time: z.number().finite().min(0).max(1).optional(),
  backend: z.enum(['Manifold', 'CGAL']).default('Manifold'),
  hard_warnings: z.boolean().default(true),
  check_parameters: z.boolean().default(true),
  check_parameter_ranges: z.boolean().default(true),
  timeout_ms: z.number().int().min(1).max(OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS)
    .default(OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS),
}

export function officialToolInputSchema<T extends z.ZodRawShape>(extra: T) {
  return z.object({ ...officialRunInputShape, ...extra }).strict().superRefine((input, context) => {
    const project = input as unknown as {
      source: string
      files: Array<{ path: string; text?: string; data_base64?: string }>
    }
    const paths = new Set<string>()
    let bytes = utf8Encoder.encode(project.source).byteLength
    for (const [index, file] of project.files.entries()) {
      if ([...paths].some(path => officialOpenScadProjectPathsConflict(path, file.path))) {
        context.addIssue({
          code: 'custom',
          path: ['files', index, 'path'],
          message: 'Project file paths must not duplicate or contain one another',
        })
      }
      paths.add(file.path)
      bytes += file.text !== undefined
        ? utf8Encoder.encode(file.text).byteLength
        : Buffer.from(file.data_base64!, 'base64').byteLength
    }
    if (bytes > OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES) {
      context.addIssue({
        code: 'custom',
        path: ['files'],
        message: `Project must not exceed ${OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES} bytes`,
      })
    }
  })
}

export const officialLogEntrySchema = z.string().max(OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES)
  .refine(
    value => utf8Encoder.encode(value).byteLength <= OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES,
    `Log entry must not exceed ${OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES} UTF-8 bytes`,
  )
export const officialLogsSchema = z.object({
  stdout: z.array(officialLogEntrySchema).max(OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES),
  stderr: z.array(officialLogEntrySchema).max(OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES),
  truncated: z.boolean(),
}).strict().superRefine((logs, context) => {
  if (logs.stdout.length + logs.stderr.length > OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES) {
    context.addIssue({ code: 'custom', message: 'Combined log entry count exceeds the official runtime limit' })
  }
  const bytes = [...logs.stdout, ...logs.stderr]
    .reduce((total, entry) => total + utf8Encoder.encode(entry).byteLength, 0)
  if (bytes > OFFICIAL_OPENSCAD_MAX_LOG_BYTES) {
    context.addIssue({ code: 'custom', message: 'Combined log bytes exceed the official runtime limit' })
  }
})
export const officialCapabilitiesSchema = z.object({
  provider_role: z.literal('qualification-oracle'),
  available: z.boolean(),
  unavailable_reason: z.enum([
    'not-installed',
    'manifest-invalid',
    'runtime-integrity-failed',
    'permission-model-unavailable',
  ]).nullable(),
  expected_runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION),
  runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION).nullable(),
  archive_sha256: z.literal(OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256),
  runtime_sha256: sha256Schema.nullable(),
  patch_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION),
  default_font: z.object({
    family: z.literal(OFFICIAL_OPENSCAD_FONT_FAMILY),
    filename: z.literal(OFFICIAL_OPENSCAD_FONT_FILENAME),
    sha256: sha256Schema.nullable(),
    license_filename: z.literal(OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME),
    license_sha256: sha256Schema.nullable(),
  }).strict(),
  isolation: z.object({
    filesystem: z.literal('MEMFS'),
    node_permission_model: z.literal(true),
    scad_host_file_read: z.literal(false),
    scad_host_file_write: z.literal(false),
    network_api_exposed: z.literal(false),
    network_sandbox_enforced: z.literal(false),
    wasm_memory_limit_enforced: z.literal(false),
    process_model: z.literal('one-shot'),
  }).strict(),
  formats: z.array(officialFormatSchema),
  experimental_features: z.array(officialExperimentalFeatureSchema),
  defaults: z.object({
    backend: z.literal('Manifold'),
    hard_warnings: z.literal(true),
    check_parameters: z.literal(true),
    check_parameter_ranges: z.literal(true),
    experiments_enabled: z.literal(false),
  }).strict(),
  limits: z.object({
    source_bytes: z.number().int().positive(),
    project_files: z.number().int().positive(),
    project_file_bytes: z.number().int().positive(),
    project_bytes: z.number().int().positive(),
    wire_request_bytes: z.number().int().positive(),
    output_bytes: z.number().int().positive(),
    log_bytes: z.number().int().positive(),
    log_entries: z.number().int().positive(),
    timeout_ms: z.number().int().positive(),
  }).strict(),
  setup_command: z.string().min(1),
}).strict()
export const officialLanguageSummarySchema = z.object({
  id: z.literal(OPENSCAD_2021_01_CONTRACT.id),
  release: z.literal(OPENSCAD_2021_01_CONTRACT.languageTarget.release),
  tag: z.literal(OPENSCAD_2021_01_CONTRACT.languageTarget.tag),
  commit: z.literal(OPENSCAD_2021_01_CONTRACT.languageTarget.commit),
  scope: z.literal('stable-builtins-38-functions-35-modules'),
  function_count: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.functions.length),
  module_count: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.modules.length),
  functions: z.array(z.string()).length(OPENSCAD_2021_01_CONTRACT.builtins.functions.length),
  modules: z.array(z.string()).length(OPENSCAD_2021_01_CONTRACT.builtins.modules.length),
  compatibility_tail_policy: z.literal(
    OPENSCAD_2021_01_CONTRACT.executionRuntime.compatibilityTailPolicy,
  ),
  resource_uri: z.literal(OFFICIAL_LANGUAGE_RESOURCE_URI),
}).strict()
export const officialCheckSchema = z.object({
  provider_role: z.literal('qualification-oracle'),
  language_contract: officialLanguageSummarySchema,
  runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION),
  source_sha256: sha256Schema,
  csg_sha256: sha256Schema,
  csg_bytes: z.number().int().nonnegative(),
  duration_ms: z.number().finite().nonnegative(),
  logs: officialLogsSchema,
  experimental_features: z.array(officialExperimentalFeatureSchema),
}).strict()
export const officialExportSchema = z.object({
  provider_role: z.literal('qualification-oracle'),
  language_contract: officialLanguageSummarySchema,
  runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION),
  source_sha256: sha256Schema,
  format: officialFormatSchema,
  duration_ms: z.number().finite().nonnegative(),
  logs: officialLogsSchema,
  experimental_features: z.array(officialExperimentalFeatureSchema),
}).strict()
export const officialArtifactSchema = z.object({
  id: sha256Schema,
  resource_uri: z.string(),
  file_name: z.string(),
  format: officialFormatSchema,
  mime_type: z.string(),
  sha256: sha256Schema,
  byte_length: z.number().int().nonnegative(),
}).strict()
export const capabilityIdSchema = z.string().max(128)
  .regex(/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
const executionCommonShape = {
  required_capabilities: z.array(capabilityIdSchema).max(32),
  purpose: z.enum(['preview', 'full', 'analysis', 'export']),
  quality: qualitySchema,
  automatic_fallback: z.literal(false),
}
export function meshExecutionSchemaForManifest(
  manifest: typeof CURRENT_MESH_MANIFEST,
) {
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
export const currentMeshExecutionSchema = meshExecutionSchemaForManifest(
  CURRENT_MESH_MANIFEST,
)
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
export const currentExecutionSchema = z.union([currentMeshExecutionSchema, brepExecutionSchema])
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
export const legacyBackfillExecutionSchema = z.union([
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
export const executionSchema = z.union([
  currentExecutionSchema,
  legacyBackfillExecutionSchema,
])
export const runtimeExecutionSchema = z.union([
  currentMeshExecutionSchema.extend({ evidence: z.literal('runtime') }),
  brepExecutionSchema.extend({ evidence: z.literal('runtime') }),
])
export const persistedRuntimeExecutionSchema = runtimeExecutionSchema
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
export const meshEngineManifestSchema = z.object({
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
})
export const brepEngineManifestSchema = z.object({
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
})
export const engineRegistrySchema = z.object({
  contract_version: z.literal(1),
  source_directed_routing: z.literal(true),
  automatic_fallback: z.literal(false),
  routes: z.tuple([
    z.object({
      language_contract: z.literal('legacy/current'),
      engine_class: z.literal('mesh'),
      fallback: z.literal('never'),
    }).strict(),
    z.object({
      language_contract: z.literal('openscad-viewer/brep-1'),
      engine_class: z.literal('brep'),
      fallback: z.literal('never'),
    }).strict(),
  ]),
  engines: z.tuple([
    meshEngineManifestSchema,
    // The archived manifest is static; runtime readiness may refuse this provider.
    brepEngineManifestSchema.extend({availability: z.enum(['available', 'unavailable'])}),
  ]),
})
export const wireStringSchema = z.string().max(64 * 1024)
  .refine(isWellFormedUnicode, 'String must contain well-formed Unicode')
export const promptGoalSchema = z.string().max(2_048)
  .refine(isWellFormedUnicode, 'Goal must contain well-formed Unicode')
export const scalarSchema = z.union([z.number().finite(), z.boolean(), wireStringSchema])
export const pointSchema = z.array(z.number().finite()).length(3)
export const topologySchema = z.object({
  boundary: z.number().int().nonnegative(),
  crease: z.number().int().nonnegative(),
  non_manifold: z.number().int().nonnegative(),
  degenerate: z.number().int().nonnegative(),
})
export const boundsSchema = z.object({ min: pointSchema, max: pointSchema }).nullable()
export const analysisSchema = z.object({
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
export const independentProjectInputShape = {
  source: independentProjectTextSchema,
  files: z.array(independentProjectFileSchema).max(OPENSCAD_PROJECT_MAX_FILES - 1).default([])
    .describe('Bounded project files; each contains exactly one of text or canonical data_base64.'),
}

export function validateIndependentProjectInput(
  input: {
    source: string
    files: Array<{ path: string; text?: string; data_base64?: string }>
  },
  context: z.core.$RefinementCtx,
): void {
  const paths = new Set<string>(['main.scad'])
  let bytes = utf8Encoder.encode(input.source).byteLength
  for (const [index, file] of input.files.entries()) {
    if (paths.has(file.path)) {
      context.addIssue({
        code: 'custom',
        path: ['files', index, 'path'],
        message: file.path === 'main.scad'
          ? 'main.scad is reserved for the source entrypoint'
          : 'Project file paths must be unique',
      })
    }
    paths.add(file.path)
    bytes += file.text !== undefined
      ? utf8Encoder.encode(file.text).byteLength
      : Buffer.from(file.data_base64!, 'base64').byteLength
  }
  if (bytes > OPENSCAD_PROJECT_MAX_TOTAL_BYTES) {
    context.addIssue({
      code: 'custom',
      path: ['files'],
      message: `Project must not exceed ${OPENSCAD_PROJECT_MAX_TOTAL_BYTES} bytes`,
    })
  }
}

export const independentCheckInputSchema = z.object({
  ...independentProjectInputShape,
  quality: qualitySchema.default('full'),
  time: z.number().finite().min(0).max(1).default(0),
}).strict().superRefine(validateIndependentProjectInput)
export const independentExportFormatSchema = z.enum(['stl', 'obj'])
export const independentExportFileNameSchema = z.string().min(1).max(255)
  .refine(isWellFormedUnicode, 'File name must contain well-formed Unicode')
  .refine(value => value.trim().length > 0, 'File name must contain a non-whitespace character')
  .refine(value => !/[/\\\0]/.test(value), 'File name must not contain path separators or NUL')
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
const diagnosticCommonShape = {
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
