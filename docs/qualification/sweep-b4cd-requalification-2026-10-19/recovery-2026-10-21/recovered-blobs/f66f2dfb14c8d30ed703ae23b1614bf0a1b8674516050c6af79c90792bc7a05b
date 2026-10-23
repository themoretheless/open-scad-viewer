import {sha256Hex} from '../core/sha256'

export interface WasmArtifactIdentity {
  readonly sha256: string
  readonly byteLength: number
}

export class WasmArtifactError extends Error {
  readonly code = 'WASM_ARTIFACT_MISMATCH'
  constructor(message: string) { super(message); this.name = 'WasmArtifactError' }
}

const verifiedModules = new WeakMap<WebAssembly.Module, WasmArtifactIdentity>()

export function snapshotWasmIdentity(identity: WasmArtifactIdentity): WasmArtifactIdentity {
  const {sha256, byteLength} = identity
  if (!/^[a-f0-9]{64}$/.test(sha256) || !Number.isSafeInteger(byteLength)
    || byteLength < 8 || byteLength > 16 * 1024 * 1024) {
    throw new WasmArtifactError('Invalid bounded WASM artifact identity')
  }
  return Object.freeze({sha256, byteLength})
}

function snapshotBytes(bytes: Uint8Array, identity: WasmArtifactIdentity): Uint8Array<ArrayBuffer> {
  if (bytes.byteLength !== identity.byteLength) throw new WasmArtifactError('WASM artifact length differs from the build')
  // The caller can mutate its input while WebCrypto or compilation is pending.
  return new Uint8Array(bytes)
}

function assertDigest(actual: string, identity: WasmArtifactIdentity): void {
  if (actual !== identity.sha256) throw new WasmArtifactError('WASM artifact SHA-256 differs from the build')
}

export function assertVerifiedWasmModule(module: WebAssembly.Module, identity: WasmArtifactIdentity): void {
  const observed = verifiedModules.get(module)
  if (!observed || observed.sha256 !== identity.sha256 || observed.byteLength !== identity.byteLength) {
    throw new WasmArtifactError('Host compiler returned an unverified or different WASM artifact')
  }
}

export async function compileWasmArtifact(bytes: Uint8Array, expected: WasmArtifactIdentity): Promise<WebAssembly.Module> {
  const identity = snapshotWasmIdentity(expected)
  const snapshot = snapshotBytes(bytes, identity)
  const digest = globalThis.crypto?.subtle
    ? Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', snapshot)), n => n.toString(16).padStart(2, '0')).join('')
    : sha256Hex(snapshot)
  assertDigest(digest, identity)
  const module = await WebAssembly.compile(snapshot)
  verifiedModules.set(module, identity)
  return module
}

export function compileWasmArtifactSync(bytes: Uint8Array, expected: WasmArtifactIdentity): WebAssembly.Module {
  const identity = snapshotWasmIdentity(expected)
  const snapshot = snapshotBytes(bytes, identity)
  assertDigest(sha256Hex(snapshot), identity)
  const module = new WebAssembly.Module(snapshot)
  verifiedModules.set(module, identity)
  return module
}
