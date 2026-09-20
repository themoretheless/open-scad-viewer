import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {afterEach, expect, it, vi} from 'vitest'
import {assertVerifiedWasmModule, compileWasmArtifact, compileWasmArtifactSync} from '../src/services/wasmArtifact'
import {compileOptionalWasm, setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {sha256Hex} from '../src/core/sha256'

const bytes = new Uint8Array([0,97,115,109,1,0,0,0])
const identity = {sha256: createHash('sha256').update(bytes).digest('hex'), byteLength: bytes.length}
afterEach(() => { setOptionalWasmCompiler(undefined); vi.unstubAllGlobals() })

it('attests the exact bytes with asynchronous and synchronous compilation', async () => {
  for (const module of [await compileWasmArtifact(bytes, identity), compileWasmArtifactSync(bytes, identity)]) {
    expect(() => assertVerifiedWasmModule(module, identity)).not.toThrow()
    expect(() => assertVerifiedWasmModule(module, {...identity, sha256: '0'.repeat(64)})).toThrow(/unverified or different/)
  }
})

it('rejects wrong digests, lengths and unbounded identities before instantiation', async () => {
  for (const wrong of [{...identity, sha256: '0'.repeat(64)}, {...identity, byteLength: 9},
    {...identity, byteLength: 16 * 1024 * 1024 + 1}, {...identity, byteLength: 7},
    {...identity, byteLength: NaN}, {...identity, sha256: 'invalid'}]) {
    await expect(compileWasmArtifact(bytes, wrong)).rejects.toMatchObject({code: 'WASM_ARTIFACT_MISMATCH'})
    expect(() => compileWasmArtifactSync(bytes, wrong)).toThrow()
  }
})

it('snapshots caller-owned bytes and identity before awaiting compilation', async () => {
  const mutableBytes = bytes.slice(), mutableIdentity = {...identity}
  const pending = compileWasmArtifact(mutableBytes, mutableIdentity)
  mutableBytes.fill(255)
  mutableIdentity.sha256 = '0'.repeat(64)
  const module = await pending
  expect(() => assertVerifiedWasmModule(module, identity)).not.toThrow()
})

it('supports hosts without WebCrypto using the existing SHA-256 implementation', async () => {
  vi.stubGlobal('crypto', undefined)
  const module = await compileWasmArtifact(bytes, identity)
  expect(() => assertVerifiedWasmModule(module, identity)).not.toThrow()
})

it('matches independent Node SHA-256 across padding boundaries and the 16 MiB limit', () => {
  for (const size of [0, 1, 55, 56, 63, 64, 65, 127, 128, 129, 4096, 16 * 1024 * 1024]) {
    const data = Uint8Array.from({length: size}, (_, i) => (i * 37 + size) % 256)
    expect(sha256Hex(data), `size ${size}`).toBe(createHash('sha256').update(data).digest('hex'))
  }
  expect(sha256Hex(bytes)).toBe(identity.sha256)
})

it('refuses unverified host modules even if they happen to contain matching bytes', async () => {
  setOptionalWasmCompiler(async () => new WebAssembly.Module(bytes))
  await expect(compileOptionalWasm('/kernel', identity)).rejects.toMatchObject({code: 'WASM_ARTIFACT_MISMATCH'})
  setOptionalWasmCompiler(async (_url, expected) => {
    expect(Object.isFrozen(expected)).toBe(true)
    return compileWasmArtifact(bytes, expected!)
  })
  await expect(compileOptionalWasm('/kernel', identity)).resolves.toBeInstanceOf(WebAssembly.Module)
  setOptionalWasmCompiler(async () => null)
  await expect(compileOptionalWasm('/kernel', identity)).resolves.toBeNull()
})

it('rejects a verified module from a different artifact', async () => {
  const other = new Uint8Array([...bytes, 0, 2, 1, 120])
  const module = await compileWasmArtifact(other, {
    byteLength: other.length, sha256: createHash('sha256').update(other).digest('hex'),
  })
  setOptionalWasmCompiler(async () => module)
  await expect(compileOptionalWasm('/kernel', identity)).rejects.toMatchObject({code: 'WASM_ARTIFACT_MISMATCH'})
})

it('binds generated geometry and language identities to packaged bytes', async () => {
  const geometry = (await import('../src/generated/geometry-kernels/identity')).default
  const language = (await import('../src/generated/language-kernel/identity')).default
  for (const [name, expected] of [['geometry', geometry], ['language', language]] as const) {
    const artifact = readFileSync(new URL(`../public/wasm/${name}-kernel.wasm`, import.meta.url))
    expect(expected).toEqual({byteLength: artifact.length, sha256: createHash('sha256').update(artifact).digest('hex')})
    expect(Object.isFrozen(expected)).toBe(true)
  }
})

it('keeps the real geometry kernel uninitialized on an unverified hook and can recover', async () => {
  vi.resetModules()
  const host = await import('../src/services/wasmCompilation')
  const kernel = await import('../src/services/geometry/kernel')
  host.setOptionalWasmCompiler(async () => new WebAssembly.Module(bytes))
  try {
    await expect(kernel.warmGeometryKernel()).rejects.toMatchObject({code: 'WASM_ARTIFACT_MISMATCH'})
    expect(kernel.isGeometryKernelReady()).toBe(false)
    host.setOptionalWasmCompiler(undefined)
    await kernel.warmGeometryKernel()
    expect(kernel.isGeometryKernelReady()).toBe(true)
    const {parseGcodePreview} = await import('../src/services/geometry/polygon')
    expect(parseGcodePreview('G1 X0 Y0 Z0\nM83\nG1 X1 E1\n').extrusionMm).toBe(1)
  } finally { host.setOptionalWasmCompiler(undefined) }
})
