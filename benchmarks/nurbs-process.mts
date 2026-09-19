import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import os from 'node:os'
import { fileURLToPath } from 'node:url'
import { runOwnNurbs } from '../src/mcp/modelGraphNurbsRuntime'
import { nurbsProcessFixtures } from './nurbs-process-fixtures'

const root = fileURLToPath(new URL('../', import.meta.url))
const out = process.argv[2]
const iterations = Number(process.argv[3] ?? 5)
assert.ok(Number.isSafeInteger(iterations) && iterations >= 1 && iterations <= 50, 'iterations must be 1..50')
const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const inputs = [
  'benchmarks/nurbs-process.mts', 'benchmarks/nurbs-process-fixtures.ts',
  'src/mcp/modelGraphNurbsRuntime.ts', 'src/mcp/ownNurbsProcess.ts',
  'src/services/modelGraphNurbsKernel.ts', 'src/services/modelGraphNurbs.ts',
  'src/services/geometry/kernel.ts', 'src/generated/geometry-kernels/bytes.ts',
  'src/generated/language-kernel/bytes.ts',
  'src/generated/wasm-brotli/bytes.ts', 'src/services/wasmBrotliPacking.ts',
  'src/services/wasmPacking.ts', 'crates/Cargo.toml', 'scripts/build-wasm-brotli.mjs',
]
async function fingerprints() {
  return Promise.all(inputs.map(async path => ({ path, sha256: hash(await readFile(resolve(root, path))) })))
}
const files = await fingerprints()
const git = (...args: string[]) => execFileSync('git', ['-c', 'core.fsmonitor=false', ...args], { cwd: root, encoding: 'utf8' }).trim()
const source = { head: git('rev-parse', 'HEAD'), status: git('status', '--short'), files }
const samples: { iteration: number; fixture: string; wallMs: number; responseBytes: number; responseSha256: string }[] = []
const startedAt = new Date().toISOString()
for (let iteration = 0; iteration < iterations; iteration++) {
  for (const fixture of nurbsProcessFixtures) {
    const start = performance.now()
    const result = await runOwnNurbs(fixture.document, { action: 'export', format: 'stl' })
    const wallMs = performance.now() - start
    assert.ok(result.ok, JSON.stringify(result))
    assert.ok(Math.abs(result.report.mesh!.signedVolumeMm3 - fixture.volume) < 1e-8)
    assert.equal(result.report.mesh!.closed, true)
    assert.ok('artifact' in result && result.artifact?.text.includes('facet normal'))
    const serialized = JSON.stringify(result)
    const responseSha256 = hash(serialized)
    const earlier = samples.find(sample => sample.fixture === fixture.name)
    if (earlier) assert.equal(responseSha256, earlier.responseSha256, 'Output changed between samples')
    samples.push({ iteration, fixture: fixture.name, wallMs, responseBytes: Buffer.byteLength(serialized), responseSha256 })
  }
}
assert.deepEqual(await fingerprints(), files, 'Inputs changed during benchmark')
const summary = nurbsProcessFixtures.map(({ name: fixture }) => {
  const sorted = samples.filter(sample => sample.fixture === fixture).map(sample => sample.wallMs).sort((a, b) => a - b)
  return { fixture, n: sorted.length, min: sorted[0], p50: sorted[Math.ceil(sorted.length / 2) - 1], max: sorted.at(-1) }
})
const report = {
  startedAt, source,
  environment: { node: process.version, v8: process.versions.v8, platform: process.platform, arch: process.arch, release: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] },
  boundaries: [
    'Production disposable NURBS processes: parent validation, startup, evaluation, output transfer and child close; not browser latency.',
    'Sequential requests, fresh child/WASM per request. Host/filesystem caches and parent validation may warm; no discarded warmup samples.',
    'Default 30-second deadline, two-job admission and output size limits unchanged. No alternate geometry engine.',
    'Selected input fingerprints plus git state, not a full repository fingerprint.',
    'No concurrent builds/profilers, forced GC, CPU isolation or timing regression threshold. Result assertions/hashing occur outside timing.',
  ],
  fixtureSha256: hash(JSON.stringify(nurbsProcessFixtures)), samples, summary,
}
const json = `${JSON.stringify(report, null, 2)}\n`
if (out) {
  await mkdir(dirname(resolve(out)), { recursive: true })
  await writeFile(out, json, { flag: 'wx' })
  console.log(JSON.stringify({ summary, report: out }, null, 2))
} else console.log(json)
