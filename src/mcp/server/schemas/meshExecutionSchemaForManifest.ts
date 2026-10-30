// schemas: meshExecutionSchemaForManifest.
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
import { executionCommonShape, sha256Schema, capabilityIdSchema, qualitySchema, independentProjectTextSchema, independentProjectFileSchema, utf8Encoder } from './idSchema'

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

export const legacyBackfillCommonShape = {
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

export const engineManifestCommonShape = {
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
