/**
 * Deep copy of a document-shaped JSON tree (plain objects, arrays, numbers, strings,
 * booleans, null, typed arrays). Numeric arrays are copied with `slice`, which V8 turns
 * into a block copy for packed arrays; this is several times faster than `structuredClone`
 * on mesh-heavy documents. Anything that is not plain data falls back to `structuredClone`
 * so the result never aliases caller-owned state.
 */
export function cloneDocumentTree<T>(value: T): T {
  return copy(value, 0) as T
}

/** Copy plus an estimate of its retained bytes (8 per number, 2 per string character,
 * 16 per container), without serializing it. */
export function cloneDocumentTreeSized<T>(value: T): { copy: T; bytes: number } {
  sized = 0
  measuring = true
  try {
    return { copy: copy(value, 0) as T, bytes: sized }
  } finally {
    measuring = false
  }
}
let sized = 0,
  measuring = false

const MAX_DEPTH = 200

function copy(value: unknown, depth: number): unknown {
  if (value === null || typeof value !== 'object') {
    if (measuring) sized += typeof value === 'string' ? 2 * value.length : 8
    return value
  }
  if (measuring) sized += 16
  if (depth > MAX_DEPTH) return structuredClone(value)
  if (Array.isArray(value)) {
    const length = value.length
    let numeric = true
    // A hole reads as undefined, leaves the fast path and is copied as undefined (JSON trees have none).
    for (let i = 0; i < length; i++)
      if (typeof value[i] !== 'number') {
        numeric = false
        break
      }
    if (numeric) {
      if (measuring) sized += 8 * length
      return value.slice()
    }
    const out = new Array(length)
    for (let i = 0; i < length; i++) out[i] = copy(value[i], depth + 1)
    return out
  }
  if (ArrayBuffer.isView(value)) {
    if (measuring) sized += value.byteLength
    return value instanceof DataView ? structuredClone(value) : (value as unknown as Uint8Array).slice()
  }
  const prototype = Object.getPrototypeOf(value)
  if (prototype !== Object.prototype && prototype !== null) return structuredClone(value)
  const out: Record<string, unknown> = {}
  for (const key of Object.keys(value)) {
    if (measuring) sized += 2 * key.length
    out[key] = copy((value as Record<string, unknown>)[key], depth + 1)
  }
  return out
}
