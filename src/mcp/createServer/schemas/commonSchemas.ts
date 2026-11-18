import { type RequestId } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { type McpGeometryService } from '../../geometryService'
import {
  MAX_MODEL_ID_LENGTH,
  MAX_MODEL_NAME_LENGTH,
  MAX_MODEL_SOURCE_LENGTH,
  isWellFormedUnicode,
  type ModelStore,
} from '../../modelStore'
import { type OfficialOpenScadRuntimeService } from '../../officialOpenScadRuntimeService'
import { type McpToolsetProfile } from '../../toolsetProfiles'

export interface CreateOpenScadMcpServerOptions {
  readonly localEditor?: boolean
  readonly toolsetProfiles?: ReadonlySet<McpToolsetProfile>
  store: ModelStore
  additionalInstructions?: string
  geometry?: McpGeometryService
  /** Optional upstream oracle used only by the explicitly named official tools. */
  officialRuntime?: OfficialOpenScadRuntimeService
  /** Releases transport admission only after an SDK request handler settles. */
  onRequestSettled?: (requestId: RequestId) => void
  /** Mirrors the SDK handler AbortSignal after protocol validation. */
  onRequestCancelled?: (requestId: RequestId) => void
}

export interface SelectedModel {
  id: string
  name: string
  source: string
  revision: number
}

export const utf8Encoder = new TextEncoder()

export function wellFormedText(maxLength: number, label = 'Value') {
  return z.string().max(maxLength).refine(isWellFormedUnicode, `${label} must contain well-formed Unicode`)
}

export const idSchema = z
  .string()
  .min(1)
  .max(MAX_MODEL_ID_LENGTH)
  .regex(/^[A-Za-z0-9][A-Za-z0-9._:-]*$/)

export const nameSchema = z
  .string()
  .min(1)
  .max(MAX_MODEL_NAME_LENGTH)
  .refine(isWellFormedUnicode, 'Name must contain well-formed Unicode')
  .refine(name => name.trim().length > 0, 'Name must contain a non-whitespace character')

export const sourceSchema = wellFormedText(MAX_MODEL_SOURCE_LENGTH, 'Source')
export const qualitySchema = z.enum(['preview', 'full'])
export const sha256Schema = z.string().regex(/^[a-f0-9]{64}$/)

const wireStringSchema = wellFormedText(64 * 1024, 'String')
export const promptGoalSchema = wellFormedText(2_048, 'Goal')
export const scalarSchema = z.union([z.number().finite(), z.boolean(), wireStringSchema])

export const modelMetadataSchema = z.object({
  id: z.string(),
  name: z.string(),
  revision: z.number().int().nonnegative(),
  created_at: z.string(),
  updated_at: z.string(),
  resource_uri: z.string(),
})

export const sourceSelectorFields = {
  source: sourceSchema.describe('Inline OpenSCAD source; mutually exclusive with model_id.').optional(),
  model_id: idSchema.describe('Saved DuckDB model id; mutually exclusive with source.').optional(),
  revision: z
    .number()
    .int()
    .nonnegative()
    .describe('Immutable saved-model revision; valid only with model_id. Defaults to the current revision.')
    .optional(),
}

export const sourceReferenceSchema = z.union([
  z
    .object({
      source: sourceSelectorFields.source.unwrap(),
      model_id: z.never().optional(),
      revision: z.never().optional(),
    })
    .strict(),
  z
    .object({
      source: z.never().optional(),
      model_id: sourceSelectorFields.model_id.unwrap(),
      revision: sourceSelectorFields.revision,
    })
    .strict(),
])
