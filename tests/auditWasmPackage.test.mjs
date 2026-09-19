import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import test from 'node:test'
import { auditWasmPackage } from '../scripts/audit-wasm-package.mjs'

test('packs a valid module with an exact roundtrip and stable fingerprints', () => {
  const wasm = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])
  const report = auditWasmPackage(wasm)
  assert.equal(report.wasmBytes, 8)
  assert.equal(report.wasmSha256, createHash('sha256').update(wasm).digest('hex'))
  assert.equal(report.roundTripExact, true)
  assert.deepEqual(report.exports, [])
  assert.deepEqual(report.imports, [])
  assert.deepEqual(auditWasmPackage(wasm), report)
  assert.deepEqual(auditWasmPackage(Uint8Array.from(wasm)), report)
  assert.equal(report.base85Characters, 9 + Math.ceil((4 + report.brotliBytes) / 4) * 5)
})

test('rejects malformed modules and oversized raw input', () => {
  assert.throws(() => auditWasmPackage(Buffer.from('not wasm')), WebAssembly.CompileError)
  assert.throws(() => auditWasmPackage(Buffer.alloc(16 * 1024 * 1024 + 1)), /output limit/)
})

test('rejects modules with external imports', () => {
  // One function import m.f of type () -> (), with no definitions or exports.
  const wasm = Buffer.from([0,97,115,109,1,0,0,0,1,4,1,96,0,0,2,7,1,1,109,1,102,0,0])
  assert.throws(() => auditWasmPackage(wasm), /must not import/)
})
