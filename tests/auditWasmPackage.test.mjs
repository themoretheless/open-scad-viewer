import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import test from 'node:test'
import { auditWasmPackage } from '../scripts/audit-wasm-package.mjs'
import { verifyUniquePackedWasm, verifyRawWasm } from '../scripts/verify-packed-wasm.mjs'

test('binds streaming bytes exactly, rejecting equal-size drift and truncation', () => {
  const wasm = Buffer.from([0,97,115,109,1,0,0,0])
  expectMatching(wasm, wasm)
  const changed = Buffer.from(wasm)
  changed[4] = 2
  assert.throws(() => verifyRawWasm(changed, wasm, 'test'), /streaming WASM differs/)
  assert.throws(() => verifyRawWasm(wasm.subarray(1), wasm, 'test'), /streaming WASM differs/)
  // Respect view offsets rather than comparing the entire backing allocation.
  const backing = Buffer.concat([Buffer.from([255]), wasm, Buffer.from([255])])
  expectMatching(backing.subarray(1, -1), wasm)
  function expectMatching(raw, expected) {
    assert.equal(verifyRawWasm(raw, expected, 'test'), expected.length)
  }
})

test('rejects packed payload copies across chunks and within one chunk', () => {
  const asset = (path, source) => ({path, source})
  assert.equal(verifyUniquePackedWasm([asset('a.js', 'const a="b85:first";'), asset('b.js', "const b='b85:second';")]), 2)
  assert.throws(() => verifyUniquePackedWasm([
    asset('shared.js', 'export default "b85:first";'),
    asset('worker.js', 'compile(unpack(`b85:first`));'),
  ]), /Duplicate packed WASM literal: shared.js and worker.js/)
  assert.throws(() => verifyUniquePackedWasm([asset('a.js', 'f("b85:first");g("b85:first");')]), /Duplicate packed WASM/)
  assert.equal(verifyUniquePackedWasm([asset('a.js', 'import value from "./shared.js";')]), 0)
})

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
