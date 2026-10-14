import { prepareGraphRust } from './languages/kernel'
import { sha256Hex } from '../core/sha256'
import type { RushGraphCompilation } from './rushGraph'

export class RushGraphError extends Error {
  constructor(readonly code: string, readonly path: string, message: string, readonly details?: unknown) { super(message) }
}

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value !== null && typeof value === 'object') return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`).join(',')}}`
  return JSON.stringify(value)
}

export const hashRushGraphDocument = (document: unknown) => sha256Hex(canonical(document))

/** Runtime compilation needs Rust validation, not MCP's schema construction. */
export function compileRushGraph(value: unknown): RushGraphCompilation {
  const result = prepareGraphRust<Omit<RushGraphCompilation, 'document_sha256'>>('graph', value)
  if (!result.ok) throw new RushGraphError(result.error.code, result.error.path, result.error.message, result.error.details)
  return { ...result.value, document_sha256: hashRushGraphDocument(result.value.document) }
}
