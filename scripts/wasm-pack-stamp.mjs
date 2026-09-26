import {createHash} from 'node:crypto'
import {existsSync, readFileSync, writeFileSync} from 'node:fs'

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')

/**
 * Stamp cache for wasm packaging (same idea as wasm-opt-cache.mjs): the stamp
 * file stores the sha256 of the source wasm that produced the outputs. A hit
 * skips brotli quality-11 compression, base encoding and WebAssembly.Module
 * compilation when the wasm is unchanged. Hashing detects corruption or
 * partial writes, not hostile authors.
 */

/** True when the stamp matches sourceBytes and every expected output exists. */
export function packStampHit(stampPath, sourceBytes, outputs) {
  try {
    if (readFileSync(stampPath, 'utf8').trim() !== sha256(sourceBytes)) return false
  } catch {
    return false
  }
  return outputs.every(output => existsSync(output))
}

/** Record the source hash after a full packaging run. */
export function writePackStamp(stampPath, sourceBytes) {
  writeFileSync(stampPath, `${sha256(sourceBytes)}\n`)
}
