import {describe, expect, it} from 'vitest'
import {deflateRawSync} from 'node:zlib'
import {unpackWasm} from '../src/services/wasmPacking'

const lengthBase = [3, 4, 5, 6, 7, 8, 9, 10]
const distanceBase = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33]

/** Packs a raw fixed-Huffman DEFLATE stream with explicit LZ matches (no extra bits). */
function fixedPacked(size: number, tokens: (number | {length: number; distance: number})[]): Uint8Array {
  const bits: number[] = []
  const lsb = (value: number, count: number) => { for (let j = 0; j < count; j++) bits.push(value >>> j & 1) }
  const code = (value: number, count: number) => { for (let j = count - 1; j >= 0; j--) bits.push(value >>> j & 1) }
  lsb(1, 1)
  lsb(1, 2)
  for (const token of tokens) {
    if (typeof token === 'number') {
      code(token < 144 ? 0x30 + token : 0x190 + token - 144, token < 144 ? 8 : 9)
      continue
    }
    code(lengthBase.indexOf(token.length) + 1, 7)
    code(distanceBase.indexOf(token.distance), 5)
  }
  code(0, 7)
  const packed = new Uint8Array(4 + Math.ceil(bits.length / 8))
  new DataView(packed.buffer).setUint32(0, size, true)
  bits.forEach((bit, index) => { if (bit) packed[4 + (index >>> 3)]! |= 1 << (index & 7) })
  return packed
}

describe('DEFLATE match copying', () => {
  it('replicates overlapping match patterns byte for byte', () => {
    expect(unpackWasm(fixedPacked(6, [65, {length: 5, distance: 1}])))
      .toEqual(Uint8Array.from([65, 65, 65, 65, 65, 65]))
    expect(unpackWasm(fixedPacked(12, [97, 98, 99, {length: 9, distance: 3}])))
      .toEqual(Uint8Array.from([97, 98, 99, 97, 98, 99, 97, 98, 99, 97, 98, 99]))
    expect(unpackWasm(fixedPacked(9, [1, 2, 3, 4, {length: 5, distance: 2}])))
      .toEqual(Uint8Array.from([1, 2, 3, 4, 3, 4, 3, 4, 3]))
  })

  it('round-trips zlib streams whose matches overlap at every small distance', () => {
    for (const period of [1, 2, 3, 4, 5, 6, 7, 64, 4096]) {
      const data = Uint8Array.from({length: 5000}, (_, i) => i % period)
      const header = Buffer.alloc(4)
      header.writeUInt32LE(data.length)
      expect(unpackWasm(Buffer.concat([header, deflateRawSync(data, {level: 9})]))).toEqual(data)
    }
  })
})
