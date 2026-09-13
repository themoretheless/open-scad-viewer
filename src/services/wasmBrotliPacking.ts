/** Bounded synchronous Brotli bootstrap for the generated geometry kernel. */
import decoderBytes from '../generated/wasm-brotli/bytes'
import {unpackWasmBase64} from './wasmPacking'

const inputLimit = 4 * 1024 * 1024
const outputLimit = 16 * 1024 * 1024
let decoderModule: WebAssembly.Module | undefined
interface Decoder extends WebAssembly.Exports {
  memory: WebAssembly.Memory
  prepare(size: number): number
  decode(size: number): bigint
  prepare_encoded(size: number): number
  decode_encoded(): bigint
}

/** 4-byte little-endian output length followed by one complete Brotli stream. */
export function unpackBrotliWasm(input: Uint8Array): Uint8Array<ArrayBuffer> {
  if (input.length <= 4) throw new Error('Truncated Brotli WASM package')
  if (input.length - 4 > inputLimit) throw new Error('Brotli WASM input limit')
  const size = new DataView(input.buffer, input.byteOffset, input.byteLength).getUint32(0, true)
  if (size > outputLimit) throw new Error('Brotli WASM output limit')
  decoderModule ??= new WebAssembly.Module(unpackWasmBase64(decoderBytes))
  // The bounded decoder memory is released with this temporary instance.
  const decoder = new WebAssembly.Instance(decoderModule).exports as Decoder
  const pointer = decoder.prepare(input.length - 4)
  if (!pointer) throw new Error('Brotli WASM input allocation failed')
  new Uint8Array(decoder.memory.buffer, pointer, input.length - 4).set(input.subarray(4))
  const packed = decoder.decode(size)
  if (!packed) throw new Error('Invalid Brotli WASM stream or package size mismatch')
  const resultPointer = Number(packed & 0xffffffffn), resultSize = Number(packed >> 32n)
  if (resultSize !== size) throw new Error('Brotli WASM package size mismatch')
  return new Uint8Array(decoder.memory.buffer, resultPointer, resultSize).slice()
}

export function unpackBrotliWasmBase64(packed: string): Uint8Array<ArrayBuffer> {
  // Tagged compact literals use Rust for both ASCII decoding and decompression.
  // The historical entry point still accepts legacy base64 packages.
  if (packed.startsWith('b85:')) {
    const limit = 5 + Math.ceil((inputLimit + 4) / 4) * 5
    if (packed.length - 4 > limit) throw new Error('Brotli WASM input limit')
    const input = new TextEncoder().encode(packed.slice(4))
    if (input.length > limit) throw new Error('Brotli WASM input limit')
    decoderModule ??= new WebAssembly.Module(unpackWasmBase64(decoderBytes))
    const decoder = new WebAssembly.Instance(decoderModule).exports as Decoder
    const pointer = decoder.prepare_encoded(input.length)
    if (!pointer) throw new Error('Brotli WASM input allocation failed')
    new Uint8Array(decoder.memory.buffer,pointer,input.length).set(input)
    const result = decoder.decode_encoded()
    if (!result) throw new Error('Invalid base85/Brotli WASM package')
    return new Uint8Array(decoder.memory.buffer,Number(result & 0xffffffffn),Number(result >> 32n)).slice()
  }
  if (packed.length > 4 * Math.ceil((inputLimit + 4) / 3)) throw new Error('Brotli WASM input limit')
  return unpackBrotliWasm(Uint8Array.from(atob(packed), character => character.charCodeAt(0)))
}
