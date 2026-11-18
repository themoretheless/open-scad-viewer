import { randomUUID } from 'node:crypto'
import type { PublicToolError } from '../publicError'
import { mapGeometryAndCatalogError } from './mapGeometryAndCatalogError'
import { mapOfficialRuntimeError } from './mapOfficialRuntimeError'
import { mapSourceCompilerError } from './mapSourceCompilerError'

export function publicToolError(error: unknown): { error: PublicToolError; internal: boolean } {
  const mapped = mapOfficialRuntimeError(error) ?? mapSourceCompilerError(error) ?? mapGeometryAndCatalogError(error)
  if (mapped) return mapped
  return {
    error: {
      code: 'internal_error',
      message: 'The MCP server could not complete the request.',
      retryable: true,
      next_action: 'Retry once; if the failure persists, inspect the server log using the correlation_id.',
      correlation_id: randomUUID(),
    },
    internal: true,
  }
}
