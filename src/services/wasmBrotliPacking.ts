/** Bounded synchronous Brotli bootstrap for the generated geometry kernel. */
import {decodeBase91} from './wasmBase91'
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

/** Legacy single stream, or high-bit length + bounded stream table. */
export function unpackBrotliWasm(input: Uint8Array): Uint8Array<ArrayBuffer> {
  if (input.length <= 4) throw new Error('Truncated Brotli WASM package')
  if (input.length - 4 > inputLimit) throw new Error('Brotli WASM input limit')
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const word = view.getUint32(0, true), multiple = !!(word & 0x80000000)
  const size = word & 0x7fffffff
  if (size > outputLimit) throw new Error('Brotli WASM output limit')
  const streams: {size: number; start: number; end: number}[] = []
  if (multiple) {
    const count = input[4]
    if (!count || count > 8 || input.length <= 5 + count * 8) throw new Error('Invalid Brotli WASM stream table')
    let start = 5 + count * 8, total = 0
    for (let i = 0; i < count; i++) {
      const length = view.getUint32(5 + i * 8, true), compressed = view.getUint32(9 + i * 8, true)
      const end = start + compressed
      total += length
      if (!length || !compressed || total > size || end > input.length) throw new Error('Invalid Brotli WASM stream size')
      streams.push({size: length, start, end}); start = end
    }
    if (total !== size || start !== input.length) throw new Error('Brotli WASM package size mismatch')
  } else streams.push({size, start: 4, end: input.length})
  decoderModule ??= new WebAssembly.Module(unpackWasmBase64(decoderBytes))
  const output = new Uint8Array(size)
  let offset = 0
  for (const stream of streams) {
    // Temporary bounded memory per stream; retain only the owned final output.
    const decoder = new WebAssembly.Instance(decoderModule).exports as Decoder
    const length = stream.end - stream.start, pointer = decoder.prepare(length)
    if (!pointer) throw new Error('Brotli WASM input allocation failed')
    new Uint8Array(decoder.memory.buffer, pointer, length).set(input.subarray(stream.start, stream.end))
    const packed = decoder.decode(stream.size)
    if (!packed) throw new Error('Invalid Brotli WASM stream or package size mismatch')
    const resultPointer = Number(packed & 0xffffffffn), resultSize = Number(packed >> 32n)
    if (resultSize !== stream.size) throw new Error('Brotli WASM package size mismatch')
    output.set(new Uint8Array(decoder.memory.buffer, resultPointer, resultSize), offset)
    offset += resultSize
  }
  return output
}

export function unpackBrotliWasmBase64(packed: string): Uint8Array<ArrayBuffer> {
  if((packed.startsWith('b91:')||packed.startsWith('b9v:')||packed.startsWith('b92:')||packed.startsWith('b93:')||packed.startsWith('b94:')||packed.startsWith('b95:')))return unpackBrotliWasm(decodeBase91(packed,inputLimit+4))
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
