import {readFileSync} from 'node:fs'
import {brotliCompressSync, deflateRawSync} from 'node:zlib'
import {describe, expect, it} from 'vitest'
import packedHarfBuzz from '../src/generated/harfbuzz/bytes'
import {unpackBrotliWasmBase64} from '../src/services/wasmBrotliPacking'
import {verifyPackedWasmChunk} from '../scripts/verify-packed-wasm.mjs'

const original = readFileSync(new URL('../node_modules/harfbuzzjs/hb.wasm', import.meta.url))
const chunk = (packed: string) => `var wasm="${packed}";export{wasm as default};`

describe('lossless packed HarfBuzz runtime', () => {
  it('decodes every installed HarfBuzz byte through the production WASM decoder', () => {
    const decoded = unpackBrotliWasmBase64(packedHarfBuzz)
    expect(Buffer.from(decoded).equals(original)).toBe(true)
    const module = new WebAssembly.Module(decoded)
    expect(WebAssembly.Module.exports(module)).toEqual(WebAssembly.Module.exports(new WebAssembly.Module(original)))
    expect(verifyPackedWasmChunk(chunk(packedHarfBuzz), original, 'HarfBuzz')).toBe(original.length)
  })

  it('rejects absent, duplicate, stale and corrupt emitted packages', () => {
    expect(() => verifyPackedWasmChunk('export default "";', original, 'HarfBuzz')).toThrow()
    expect(() => verifyPackedWasmChunk(chunk(packedHarfBuzz).repeat(2), original, 'HarfBuzz')).toThrow()
    const stale = Buffer.from(original)
    stale[stale.length - 1] ^= 1
    expect(() => verifyPackedWasmChunk(chunk(packedHarfBuzz), stale, 'HarfBuzz')).toThrow(/differs from the original/)
    const wrongSize = Buffer.from(packedHarfBuzz, 'base64')
    wrongSize.writeUInt32LE(original.length + 1)
    expect(() => verifyPackedWasmChunk(chunk(wrongSize.toString('base64')), original, 'HarfBuzz')).toThrow(/size mismatch/)
    const trailing = Buffer.concat([Buffer.from(packedHarfBuzz, 'base64'), Buffer.from([0, 0])])
    expect(() => verifyPackedWasmChunk(chunk(trailing.toString('base64')), original, 'HarfBuzz')).toThrow(/trailing/)
    const invalid = Buffer.from(original)
    invalid[0] = 1
    const header = Buffer.alloc(4)
    header.writeUInt32LE(invalid.length)
    const invalidPackage = Buffer.concat([header, brotliCompressSync(invalid)])
    expect(() => verifyPackedWasmChunk(chunk(invalidPackage.toString('base64')), invalid, 'HarfBuzz')).toThrow(/invalid WASM header/)
  })

  it('strictly verifies the DEFLATE bootstrap representation as well', () => {
    const header = Buffer.alloc(4)
    header.writeUInt32LE(original.length)
    const packed = Buffer.concat([header, deflateRawSync(original)])
    expect(verifyPackedWasmChunk(chunk(packed.toString('base64')), original, 'bootstrap', 'deflate')).toBe(original.length)
    expect(() => verifyPackedWasmChunk(chunk(Buffer.concat([packed, Buffer.from([0])]).toString('base64')), original, 'bootstrap', 'deflate')).toThrow(/trailing/)
    expect(() => verifyPackedWasmChunk(chunk(packed.toString('base64')), original, 'bootstrap', 'brotli')).toThrow()
  })
})
