import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import binaryen from 'binaryen'
import { optimizeWasm } from '../scripts/wasm-optimize.mjs'

test('repeated optimization preserves Cargo input and produces identical runnable bytes', () => {
  const directory = mkdtempSync(join(tmpdir(), 'wasm-opt-test-'))
  const module = new binaryen.Module()
  try {
    module.addFunction('answer', binaryen.none, binaryen.i32, [], module.i32.add(module.i32.const(20), module.i32.const(22)))
    module.addFunctionExport('answer', 'answer')
    const original = Buffer.from(module.emitBinary())
    const input = join(directory, 'cargo.wasm')
    writeFileSync(input, original)
    const first = optimizeWasm(input)
    assert.deepEqual(readFileSync(input), original)
    // Exercise Binaryen twice independently, not just a cache hit.
    rmSync(join(directory, '.osv-wasm-opt-cache'), { recursive: true })
    const second = optimizeWasm(input)
    assert.deepEqual(readFileSync(input), original)
    assert.deepEqual(first, second)
    assert.deepEqual(optimizeWasm(input), second)
    assert.equal(new WebAssembly.Instance(new WebAssembly.Module(first)).exports.answer(), 42)
    assert.ok(first.length < original.length)
  } finally {
    module.dispose()
    rmSync(directory, { recursive: true, force: true })
  }
})

test('optimizer failure is catchable and does not alter the input', () => {
  const directory = mkdtempSync(join(tmpdir(), 'wasm-opt-invalid-'))
  try {
    const input = join(directory, 'invalid.wasm')
    writeFileSync(input, 'invalid module')
    assert.throws(() => optimizeWasm(input), /wasm-opt failed/)
    assert.equal(readFileSync(input, 'utf8'), 'invalid module')
  } finally { rmSync(directory, { recursive: true, force: true }) }
})
