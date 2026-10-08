import { prepareGraphRust } from './languages/kernel'
import { sha256Hex } from '../core/sha256'
import type { RushGraphNurbsCompilation } from './rushGraphNurbs'

export class RushGraphNurbsError extends Error {
  constructor(readonly code: string, readonly path: string, message: string) { super(message); this.name = 'RushGraphNurbsError' }
}

export function compileRushGraphNurbs(input: unknown): RushGraphNurbsCompilation {
  const result = prepareGraphRust<Omit<RushGraphNurbsCompilation, 'document_sha256'>>('nurbs', input)
  if (!result.ok) throw new RushGraphNurbsError(result.error.code, result.error.path, result.error.message)
  return { ...result.value, document_sha256: hashNurbsDocument(result.value.document) }
}

export function hashNurbsDocument(document: unknown): string {
  // Preserve this format's existing ordering and encoding, distinct from Rush IR/1.
  const canonical = (v: unknown): string => Array.isArray(v) ? `[${v.map(canonical).join(',')}]` : v && typeof v === 'object' ? `{${Object.entries(v).sort(([a], [b]) => a.localeCompare(b)).map(([k, x]) => JSON.stringify(k) + ':' + canonical(x)).join(',')}}` : JSON.stringify(v)
  return sha256Hex(canonical(document))
}
