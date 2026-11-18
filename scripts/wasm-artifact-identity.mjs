import {createHash} from 'node:crypto'

export function wasmArtifactIdentityModule(bytes, maximumBytes = 16 * 1024 * 1024) {
  if (!Number.isSafeInteger(maximumBytes) || maximumBytes < 8 || maximumBytes > 20 * 1024 * 1024
    || !(bytes instanceof Uint8Array) || bytes.byteLength < 8 || bytes.byteLength > maximumBytes) {
    throw new RangeError('WASM artifact exceeds its bounded byte limit')
  }
  const identity = {sha256: createHash('sha256').update(bytes).digest('hex'), byteLength: bytes.byteLength}
  return `// Generated from the packaged WASM; not qualification evidence.\nexport default Object.freeze(${JSON.stringify(identity)})\n`
}
