import { MAX_DOCUMENT_CHARACTERS } from '../directDocumentLimits'

export const EXACT_SOLID_MAX_SOURCE_CHARACTERS = 100_000
export const EXACT_SOLID_MAX_DOCUMENT_CHARACTERS = MAX_DOCUMENT_CHARACTERS
export const EXACT_SOLID_TIMEOUT_MS = 120_000

export interface ExactSolidRequest {
  kind: 'exact-solid'
  version: 1
  source: string
}

export type ExactSolidResponse =
  | { kind: 'exact-solid'; version: 1; ok: true; document: string }
  | { kind: 'exact-solid'; version: 1; ok: false; error: { name: string; message: string } }

export function isExactSolidRequest(value: unknown): value is ExactSolidRequest {
  if (!value || typeof value !== 'object') return false
  const request = value as Record<string, unknown>
  return Object.keys(request).length === 3 && request.kind === 'exact-solid'
    && request.version === 1 && typeof request.source === 'string'
    && request.source.length <= EXACT_SOLID_MAX_SOURCE_CHARACTERS
}

export function isExactSolidResponse(value: unknown): value is ExactSolidResponse {
  if (!value || typeof value !== 'object') return false
  const response = value as Record<string, unknown>
  if (Object.keys(response).length !== 4 || response.kind !== 'exact-solid' || response.version !== 1) return false
  if (response.ok === true) return typeof response.document === 'string'
    && response.document.length <= EXACT_SOLID_MAX_DOCUMENT_CHARACTERS
  if (response.ok !== false || !response.error || typeof response.error !== 'object') return false
  const error = response.error as Record<string, unknown>
  return Object.keys(error).length === 2 && typeof error.name === 'string' && error.name.length <= 100
    && typeof error.message === 'string' && error.message.length <= 4096
}
