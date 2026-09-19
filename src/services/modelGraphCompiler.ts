import { prepareGraphRust } from './languages/kernel'
import { sha256Hex } from '../core/sha256'
import type { ModelGraphCompilation } from './modelGraph'

export class ModelGraphError extends Error {
  constructor(readonly code: string, readonly path: string, message: string, readonly details?: unknown) { super(message) }
}

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value !== null && typeof value === 'object') return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`).join(',')}}`
  return JSON.stringify(value)
}

export const hashModelGraphDocument = (document: unknown) => sha256Hex(canonical(document))

/** Runtime compilation needs Rust validation, not MCP's schema construction. */
export function compileModelGraph(value: unknown): ModelGraphCompilation {
  const result = prepareGraphRust<Omit<ModelGraphCompilation, 'document_sha256'>>('graph', value)
  if (!result.ok) throw new ModelGraphError(result.error.code, result.error.path, result.error.message, result.error.details)
  return { ...result.value, document_sha256: hashModelGraphDocument(result.value.document) }
}
