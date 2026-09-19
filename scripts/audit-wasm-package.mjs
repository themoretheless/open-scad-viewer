import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { parseArgs } from 'node:util'
import { pathToFileURL } from 'node:url'
import { brotliCompressSync, brotliDecompressSync, constants } from 'node:zlib'
import { encodeBase85, decodeBase85 } from './wasm-base85.mjs'

const MAX_WASM_BYTES = 16 * 1024 * 1024
const MAX_COMPRESSED_BYTES = 4 * 1024 * 1024
const hash = bytes => createHash('sha256').update(bytes).digest('hex')

/** Delivery-size evidence only; matching signatures do not prove semantic equivalence. */
export function auditWasmPackage(bytes) {
  if (bytes.length > MAX_WASM_BYTES) throw new Error('WASM exceeds decompression output limit')
  const module = new WebAssembly.Module(bytes)
  const imports = WebAssembly.Module.imports(module)
  if (imports.length) throw new Error('Kernel WASM must not import external functions')
  const compressed = brotliCompressSync(bytes, { params: {
    [constants.BROTLI_PARAM_QUALITY]: 11,
    [constants.BROTLI_PARAM_LGWIN]: 24,
  } })
  if (compressed.length > MAX_COMPRESSED_BYTES) throw new Error('WASM exceeds compressed input limit')
  const header = Buffer.alloc(4)
  header.writeUInt32LE(bytes.length)
  const packed = encodeBase85(Buffer.concat([header, compressed]))
  const decoded = Buffer.from(decodeBase85(packed))
  assert.equal(decoded.readUInt32LE(0), bytes.length)
  assert.deepEqual(brotliDecompressSync(decoded.subarray(4)), Buffer.from(bytes))
  return {
    wasmBytes: bytes.length, wasmSha256: hash(bytes),
    brotliBytes: compressed.length, brotliSha256: hash(compressed),
    base85Characters: packed.length, base85Sha256: hash(packed),
    imports, exports: WebAssembly.Module.exports(module).sort((a, b) => a.name.localeCompare(b.name)),
    roundTripExact: true,
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const { values, positionals } = parseArgs({ allowPositionals: true, options: { out: { type: 'string' } } })
  assert(positionals.length >= 1 && positionals.length <= 4, 'Supply 1..4 WASM paths; the first is the baseline')
  const artifacts = positionals.map(path => {
    if (statSync(path).size > MAX_WASM_BYTES) throw new Error(`${path} exceeds decompression output limit`)
    return { path, ...auditWasmPackage(readFileSync(path)) }
  })
  const baseline = artifacts[0]
  const report = {
    node: process.version, zlib: process.versions.zlib, brotli: process.versions.brotli,
    scope: 'Production quality=11/window=24 Brotli+length header+Base85, excluding JS wrapper. Signature comparison covers export names/kinds, not full ABI types or geometry correctness.',
    artifacts: artifacts.map(artifact => ({ ...artifact,
      sameExportNamesAndKinds: JSON.stringify(artifact.exports) === JSON.stringify(baseline.exports),
      wasmBytesSaved: baseline.wasmBytes - artifact.wasmBytes,
      base85CharactersSaved: baseline.base85Characters - artifact.base85Characters,
    })),
  }
  if (values.out) {
    mkdirSync(dirname(resolve(values.out)), { recursive: true })
    writeFileSync(values.out, JSON.stringify(report, null, 2))
  }
  console.log(JSON.stringify(report, null, 2))
}
