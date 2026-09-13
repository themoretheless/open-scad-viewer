import {readFileSync} from 'node:fs'
import {brotliCompressSync, constants} from 'node:zlib'
import {describe, expect, it} from 'vitest'
import geometryBytes from '../src/generated/geometry-kernels/bytes'
import decoderBytes from '../src/generated/wasm-brotli/bytes'
import photogrammetryBytes from '../src/generated/photogrammetry/bytes'
import {unpackWasmBase64} from '../src/services/wasmPacking'
import {unpackBrotliWasm, unpackBrotliWasmBase64} from '../src/services/wasmBrotliPacking'
import {encodeBase85, decodeBase85} from '../scripts/wasm-base85.mjs'
import {verifyPackedWasmChunk} from '../scripts/verify-packed-wasm.mjs'

function pack(data: Uint8Array, quality = 11): Uint8Array {
  const header = Buffer.alloc(4)
  header.writeUInt32LE(data.length)
  return Buffer.concat([header, brotliCompressSync(data, {params: {[constants.BROTLI_PARAM_QUALITY]: quality}})])
}

describe('synchronous Brotli WASM packages', () => {
  it('round-trips compact literals through both build-time and Rust decoders', () => {
    for (let length = 0; length < 100; length++) {
      const data = Uint8Array.from({length}, (_, i) => (i * 97 + length) & 255)
      expect(decodeBase85(encodeBase85(data))).toEqual(data)
      const packageBytes = pack(data, length % 12)
      const text = encodeBase85(packageBytes)
      expect(decodeBase85(text)).toEqual(new Uint8Array(packageBytes))
      expect(unpackBrotliWasmBase64(text)).toEqual(data)
    }
  })

  it('rejects noncanonical base85 words, padding, lengths and excess output', () => {
    const bytes = pack(new Uint8Array())
    const encoded = encodeBase85(bytes)
    const padded = new Uint8Array(Math.ceil(bytes.length / 4) * 4)
    padded.set(bytes); padded[padded.length - 1] = 1
    const noncanonical = encoded.slice(0,9) + encodeBase85(padded).slice(9)
    for (const invalid of [noncanonical, encoded.slice(0,-1), encoded+'00000', 'b85:00005#####00000', 'b85:000050000~00000']) {
      expect(() => decodeBase85(invalid)).toThrow()
      expect(() => unpackBrotliWasmBase64(invalid)).toThrow()
    }
    const excessive = bytes.slice()
    new DataView(excessive.buffer,excessive.byteOffset,excessive.byteLength).setUint32(0,16 * 1024 * 1024 + 1,true)
    expect(() => unpackBrotliWasmBase64(encodeBase85(excessive))).toThrow()
    expect(() => unpackBrotliWasmBase64('b85:'+'0'.repeat(6 + Math.ceil((4*1024*1024+4)/4)*5))).toThrow(/input limit/)
  })

  it('round-trips reference-encoded dictionary, overlapping and binary streams', () => {
    const samples = [
      new Uint8Array(),
      new TextEncoder().encode('Compression transformation dictionary information international. '.repeat(400)),
      new Uint8Array(65536),
      Uint8Array.from({length: 65536}, (_, i) => (i * 17 + i % 7) & 255),
    ]
    for (const data of samples) for (const quality of [0, 5, 11]) {
      expect(unpackBrotliWasm(pack(data, quality))).toEqual(data)
    }
  })

  it('decodes the exact generated kernel bytes and preserves its exports', () => {
    const original = readFileSync(new URL('../src/generated/geometry-kernels/kernel_bg.wasm', import.meta.url))
    const decoded = unpackBrotliWasmBase64(geometryBytes)
    expect(Buffer.compare(Buffer.from(decoded), original)).toBe(0)
    const module = new WebAssembly.Module(decoded)
    expect(WebAssembly.Module.imports(module)).toEqual([])
    expect(WebAssembly.Module.exports(module)).toEqual(WebAssembly.Module.exports(new WebAssembly.Module(original)))
    expect(WebAssembly.Module.exports(module).some(entry => entry.name === 'abi_request')).toBe(true)
    const literal = `export default '${geometryBytes}'`
    expect(verifyPackedWasmChunk(literal,original,'geometry')).toBe(original.length)
    expect(() => verifyPackedWasmChunk(literal+literal,original,'geometry')).toThrow(/exactly one/)
  })

  it('rejects truncated, trailing, concatenated and incorrectly sized packages', () => {
    const data = new TextEncoder().encode('A complete Brotli stream. '.repeat(100))
    const packed = pack(data)
    for (const invalid of [packed.subarray(0, 4), packed.subarray(0, -1), packed.subarray(0, packed.length / 2), Buffer.concat([packed, Buffer.from([0])]), Buffer.concat([packed, packed.subarray(4)])]) {
      expect(() => unpackBrotliWasm(invalid)).toThrow()
    }
    for (const size of [data.length - 1, data.length + 1]) {
      const invalid = packed.slice()
      new DataView(invalid.buffer, invalid.byteOffset, invalid.byteLength).setUint32(0, size, true)
      expect(() => unpackBrotliWasm(invalid)).toThrow(/stream|size/i)
    }
    expect(() => unpackBrotliWasm(Uint8Array.from([0, 0, 0, 0, 255, 255, 255]))).toThrow()
  })

  it('checks host input/output caps and enforces the decoder memory maximum', () => {
    expect(() => unpackBrotliWasm(new Uint8Array(4 * 1024 * 1024 + 5))).toThrow(/input limit/)
    const outputOverflow = pack(new Uint8Array())
    new DataView(outputOverflow.buffer, outputOverflow.byteOffset, outputOverflow.byteLength).setUint32(0, 16 * 1024 * 1024 + 1, true)
    expect(() => unpackBrotliWasm(outputOverflow)).toThrow(/output limit/)
    expect(() => unpackBrotliWasmBase64('A'.repeat(4 * Math.ceil((4 * 1024 * 1024 + 4) / 3) + 1))).toThrow(/input limit/)
    const module = new WebAssembly.Module(unpackWasmBase64(decoderBytes))
    expect(WebAssembly.Module.imports(module)).toEqual([])
    const runtime = new WebAssembly.Instance(module).exports as {memory: WebAssembly.Memory; prepare(size: number): number; decode(size: number): bigint}
    expect(runtime.prepare(4 * 1024 * 1024 + 1)).toBe(0)
    expect(runtime.decode(16 * 1024 * 1024 + 1)).toBe(0n)
    expect(() => runtime.memory.grow(1025 - runtime.memory.buffer.byteLength / 65536)).toThrow(RangeError)
  })

  it('returns owned bytes that survive later decoder instances', () => {
    const first = unpackBrotliWasm(pack(Uint8Array.from([1, 2, 3])))
    expect(unpackBrotliWasm(pack(Uint8Array.from([4, 5])))).toEqual(Uint8Array.from([4, 5]))
    expect(first).toEqual(Uint8Array.from([1, 2, 3]))
  })

  it('decodes photogrammetry to its original WASM artifact without changing the math kernel', () => {
    const original = readFileSync(new URL('../crates/target/wasm32-unknown-unknown/release/photogrammetry_wasm.wasm', import.meta.url))
    const decoded = unpackBrotliWasmBase64(photogrammetryBytes)
    expect(Buffer.compare(Buffer.from(decoded), original)).toBe(0)
    expect(WebAssembly.Module.imports(new WebAssembly.Module(decoded))).toEqual([])
  })
})
