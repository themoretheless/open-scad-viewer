/**
 * `sameMeshJson(a, b)` is `stringifyMeshJson(a) === stringifyMeshJson(b)` without building
 * either string: identical references short-circuit, numeric arrays and typed arrays compare
 * element-wise, objects compare their JSON-visible keys in order. Values JSON cannot represent
 * the same way everywhere (functions, symbols, toJSON, exotic objects) fall back to the strings.
 */
import { stringifyMeshJson } from './meshJson'

const UNDECIDED = 2

export function sameMeshJson(a: unknown, b: unknown): boolean {
  const result = compare(a, b, 0)
  return result === UNDECIDED ? stringifyMeshJson(a) === stringifyMeshJson(b) : result === 1
}

const jsonNumber = (n: number) => (Number.isFinite(n) ? (n === 0 ? 0 : n) : null)
const isNumericView = (v: unknown): v is ArrayLike<number> =>
  ArrayBuffer.isView(v) && !(v instanceof DataView) && !(v instanceof BigInt64Array) && !(v instanceof BigUint64Array)
const plain = (v: object) => {
  const prototype = Object.getPrototypeOf(v)
  return (prototype === Object.prototype || prototype === null) && !('toJSON' in v)
}
/** JSON drops undefined, functions and symbols from objects. */
const visible = (v: unknown) => v !== undefined && typeof v !== 'function' && typeof v !== 'symbol'

function compare(a: unknown, b: unknown, depth: number): 0 | 1 | typeof UNDECIDED {
  if (a === b && (typeof a !== 'number' || Number.isFinite(a))) return 1
  if (depth > 200) return UNDECIDED
  const ta = typeof a,
    tb = typeof b
  if (ta === 'number' && tb === 'number') return jsonNumber(a as number) === jsonNumber(b as number) ? 1 : 0
  if (ta === 'number' || tb === 'number') {
    // A non-finite number serializes as null.
    const n = ta === 'number' ? (a as number) : (b as number),
      other = ta === 'number' ? b : a
    return !Number.isFinite(n) && other === null ? 1 : 0
  }
  if (
    ta === 'bigint' ||
    tb === 'bigint' ||
    ta === 'function' ||
    tb === 'function' ||
    ta === 'symbol' ||
    tb === 'symbol'
  )
    return UNDECIDED
  if (
    (ta === 'object' && a !== null && !Array.isArray(a) && !isNumericView(a) && !plain(a as object)) ||
    (tb === 'object' && b !== null && !Array.isArray(b) && !isNumericView(b) && !plain(b as object))
  )
    return UNDECIDED
  if (a === null || b === null || ta !== 'object' || tb !== 'object') return a === b ? 1 : 0
  const arrayA = Array.isArray(a) || isNumericView(a),
    arrayB = Array.isArray(b) || isNumericView(b)
  if (arrayA !== arrayB) return 0
  if (arrayA) {
    const x = a as ArrayLike<unknown>,
      y = b as ArrayLike<unknown>
    if (x.length !== y.length) return 0
    for (let i = 0; i < x.length; i++) {
      const u = x[i],
        v = y[i]
      if (u === v && typeof u !== 'object' && (typeof u !== 'number' || Number.isFinite(u))) continue
      // Arrays serialize undefined, functions and symbols as null.
      const r = compare(visible(u) ? u : null, visible(v) ? v : null, depth + 1)
      if (r !== 1) return r
    }
    return 1
  }
  if (!plain(a as object) || !plain(b as object)) return UNDECIDED
  const ka = Object.keys(a as object).filter(k => visible((a as Record<string, unknown>)[k]))
  const kb = Object.keys(b as object).filter(k => visible((b as Record<string, unknown>)[k]))
  if (ka.length !== kb.length) return 0
  for (let i = 0; i < ka.length; i++) {
    if (ka[i] !== kb[i]) return 0
    const r = compare((a as Record<string, unknown>)[ka[i]!], (b as Record<string, unknown>)[kb[i]!], depth + 1)
    if (r !== 1) return r
  }
  return 1
}
