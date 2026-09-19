import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import { fileURLToPath } from 'node:url'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'

const root = fileURLToPath(new URL('../', import.meta.url))
const out = process.argv[2]
const iterations = Number(process.argv[3] ?? 5)
assert.ok(Number.isSafeInteger(iterations) && iterations >= 1 && iterations <= 50, 'iterations must be 1..50')
const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const inputs = [
  'benchmarks/worker-lifecycle.mts', 'src/mcp/directGeometrySupervisor.ts',
  'src/mcp/directGeometry.worker.mjs', 'src/mcp/directGeometry.worker.ts',
  'src/services/openscadParser.ts', 'src/services/cadKernelOps.ts',
  'src/services/geometry/meshAnalysis.ts', 'src/services/geometry/kernel.ts',
  'src/generated/geometry-kernels/bytes.ts', 'src/generated/language-kernel/bytes.ts',
  'src/generated/wasm-brotli/bytes.ts', 'src/services/wasmBrotliPacking.ts',
  'src/services/wasmPacking.ts', 'crates/Cargo.toml', 'scripts/build-wasm-brotli.mjs',
]
async function fingerprints() {
  return Promise.all(inputs.map(async path => ({ path, sha256: hash(await readFile(new URL(path, new URL('../', import.meta.url)))) })))
}
const files = await fingerprints()
const git = (...args: string[]) => execFileSync('git', ['-c', 'core.fsmonitor=false', ...args], { cwd: root, encoding: 'utf8' }).trim()
const source = { head: git('rev-parse', 'HEAD'), status: git('status', '--short'), files }
const samples: { iteration: number; operation: string; wallMs: number }[] = []
const supervisor = new DirectGeometrySupervisor({ jobDeadlineMs: 10_000, startupTimeoutMs: 5_000 })
const startedAt = new Date().toISOString()
try {
  for (let iteration = 0; iteration < iterations; iteration++) {
    // A failed build must be joined before the following successful request.
    const startFailure = performance.now()
    await assert.rejects(supervisor.build('assert(false, "worker benchmark refusal");', 'full', 'analysis'), /worker benchmark refusal/)
    samples.push({ iteration, operation: 'refusal-and-join', wallMs: performance.now() - startFailure })

    const startBuild = performance.now()
    const built = await supervisor.build('cube(2);', 'full', 'analysis')
    const buildMs = performance.now() - startBuild
    assert.equal(built.result.volume, 8)
    assert.equal(built.result.surfaceArea, 24)
    assert.equal(built.result.meshes.length, 1)
    assert.equal(built.result.meshes[0]!.indices.length, 36)
    samples.push({ iteration, operation: 'build-and-join', wallMs: buildMs })

    const startCapabilities = performance.now()
    const capabilities = await supervisor.capabilities()
    const capabilitiesMs = performance.now() - startCapabilities
    assert.ok(capabilities.engines.every(engine => engine.availability === 'available'))
    samples.push({ iteration, operation: 'capabilities-and-join', wallMs: capabilitiesMs })
    const snapshot = supervisor.snapshot()
    assert.equal(snapshot.workersStarted, (iteration + 1) * 3)
    assert.equal(snapshot.workersJoined, snapshot.workersStarted)
    assert.equal(snapshot.admittedJobs, 0)
    assert.equal(snapshot.quarantined, false)
  }
} finally {
  await supervisor.close()
}
assert.deepEqual(await fingerprints(), files, 'Inputs changed during benchmark')
const summary = [...new Set(samples.map(sample => sample.operation))].map(operation => {
  const sorted = samples.filter(sample => sample.operation === operation).map(sample => sample.wallMs).sort((a, b) => a - b)
  return { operation, n: sorted.length, min: sorted[0], p50: sorted[Math.ceil(sorted.length / 2) - 1], max: sorted.at(-1) }
})
const report = {
  startedAt, source,
  environment: { node: process.version, platform: process.platform, arch: process.arch, cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] },
  boundaries: [
    'Actual production disposable Node workers, including startup, execution and terminate/join; not browser edit latency.',
    'Default production join timeout is unchanged. Refusals, wrong geometry, missing joins and quarantine fail the run.',
    'Sequential requests; each has a fresh worker and WASM instance. Host and filesystem caches may warm across samples.',
    'Selected entrypoint/kernel fingerprints plus git state, not a full repository fingerprint.',
    'No profiling, forced GC, CPU isolation or timing regression threshold. Validation occurs after timing except expected refusal matching.',
  ],
  samples, summary, final: supervisor.snapshot(),
}
const json = `${JSON.stringify(report, null, 2)}\n`
if (out) {
  await mkdir(dirname(resolve(out)), { recursive: true })
  await writeFile(out, json, { flag: 'wx' })
  console.log(JSON.stringify({ summary, final: report.final, report: out }, null, 2))
} else console.log(json)
