import {createHash} from 'node:crypto'

export function wasmArtifactIdentityModule(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.byteLength < 8 || bytes.byteLength > 16 * 1024 * 1024) {
    throw new RangeError('WASM artifact must contain 8 bytes to 16 MiB')
  }
  const identity = {sha256: createHash('sha256').update(bytes).digest('hex'), byteLength: bytes.byteLength}
  return `// Generated from the packaged WASM; not qualification evidence.\nexport default Object.freeze(${JSON.stringify(identity)})\n`
}
