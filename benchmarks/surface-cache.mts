import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { SurfaceGroupContentCache } from '../src/services/surfaceGroupContentCache'

const variant = process.argv[2]
if (variant === 'unbounded' || variant === 'bounded') {
  assert.ok(global.gc, 'Run with --expose-gc')
  const cache = variant === 'bounded' ? new SurfaceGroupContentCache() : new Map<string, Uint32Array>()
  const collect = async () => {
    global.gc!()
    await new Promise<void>(resolve => setImmediate(resolve))
    global.gc!()
  }
  await collect()
  const before = process.memoryUsage().arrayBuffers
  for (let index = 0; index < 1000; index++) {
    const key = `asset:synthetic:${index}`
    if (cache instanceof SurfaceGroupContentCache) cache.getOrCompute(key, () => new Uint32Array(32768).fill(index))
    else cache.set(key, new Uint32Array(32768).fill(index))
  }
  await collect()
  const retainedArrayBufferDelta = process.memoryUsage().arrayBuffers - before
  const latest = cache instanceof SurfaceGroupContentCache
    ? cache.getOrCompute('asset:synthetic:999', () => { throw Error('Latest entry missing') })
    : cache.get('asset:synthetic:999')!
  assert.equal(latest[0], 999)
  console.log(JSON.stringify({ variant, publications: 1000, trianglesPerPublication: 32768,
    entries: cache.size, retainedArrayBufferDelta,
    accountedBytes: cache instanceof SurfaceGroupContentCache ? cache.retainedBytes : null }))
} else {
  const samples = ['unbounded', 'bounded'].map(variant => {
    const result = spawnSync(process.execPath, ['--expose-gc', '--import', 'tsx', fileURLToPath(import.meta.url), variant],
      { encoding: 'utf8', timeout: 30_000 })
    if (result.error) throw result.error
    if (result.status !== 0) throw Error(result.stderr || result.stdout)
    return JSON.parse(result.stdout.trim())
  })
  console.log(JSON.stringify({ schema: 1, node: process.version, platform: process.platform,
    method: 'Separate Node processes, 1000 synthetic unique publications, GC before/after; ArrayBuffer delta, not RSS or grouping latency.', samples }, null, 2))
}
