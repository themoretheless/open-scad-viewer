import {brotliDecompressSync, inflateRawSync} from 'node:zlib'
import {decodeBase85} from './wasm-base85.mjs'

/** Validate the emitted literal without executing generated application code. */
export function verifyPackedWasmChunk(source, expected, label, compression = 'brotli') {
  if (compression !== 'brotli' && compression !== 'deflate') throw new Error(`${label}: unsupported packing format`)
  const literals = [...source.matchAll(/(["'`])(b85:[^"'`\\\r\n]+|[A-Za-z0-9+/]{64,}={0,2})\1/g)]
  if (literals.length !== 1) throw new Error(`${label}: expected exactly one packed WASM literal`)
  const literal = literals[0][2]
  const packed = literal.startsWith('b85:') ? Buffer.from(decodeBase85(literal)) : Buffer.from(literal, 'base64')
  if (!literal.startsWith('b85:') && packed.toString('base64') !== literal) throw new Error(`${label}: invalid base64`)
  if (packed.length <= 4 || packed.length - 4 > 4 * 1024 * 1024) throw new Error(`${label}: compressed size limit`)
  const size = packed.readUInt32LE(0)
  if (size > 16 * 1024 * 1024 || size !== expected.length) throw new Error(`${label}: decoded size mismatch`)
  const compressed = packed.subarray(4)
  const unpack = compression === 'brotli' ? brotliDecompressSync : inflateRawSync
  const {buffer: decoded, engine} = unpack(compressed, {maxOutputLength: 16 * 1024 * 1024, info: true})
  if (engine.bytesWritten !== compressed.length) throw new Error(`${label}: trailing compressed data`)
  if (decoded.length !== size) throw new Error(`${label}: decoded size mismatch`)
  if (!decoded.subarray(0, 8).equals(Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]))) throw new Error(`${label}: invalid WASM header`)
  new WebAssembly.Module(decoded)
  if (!decoded.equals(expected)) throw new Error(`${label}: emitted WASM differs from the original`)
  return decoded.length
}
