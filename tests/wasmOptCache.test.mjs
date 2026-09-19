import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, readdirSync, rmSync, truncateSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { createHash } from 'node:crypto'
import { cachedWasmOptimization } from '../scripts/wasm-opt-cache.mjs'
import { Worker } from 'node:worker_threads'
import { once } from 'node:events'

const wasm = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])
function fixture(run) {
  const directory = mkdtempSync(join(tmpdir(), 'wasm-cache-test-'))
  let calls = 0
  const options = { input: wasm, tool: Buffer.from('tool'), flags: ['-Oz'], cacheDir: directory,
    optimize: () => { calls++; return Buffer.from(wasm) } }
  try { run(options, () => calls) } finally { rmSync(directory, { recursive: true, force: true }) }
}
test('reuses validated bytes and invalidates input, tool and flags independently', () => fixture((options, calls) => {
  cachedWasmOptimization(options)
  assert.deepEqual(cachedWasmOptimization(options), wasm)
  assert.equal(calls(), 1)
  for (const changed of [{ input: Buffer.from('different input') }, { tool: Buffer.from('new tool') }, { flags: ['-O3'] }]) {
    cachedWasmOptimization({ ...options, ...changed })
  }
  assert.equal(calls(), 4)
}))
test('damaged or mismatched entries are replaced without trusting their bytes', () => fixture((options, calls) => {
  cachedWasmOptimization(options)
  const file = join(options.cacheDir, readdirSync(options.cacheDir)[0])
  const valid = JSON.parse(readFileSync(file, 'utf8'))
  const invalid = Buffer.from('not wasm')
  for (const entry of ['not JSON', JSON.stringify({ ...valid, key: 'wrong' }), JSON.stringify({ ...valid, bytes: 'AAAA' }),
    JSON.stringify({ ...valid, bytes: invalid.toString('base64'), sha256: createHash('sha256').update(invalid).digest('hex') })]) {
    writeFileSync(file, entry)
    assert.deepEqual(cachedWasmOptimization(options), wasm)
  }
  assert.equal(calls(), 5)
  assert.ok(readdirSync(options.cacheDir).every(name => name.endsWith('.json')))
}))
test('does not cache optimizer failure or invalid output', () => fixture(options => {
  assert.throws(() => cachedWasmOptimization({ ...options, optimize: () => { throw Error('failed') } }), /failed/)
  assert.throws(() => cachedWasmOptimization({ ...options, optimize: () => Buffer.from('invalid') }), /invalid module/)
  assert.deepEqual(readdirSync(options.cacheDir), [])
}))
test('unwritable cache location does not discard a valid optimization', () => fixture(options => {
  const cacheDir = join(options.cacheDir, 'file')
  writeFileSync(cacheDir, 'not a directory')
  assert.deepEqual(cachedWasmOptimization({ ...options, cacheDir }), wasm)
}))

test('oversized cache entry is a miss and is repaired', () => fixture((options, calls) => {
  cachedWasmOptimization(options)
  const file = join(options.cacheDir, readdirSync(options.cacheDir)[0])
  truncateSync(file, 32 * 1024 * 1024 + 1)
  assert.deepEqual(cachedWasmOptimization(options), wasm)
  assert.equal(calls(), 2)
  assert.deepEqual(cachedWasmOptimization(options), wasm)
  assert.equal(calls(), 2)
}))

test('concurrent misses publish one complete entry without scratch leftovers', { timeout: 10000 }, async () => {
  const cacheDir = mkdtempSync(join(tmpdir(), 'wasm-cache-concurrent-'))
  const barrier = new SharedArrayBuffer(4)
  const workers = []
  try {
    const ready = [], exits = []
    for (let index = 0; index < 2; index++) {
      const worker = new Worker(`
        const {workerData, parentPort} = require('node:worker_threads');
        const assert = require('node:assert/strict');
        import(workerData.module).then(({cachedWasmOptimization}) => {
          const wasm = Buffer.from([0,97,115,109,1,0,0,0]);
          const bytes = cachedWasmOptimization({input:wasm, tool:Buffer.from('tool'), flags:['-Oz'],
            cacheDir:workerData.cacheDir, optimize:() => {
              parentPort.postMessage('miss');
              const gate = new Int32Array(workerData.barrier);
              assert.notEqual(Atomics.wait(gate,0,0,5000),'timed-out');
              assert.equal(Atomics.load(gate,0),1);
              return wasm;
            }});
          assert.deepEqual(bytes,wasm);
        }).catch(error => { throw error });
      `, { eval: true, workerData: { cacheDir, barrier, module: new URL('../scripts/wasm-opt-cache.mjs', import.meta.url).href } })
      workers.push(worker)
      ready.push(once(worker, 'message'))
      // Keep error outcomes handled even while waiting for the initial barrier.
      exits.push(once(worker, 'exit').then(value => ({ value }), error => ({ error })))
    }
    assert.deepEqual(await Promise.all(ready), [['miss'], ['miss']])
    Atomics.store(new Int32Array(barrier), 0, 1)
    Atomics.notify(new Int32Array(barrier), 0, 2)
    for (const outcome of await Promise.all(exits)) {
      if (outcome.error) throw outcome.error
      assert.deepEqual(outcome.value, [0])
    }
    assert.equal(readdirSync(cacheDir).length, 1)
    assert.ok(readdirSync(cacheDir)[0].endsWith('.json'))
    assert.deepEqual(cachedWasmOptimization({ input: wasm, tool: Buffer.from('tool'), flags: ['-Oz'], cacheDir,
      optimize: () => { throw Error('Expected intact shared cache entry') } }), wasm)
  } finally {
    await Promise.all(workers.map(worker => worker.terminate()))
    rmSync(cacheDir, { recursive: true, force: true })
  }
})
