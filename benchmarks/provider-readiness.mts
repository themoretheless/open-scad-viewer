import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { fileURLToPath } from 'node:url'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { cpus } from 'node:os'

if (process.argv.includes('--child')) {
  const start = performance.now()
  const { GeometryBuildEngine } = await import('../src/services/geometryBuildEngine')
  const { warmGeometryKernel } = await import('../src/services/openscadParser')
  const importMs = performance.now() - start
  const engine = new GeometryBuildEngine()
  const coldStart = performance.now()
  const cold = await engine.capabilities()
  const coldMs = performance.now() - coldStart
  // Observe the original in-flight warmup; do not create another engine or retry
  // compilation in a new realm after the admission deadline expires.
  await warmGeometryKernel()
  const settledMs = performance.now() - coldStart
  const warmStart = performance.now()
  const warm = await engine.capabilities()
  const warmMs = performance.now() - warmStart
  const states = (value: typeof cold) => value.engines.map(({ engineClass, availability, unavailableReason }) => ({ engineClass, availability, unavailableReason }))
  assert.ok(warm.engines.every(engine => engine.availability === 'available'))
  console.log(JSON.stringify({ importMs, coldMs, settledMs, warmMs, cold: states(cold), warm: states(warm) }))
} else {
  const concurrency = Number(process.env.READINESS_CONCURRENCY ?? 1)
  assert.ok(Number.isInteger(concurrency) && concurrency >= 1 && concurrency <= 16,
    'READINESS_CONCURRENCY must be an integer between 1 and 16')
  const run = promisify(execFile)
  const samples = []
  for (let wave = 0; wave < 3; wave++) {
    // Wait for every child on failure too, so the benchmark leaves no work behind.
    const results = await Promise.allSettled(Array.from({length: concurrency}, async () => {
      const child = await run(process.execPath, ['--import', 'tsx', fileURLToPath(import.meta.url), '--child'], {
        encoding: 'utf8', timeout: 60_000, maxBuffer: 1024 * 1024,
      })
      return JSON.parse(child.stdout.trim())
    }))
    for (const result of results) {
      if (result.status === 'rejected') throw result.reason
      samples.push(result.value)
    }
  }
  console.log(JSON.stringify({ schema: 1, node: process.version, platform: process.platform, arch: process.arch,
    cpu: cpus()[0]?.model,
    packedKernelSha256: createHash('sha256').update(readFileSync(new URL('../src/generated/geometry-kernels/bytes.ts', import.meta.url))).digest('hex'),
    concurrency, waves: 3,
    method: 'Three sequential waves of fresh Node processes; concurrent children per wave specified by concurrency; module import measured separately; unchanged 250 ms admission deadline.', samples }, null, 2))
}
