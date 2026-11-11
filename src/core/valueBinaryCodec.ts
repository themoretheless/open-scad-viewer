/** MGV1: bounded, versioned, little-endian values. No JSON text at the WASM boundary. */
const LIMIT = 32 * 1024 * 1024
const encoder = new TextEncoder()
const decoder = new TextDecoder('utf-8', { fatal: true })

/** Typed numeric arrays encode exactly like the equivalent plain array (tag 5, per-element
 * numbers), so callers can hand scene-owned Float32Array/Uint32Array views straight to the
 * kernel boundary without a temporary number[] copy. DataView and BigInt views are rejected. */
const isFlatNumberView = (v: unknown): v is ArrayBufferView & { length: number } =>
  ArrayBuffer.isView(v) && !(v instanceof DataView) && !(v instanceof BigInt64Array) && !(v instanceof BigUint64Array)

const isUnsignedIntView = (v: unknown): v is Uint32Array | Uint16Array | Uint8Array | Uint8ClampedArray =>
  v instanceof Uint32Array || v instanceof Uint16Array || v instanceof Uint8Array || v instanceof Uint8ClampedArray

const isSignedIntView = (v: unknown): v is Int32Array | Int16Array | Int8Array =>
  v instanceof Int32Array || v instanceof Int16Array || v instanceof Int8Array

/** Per-key hints: arrays of numeric triples decode directly into flat typed arrays. */
export type BinaryTripleHints = Readonly<Record<string, 'f64' | 'u8' | 'u32'>>

let sharedBuf = new Uint8Array(65536)
let sharedView = new DataView(sharedBuf.buffer)
let sharedBusy = false

export function encodeBinary(value: unknown): Uint8Array {
  // Repeated schema keys dominate CAD packets; cache only bounded keys for this call.
  const encodedKeys = new Map<string, Uint8Array>()
  const useShared = !sharedBusy
  if (useShared) sharedBusy = true
  let data = useShared ? sharedBuf : new Uint8Array(4096)
  let view = useShared ? sharedView : new DataView(data.buffer)
  let offset = 4
  let nodes = 0
  data[0] = 77; data[1] = 71; data[2] = 86; data[3] = 49

  const reserve = (n: number) => {
    if (n > LIMIT - offset) throw new Error('Binary size limit')
    if (offset + n > data.length) {
      const next = new Uint8Array(Math.min(LIMIT, Math.max(offset + n, data.length * 2)))
      next.set(data.subarray(0, offset))
      data = next
      view = new DataView(data.buffer)
      if (useShared) {
        sharedBuf = data
        sharedView = view
      }
    }
  }
  const byte = (n: number) => {
    reserve(1)
    data[offset++] = n
  }
  const count = (n: number) => {
    reserve(4)
    view.setUint32(offset, n, true)
    offset += 4
  }
  const putNumber = (v: number) => {
    if (!Number.isFinite(v)) throw new Error('Nonfinite binary number')
    const integer = Number.isSafeInteger(v) && !Object.is(v, -0)
    if (!integer) {
      reserve(9)
      data[offset] = 3
      view.setFloat64(offset + 1, v, true)
      offset += 9
      return
    }
    reserve(9)
    data[offset] = v >= 0 ? 7 : 8
    const high = Math.floor(v / 4294967296)
    const low = v - high * 4294967296
    view.setUint32(offset + 1, low >>> 0, true)
    view.setInt32(offset + 5, high | 0, true)
    offset += 9
  }
  const put = (v: unknown, depth: number, key = false) => {
    if (depth > 128 || ++nodes > 4_000_000) throw new Error('Binary nesting or item limit')
    if (v === null) { byte(0); return }
    if (v === false) { byte(1); return }
    if (v === true) { byte(2); return }
    if (typeof v === 'number') {
      putNumber(v)
      return
    }
    if (typeof v === 'string') {
      let bytes = key ? encodedKeys.get(v) : undefined
      if (!bytes) {
        bytes = encoder.encode(v)
        if (key && v.length <= 128 && encodedKeys.size < 256) encodedKeys.set(v, bytes)
      }
      reserve(5 + bytes.length)
      data[offset] = 4
      view.setUint32(offset + 1, bytes.length, true)
      data.set(bytes, offset + 5)
      offset += 5 + bytes.length
      return
    }
    if (isUnsignedIntView(v)) {
      const len = v.length
      if (len > 0 && (depth + 1 > 128 || nodes + len > 4_000_000)) throw new Error('Binary nesting or item limit')
      nodes += len
      reserve(5 + len * 9)
      data[offset] = 5
      view.setUint32(offset + 1, len, true)
      offset += 5
      for (let i = 0; i < len; i++) {
        data[offset] = 7
        view.setUint32(offset + 1, v[i], true)
        view.setUint32(offset + 5, 0, true)
        offset += 9
      }
      return
    }
    if (isSignedIntView(v)) {
      const len = v.length
      if (len > 0 && (depth + 1 > 128 || nodes + len > 4_000_000)) throw new Error('Binary nesting or item limit')
      nodes += len
      reserve(5 + len * 9)
      data[offset] = 5
      view.setUint32(offset + 1, len, true)
      offset += 5
      for (let i = 0; i < len; i++) {
        const x = v[i]
        data[offset] = x >= 0 ? 7 : 8
        view.setInt32(offset + 1, x, true)
        view.setInt32(offset + 5, x >= 0 ? 0 : -1, true)
        offset += 9
      }
      return
    }
    if (Array.isArray(v) || isFlatNumberView(v)) {
      const items = v as ArrayLike<unknown>
      const len = items.length
      byte(5)
      count(len)
      for (let i = 0; i < len; i++) put(items[i], depth + 1)
      return
    }
    if (typeof v === 'object' && v) {
      reserve(5)
      data[offset] = 6
      const countOffset = offset + 1
      offset += 5
      const keys = Object.keys(v)
      const rec = v as Record<string, unknown>
      let written = 0
      for (let i = 0; i < keys.length; i++) {
        const k = keys[i]
        const item = rec[k]
        if (item === undefined) continue
        written++
        put(k, depth + 1, true)
        put(item, depth + 1)
      }
      view.setUint32(countOffset, written, true)
      return
    }
    throw new Error('Unsupported binary value')
  }

  try {
    put(value, 0)
    return data.slice(0, offset)
  } finally {
    if (useShared) sharedBusy = false
  }
}

export function decodeBinary(data: Uint8Array, hints?: BinaryTripleHints): unknown {
  if (data.length > LIMIT || data.length < 4 || data[0] !== 77 || data[1] !== 71 || data[2] !== 86 || data[3] !== 49) {
    throw new Error('Invalid binary header or size')
  }
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength)
  let offset = 4
  let nodes = 0
  const need = (n: number) => {
    if (n > data.length - offset) throw new Error('Truncated binary value')
  }
  const count = () => {
    need(4)
    const n = view.getUint32(offset, true)
    offset += 4
    return n
  }
  // Fast path for the fixed 32-byte rows written by the photo adapter; any deviation rewinds to the generic walk.
  const triples = (n: number, kind: 'f64' | 'u8' | 'u32') => {
    if (n > (4_000_000 - nodes) / 4) throw new Error('Binary item limit')
    const start = offset
    if (n * 32 > data.length - offset) return undefined
    const out: Float64Array | Uint8Array | Uint32Array =
      kind === 'f64' ? new Float64Array(n * 3) : kind === 'u8' ? new Uint8Array(n * 3) : new Uint32Array(n * 3)
    for (let i = 0; i < n; i++) {
      if (data[offset] !== 5 || view.getUint32(offset + 1, true) !== 3) {
        offset = start
        return undefined
      }
      offset += 5
      for (let j = 0; j < 3; j++) {
        const tag = data[offset++]
        if (kind === 'f64') {
          if (tag === 3) {
            out[i * 3 + j] = view.getFloat64(offset, true)
            offset += 8
          } else if (tag === 0) {
            out[i * 3 + j] = NaN
          } else {
            offset = start
            return undefined
          }
        } else {
          if (tag !== 7) {
            offset = start
            return undefined
          }
          const low = view.getUint32(offset, true)
          const high = view.getUint32(offset + 4, true)
          offset += 8
          if (high !== 0 || (kind === 'u8' && low > 255)) {
            offset = start
            return undefined
          }
          out[i * 3 + j] = low
        }
      }
    }
    nodes += n * 4
    return out
  }
  const get = (depth: number, hint?: 'f64' | 'u8' | 'u32'): unknown => {
    if (depth > 128 || ++nodes > 4_000_000) throw new Error('Binary nesting or item limit')
    need(1)
    const tag = data[offset++]
    if (tag === 0) return null
    if (tag === 1) return false
    if (tag === 2) return true
    if (tag === 3 || tag === 7 || tag === 8) {
      need(8)
      const n =
        tag === 3
          ? view.getFloat64(offset, true)
          : view.getUint32(offset, true) + (tag === 7 ? view.getUint32(offset + 4, true) : view.getInt32(offset + 4, true)) * 4294967296
      offset += 8
      if (!Number.isFinite(n) || (tag !== 3 && !Number.isSafeInteger(n))) throw new Error('Invalid binary number')
      return n
    }
    if (tag === 4) {
      const n = count()
      need(n)
      const s = decoder.decode(data.subarray(offset, offset + n))
      offset += n
      return s
    }
    if (tag === 5 || tag === 6) {
      const n = count()
      if (n > data.length - offset || n > 4_000_000 - nodes) throw new Error('Binary item limit')
      if (tag === 5) {
        if (hint) {
          const flat = triples(n, hint)
          if (flat) return flat
        }
        const out = new Array<unknown>(n)
        for (let i = 0; i < n; i++) out[i] = get(depth + 1)
        return out
      }
      const out: Record<string, unknown> = {}
      for (let i = 0; i < n; i++) {
        const k = get(depth + 1)
        if (typeof k !== 'string' || Object.hasOwn(out, k)) throw new Error('Invalid or duplicate binary object key')
        const val = get(depth + 1, hints?.[k])
        if (k === '__proto__') {
          Object.defineProperty(out, k, { value: val, enumerable: true, writable: true, configurable: true })
        } else {
          out[k] = val
        }
      }
      return out
    }
    throw new Error('Invalid binary tag')
  }
  const value = get(0)
  if (offset !== data.length) throw new Error('Trailing binary data')
  return value
}
