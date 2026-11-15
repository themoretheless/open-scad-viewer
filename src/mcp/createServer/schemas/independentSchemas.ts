import { z } from 'zod/v4'
import { OPENSCAD_2021_01_CONTRACT } from '../../../core/openScad2021Contract'
import {
  OPENSCAD_PROJECT_MAX_BLOB_BYTES,
  OPENSCAD_PROJECT_MAX_FILES,
  OPENSCAD_PROJECT_MAX_PATH_BYTES,
  OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
  OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
  normalizeOpenScadProjectPath,
} from '../../../language/openScadProject'
import { DEFAULT_MAX_ARTIFACT_BYTES, MAX_ARTIFACT_BYTES } from '../../geometryService'
import { isWellFormedUnicode } from '../../modelStore'
import { INDEPENDENT_ENGINE_ID } from '../../server/contracts'
import { qualitySchema, sha256Schema, utf8Encoder } from './commonSchemas'

const independentProjectPathSchema = z
  .string()
  .min(1)
  .max(OPENSCAD_PROJECT_MAX_PATH_BYTES)
  .refine(value => {
    try {
      return normalizeOpenScadProjectPath(value) === value
    } catch {
      return false
    }
  }, 'Path must be a normalized relative POSIX project path')

const independentProjectTextSchema = z
  .string()
  .max(OPENSCAD_PROJECT_MAX_SOURCE_BYTES)
  .refine(isWellFormedUnicode, 'Text file must contain well-formed Unicode')
  .refine(
    value => utf8Encoder.encode(value).byteLength <= OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
    `Text file must not exceed ${OPENSCAD_PROJECT_MAX_SOURCE_BYTES} UTF-8 bytes`,
  )

const independentProjectBase64Schema = z
  .string()
  .max(Math.ceil(OPENSCAD_PROJECT_MAX_BLOB_BYTES / 3) * 4)
  .regex(/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/)
  .refine(value => Buffer.from(value, 'base64').toString('base64') === value, 'Data must use canonical base64')
  .refine(
    value => Buffer.from(value, 'base64').byteLength <= OPENSCAD_PROJECT_MAX_BLOB_BYTES,
    `Decoded data must not exceed ${OPENSCAD_PROJECT_MAX_BLOB_BYTES} bytes`,
  )

export const independentProjectFileSchema = z.union([
  z
    .object({
      path: independentProjectPathSchema,
      text: independentProjectTextSchema,
      data_base64: z.never().optional(),
    })
    .strict(),
  z
    .object({
      path: independentProjectPathSchema,
      text: z.never().optional(),
      data_base64: independentProjectBase64Schema,
    })
    .strict(),
])

export function validateIndependentProjectInput(
  input: {
    source: string
    files: Array<{ path: string; text?: string | undefined; data_base64?: string | undefined }>
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
        message:
          file.path === 'main.scad'
            ? 'main.scad is reserved for the source entrypoint'
            : 'Project file paths must be unique',
      })
    }
    paths.add(file.path)
    bytes +=
      file.text !== undefined
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

export const independentRunInputShape = {
  source: independentProjectTextSchema,
  files: z
    .array(independentProjectFileSchema)
    .max(OPENSCAD_PROJECT_MAX_FILES - 1)
    .default([])
    .describe('Bounded project files; each contains exactly one of text or canonical data_base64.'),
}

export const independentCheckInputSchema = z
  .object({
    ...independentRunInputShape,
    quality: qualitySchema.default('full'),
    time: z.number().finite().min(0).max(1).default(0),
  })
  .strict()
  .superRefine(validateIndependentProjectInput)

const independentExportFormatSchema = z.enum(['stl', 'obj'])
const independentExportFileNameSchema = z
  .string()
  .min(1)
  .max(255)
  .refine(isWellFormedUnicode, 'File name must contain well-formed Unicode')
  .refine(value => value.trim().length > 0, 'File name must contain a non-whitespace character')
  .refine(value => !/[/\\\0]/.test(value), 'File name must not contain path separators or NUL')

export const independentExportInputSchema = z
  .object({
    ...independentRunInputShape,
    format: independentExportFormatSchema,
    file_name: independentExportFileNameSchema.optional(),
    max_bytes: z.number().int().positive().max(MAX_ARTIFACT_BYTES).default(DEFAULT_MAX_ARTIFACT_BYTES),
    quality: z.literal('full').default('full'),
    time: z.number().finite().min(0).max(1).default(0),
  })
  .strict()
  .superRefine(validateIndependentProjectInput)

const independentEngineAttestationSchema = z
  .object({
    id: z.literal(INDEPENDENT_ENGINE_ID),
    stage: z.literal('development'),
    language_contract: z.literal(OPENSCAD_2021_01_CONTRACT.id),
    upstream_runtime_used: z.literal(false),
    production_authoritative: z.literal(false),
    complete_language_claim: z.literal(false),
    stable_function_inventory: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.functions.length),
    stable_module_inventory: z.literal(OPENSCAD_2021_01_CONTRACT.builtins.modules.length),
  })
  .strict()

const independentMetricsSchema = z
  .object({
    mesh_count: z.number().int().nonnegative(),
    vertex_count: z.number().int().nonnegative(),
    triangle_count: z.number().int().nonnegative(),
    volume: z.number().finite().nonnegative(),
    surface_area: z.number().finite().nonnegative(),
  })
  .strict()

export const independentCheckSchema = z
  .object({
    engine: independentEngineAttestationSchema,
    source_sha256: sha256Schema,
    quality: qualitySchema,
    duration_ms: z.number().finite().nonnegative(),
    ...independentMetricsSchema.shape,
    warnings: z.array(z.string()),
  })
  .strict()

export const independentExportSchema = z
  .object({
    engine: independentEngineAttestationSchema,
    source_sha256: sha256Schema,
    quality: z.literal('full'),
    format: independentExportFormatSchema,
    duration_ms: z.number().finite().nonnegative(),
    metrics: independentMetricsSchema,
    warnings: z.array(z.string()),
  })
  .strict()

export const independentArtifactSchema = z
  .object({
    id: sha256Schema,
    resource_uri: z.string(),
    file_name: independentExportFileNameSchema,
    format: independentExportFormatSchema,
    mime_type: z.enum(['model/stl', 'model/obj']),
    sha256: sha256Schema,
    byte_length: z.number().int().nonnegative(),
  })
  .strict()
