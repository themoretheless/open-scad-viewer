import assert from 'node:assert/strict'
import { readFileSync, mkdirSync } from 'node:fs'
import { encodeBinary, decodeBinary } from '../../src/core/valueBinaryCodec.ts'
/** Decoded ABI payload; its shape is asserted at runtime by this script. */
export type Wire = ReturnType<typeof JSON.parse>
export const [wasmPath, directory, sourceCommit] = process.argv.slice(2)
assert(wasmPath && directory && /^[a-f0-9]{40}$/.test(sourceCommit ?? ''))
mkdirSync(directory, { recursive: true })
export const bytes = readFileSync(wasmPath)
const module = await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module), [])
const instance = await WebAssembly.instantiate(module, {})
const w = instance.exports as unknown as {
  memory: WebAssembly.Memory
  abi_alloc(n: number): number
  abi_free(p: number, n: number): void
  abi_request(op: number, p: number, n: number): bigint
}
export const cases: object[] = []
export function request(v: unknown): Wire {
  const data = encodeBinary(v),
    original = data.slice(),
    ptr = w.abi_alloc(data.length)
  assert(ptr)
  new Uint8Array(w.memory.buffer, ptr, data.length).set(data)
  try {
    const packed = w.abi_request(0, ptr, data.length),
      p = Number(packed & 0xffffffffn),
      n = Number(packed >> 32n)
    try {
      const result = decodeBinary(new Uint8Array(w.memory.buffer, p, n))
      assert.deepEqual(new Uint8Array(w.memory.buffer, ptr, data.length), original)
      return result
    } finally {
      w.abi_free(p, n)
    }
  } finally {
    w.abi_free(ptr, data.length)
  }
}

export function reject(name: string, v: object): void {
  const start = performance.now(),
    reply = request(v)
  assert.equal(reply.ok, false, `${name}: ${JSON.stringify(reply)}`)
  assert.equal(typeof reply.error.code, 'string')
  cases.push({ name, error: reply.error.code, elapsedMs: performance.now() - start })
}
