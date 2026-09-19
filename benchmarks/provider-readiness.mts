import { spawnSync } from 'node:child_process'
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
  const samples = []
  for (let index = 0; index < 3; index++) {
    const child = spawnSync(process.execPath, ['--import', 'tsx', fileURLToPath(import.meta.url), '--child'], {
      encoding: 'utf8', timeout: 60_000, maxBuffer: 1024 * 1024,
    })
    if (child.error) throw child.error
    if (child.status !== 0) throw new Error(`Readiness child failed: ${child.stderr || child.stdout}`)
    samples.push(JSON.parse(child.stdout.trim()))
  }
  console.log(JSON.stringify({ schema: 1, node: process.version, platform: process.platform, arch: process.arch,
    cpu: cpus()[0]?.model,
    packedKernelSha256: createHash('sha256').update(readFileSync(new URL('../src/generated/geometry-kernels/bytes.ts', import.meta.url))).digest('hex'),
    method: 'Three sequential fresh Node processes; module import measured separately; unchanged 250 ms admission deadline.', samples }, null, 2))
}
