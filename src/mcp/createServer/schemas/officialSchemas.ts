import { z } from 'zod/v4'
import { OPENSCAD_2021_01_CONTRACT } from '../../../core/openScad2021Contract'
import { isWellFormedUnicode } from '../../modelStore'
import {
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
} from '../../officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS,
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES,
  OFFICIAL_OPENSCAD_MAX_DEFINES,
  OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
  OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
  OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  normalizeOfficialOpenScadProjectPath,
} from '../../officialOpenScadRuntimeProtocol'
import { OFFICIAL_LANGUAGE_RESOURCE_URI } from '../../server/contracts'
import { sha256Schema, utf8Encoder } from './commonSchemas'
import { toolErrorResultSchema } from './executionAndBuildSchemas'

export const officialFormatSchema = z.enum(OFFICIAL_OPENSCAD_EXPORT_FORMATS)
const officialExperimentalFeatureSchema = z.enum(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)

const officialSourceSchema = z
  .string()
  .max(OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES)
  .refine(isWellFormedUnicode, 'Source must contain well-formed Unicode')
  .refine(
    value => utf8Encoder.encode(value).byteLength <= OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
    `Source must not exceed ${OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES} UTF-8 bytes`,
  )

const officialProjectPathSchema = z
  .string()
  .min(1)
  .max(1_024)
  .refine(value => {
    try {
      return normalizeOfficialOpenScadProjectPath(value) === value
    } catch {
      return false
    }
  }, 'Path must be a normalized relative POSIX project path')

const strictBase64Schema = z
  .string()
  .max(Math.ceil(OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES / 3) * 4)
  .regex(/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/)
  .refine(value => Buffer.from(value, 'base64').toString('base64') === value, 'Data must use canonical base64')
  .refine(
    value => Buffer.from(value, 'base64').byteLength <= OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
    `Decoded data must not exceed ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES} bytes`,
  )

export const officialProjectFileSchema = z.union([
  z
    .object({
      path: officialProjectPathSchema,
      text: z
        .string()
        .max(OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES)
        .refine(isWellFormedUnicode, 'Text file must contain well-formed Unicode')
        .refine(
          value => utf8Encoder.encode(value).byteLength <= OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
          `Text file must not exceed ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES} UTF-8 bytes`,
        ),
      data_base64: z.never().optional(),
    })
    .strict(),
  z
    .object({
      path: officialProjectPathSchema,
      text: z.never().optional(),
      data_base64: strictBase64Schema,
    })
    .strict(),
])

const officialDefinesSchema = z
  .array(
    z
      .string()
      .max(OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES)
      .refine(isWellFormedUnicode, 'Definition must contain well-formed Unicode')
      .regex(/^[$A-Za-z_][$A-Za-z0-9_]*\s*=[^\r\n\0]*$/, 'Definition must be a single-line name=value expression'),
  )
  .max(OFFICIAL_OPENSCAD_MAX_DEFINES)
  .refine(
    values =>
      values.reduce((bytes, value) => bytes + utf8Encoder.encode(value).byteLength, 0) <=
      OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES,
    `Definitions must not exceed ${OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES} UTF-8 bytes in total`,
  )

export const officialRunInputShape = {
  source: officialSourceSchema.describe('OpenSCAD source mounted as /project/main.scad in isolated MEMFS.'),
  files: z
    .array(officialProjectFileSchema)
    .max(OFFICIAL_OPENSCAD_MAX_PROJECT_FILES)
    .default([])
    .describe('Relative project files mounted in MEMFS; each file contains exactly one of text or data_base64.'),
  experimental_features: z
    .array(officialExperimentalFeatureSchema)
    .max(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES.length)
    .refine(values => new Set(values).size === values.length, 'Experimental features must be unique')
    .default([]),
  defines: officialDefinesSchema.default([]),
  time: z.number().finite().min(0).max(1).optional(),
  backend: z.enum(['Manifold', 'CGAL']).default('Manifold'),
  hard_warnings: z.boolean().default(true),
  check_parameters: z.boolean().default(true),
  check_parameter_ranges: z.boolean().default(true),
  timeout_ms: z
    .number()
    .int()
    .min(1)
    .max(OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS)
    .default(OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS),
}

export interface OfficialToolRunInput {
  source: string
  files: Array<{
    path: string
    text?: string | undefined
    data_base64?: string | undefined
  }>
  experimental_features: Array<(typeof OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)[number]>
  defines: string[]
  time?: number | undefined
  backend: 'Manifold' | 'CGAL'
  hard_warnings: boolean
  check_parameters: boolean
  check_parameter_ranges: boolean
  timeout_ms: number
}

const officialLogEntrySchema = z
  .string()
  .max(OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES)
  .refine(
    value => utf8Encoder.encode(value).byteLength <= OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES,
    `Log entry must not exceed ${OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES} UTF-8 bytes`,
  )

const officialLogsSchema = z
  .object({
    stdout: z.array(officialLogEntrySchema).max(OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES),
    stderr: z.array(officialLogEntrySchema).max(OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES),
    truncated: z.boolean(),
  })
  .strict()
  .superRefine((logs, context) => {
    if (logs.stdout.length + logs.stderr.length > OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES) {
      context.addIssue({ code: 'custom', message: 'Combined log entry count exceeds the official runtime limit' })
    }
    const bytes = [...logs.stdout, ...logs.stderr].reduce(
      (total, entry) => total + utf8Encoder.encode(entry).byteLength,
      0,
    )
    if (bytes > OFFICIAL_OPENSCAD_MAX_LOG_BYTES) {
      context.addIssue({ code: 'custom', message: 'Combined log bytes exceed the official runtime limit' })
    }
  })

export const officialCapabilitiesSchema = z
  .object({
    provider_role: z.literal('qualification-oracle'),
    available: z.boolean(),
    unavailable_reason: z
      .enum(['not-installed', 'manifest-invalid', 'runtime-integrity-failed', 'permission-model-unavailable'])
      .nullable(),
    expected_runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION),
    runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION).nullable(),
    archive_sha256: z.literal(OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256),
    runtime_sha256: sha256Schema.nullable(),
    patch_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION),
    default_font: z
      .object({
        family: z.literal(OFFICIAL_OPENSCAD_FONT_FAMILY),
        filename: z.literal(OFFICIAL_OPENSCAD_FONT_FILENAME),
        sha256: sha256Schema.nullable(),
        license_filename: z.literal(OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME),
        license_sha256: sha256Schema.nullable(),
      })
      .strict(),
    isolation: z
      .object({
        filesystem: z.literal('MEMFS'),
        node_permission_model: z.literal(true),
        scad_host_file_read: z.literal(false),
        scad_host_file_write: z.literal(false),
        network_api_exposed: z.literal(false),
        network_sandbox_enforced: z.literal(false),
        wasm_memory_limit_enforced: z.literal(false),
        process_model: z.literal('one-shot'),
      })
      .strict(),
    formats: z.array(officialFormatSchema),
    experimental_features: z.array(officialExperimentalFeatureSchema),
    defaults: z
      .object({
        backend: z.literal('Manifold'),
        hard_warnings: z.literal(true),
        check_parameters: z.literal(true),
        check_parameter_ranges: z.literal(true),
        experiments_enabled: z.literal(false),
      })
      .strict(),
    limits: z
      .object({
        source_bytes: z.number().int().positive(),
        project_files: z.number().int().positive(),
        project_file_bytes: z.number().int().positive(),
        project_bytes: z.number().int().positive(),
        wire_request_bytes: z.number().int().positive(),
        output_bytes: z.number().int().positive(),
        log_bytes: z.number().int().positive(),
        log_entries: z.number().int().positive(),
        timeout_ms: z.number().int().positive(),
      })
      .strict(),
    setup_command: z.string().min(1),
  })
  .strict()

export const officialLanguageSummarySchema = z
  .object({
    id: z.literal(OPENSCAD_2021_01_CONTRACT.id),
    release: z.literal(OPENSCAD_2021_01_CONTRACT.languageTarget.release),
    tag: z.literal(OPENSCAD_2021_01_CONTRACT.languageTarget.tag),
    commit: z.literal(OPENSCAD_2021_01_CONTRACT.languageTarget.commit),
    scope: z.literal('stable-builtins-38-functions-35-modules'),
    function_count: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.functions.length),
    module_count: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.modules.length),
    functions: z.array(z.string()).length(OPENSCAD_2021_01_CONTRACT.builtins.functions.length),
    modules: z.array(z.string()).length(OPENSCAD_2021_01_CONTRACT.builtins.modules.length),
    compatibility_tail_policy: z.literal(OPENSCAD_2021_01_CONTRACT.executionRuntime.compatibilityTailPolicy),
    resource_uri: z.literal(OFFICIAL_LANGUAGE_RESOURCE_URI),
  })
  .strict()

export const officialCheckSchema = z
  .object({
    provider_role: z.literal('qualification-oracle'),
    language_contract: officialLanguageSummarySchema,
    runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION),
    source_sha256: sha256Schema,
    csg_sha256: sha256Schema,
    csg_bytes: z.number().int().nonnegative(),
    duration_ms: z.number().finite().nonnegative(),
    logs: officialLogsSchema,
    experimental_features: z.array(officialExperimentalFeatureSchema),
  })
  .strict()

export const officialExportSchema = z
  .object({
    provider_role: z.literal('qualification-oracle'),
    language_contract: officialLanguageSummarySchema,
    runtime_version: z.literal(OFFICIAL_OPENSCAD_RUNTIME_VERSION),
    source_sha256: sha256Schema,
    format: officialFormatSchema,
    duration_ms: z.number().finite().nonnegative(),
    logs: officialLogsSchema,
    experimental_features: z.array(officialExperimentalFeatureSchema),
  })
  .strict()

export const officialArtifactSchema = z
  .object({
    id: sha256Schema,
    resource_uri: z.string(),
    file_name: z.string(),
    format: officialFormatSchema,
    mime_type: z.string(),
    sha256: sha256Schema,
    byte_length: z.number().int().nonnegative(),
  })
  .strict()

export const officialToolErrorResultSchema = toolErrorResultSchema
  .extend({
    official_logs: officialLogsSchema.optional(),
    duration_ms: z.number().finite().nonnegative().optional(),
  })
  .strict()
