import {describe, expect, it} from 'vitest'
import {decodeBinary} from '../src/services/valueBinaryCodec'

/** Counterpart of the Rust photo adapter's byte-small/item-oversized regression. */
describe('photo transport uses the existing MGV1 decoder limits', () => {
  it('rejects a payload below 32 MiB when its item count exceeds the shared limit', () => {
    const items = 4_000_000
    const bytes = new Uint8Array(4 + 5 + items)
    bytes.set([77, 71, 86, 49, 5]) // MGV1 followed by an array; zero bytes encode null.
    new DataView(bytes.buffer).setUint32(5, items, true)
    expect(bytes.byteLength).toBeLessThan(32 * 1024 * 1024)
    expect(() => decodeBinary(bytes)).toThrow(/item limit/)
  })
})
