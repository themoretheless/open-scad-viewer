import { isWellFormedUnicode } from './unicode'

/** RFC-8785-style ordering for the JSON-only manifest payload used here. */
export function canonicalJson(value: unknown): string {
  if (value === null || typeof value === 'boolean') return JSON.stringify(value)
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new TypeError('Canonical JSON cannot encode non-finite numbers')
    return JSON.stringify(value)
  }
  if (typeof value === 'string') {
    if (!isWellFormedUnicode(value)) throw new TypeError('Canonical JSON requires well-formed Unicode')
    return JSON.stringify(value)
  }
  if (typeof value !== 'object') throw new TypeError('Canonical JSON supports JSON values only')
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  return `{${Object.entries(value as Record<string, unknown>)
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`)
    .join(',')}}`
}
