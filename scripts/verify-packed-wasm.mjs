import {brotliDecompressSync, inflateRawSync} from 'node:zlib'
import {decodeBase85} from './wasm-base85.mjs'
import {createHash} from 'node:crypto'

export function verifyRawWasm(raw, expected, label) {
  if (!Buffer.from(raw.buffer, raw.byteOffset, raw.byteLength).equals(expected)) {
    throw new Error(`${label}: streaming WASM differs from the original`)
  }
  return raw.byteLength
}

/** Inspect emitted strings: source maps do not attribute inlined payload copies. */
export function verifyUniquePackedWasm(assets) {
  const owners = new Map()
  for (const {path, source} of assets) {
    for (const match of source.matchAll(/(["'`])(b85:[^"'`\\\r\n]+)\1/g)) {
      const hash = createHash('sha256').update(match[2]).digest('hex')
      if (owners.has(hash)) throw new Error(`Duplicate packed WASM literal: ${owners.get(hash)} and ${path}`)
      owners.set(hash, path)
    }
  }
  return owners.size
}

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
