import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import os from 'node:os'
import { parseArgs } from 'node:util'

const root = fileURLToPath(new URL('../', import.meta.url))
const { values } = parseArgs({ options: { out: { type: 'string' } } })
const hash = value => createHash('sha256').update(value).digest('hex')
const git = (...args) => execFileSync('git', ['-c', 'core.fsmonitor=false', ...args], { cwd: root, encoding: 'utf8' }).trim()
const paths = [...new Set([
  ...git('ls-files', '--', 'crates/planar-geometry', 'crates/polygon-core/src', 'crates/math-core/src').split('\n'),
  'scripts/bench-profile-triangulation.mjs', 'crates/Cargo.toml', 'crates/Cargo.lock',
  'crates/planar-geometry/tests/support/profile.rs', 'crates/planar-geometry/tests/profile_triangulation.rs',
  'crates/polygon-core/examples/bench_profile_triangulation.rs',
])].sort()
const fingerprints = () => paths.map(path => ({ path, sha256: hash(readFileSync(resolve(root, path))) }))
const files = fingerprints()
const startedAt = new Date().toISOString()
const source = { head: git('rev-parse', 'HEAD'), status: git('status', '--short'), files }
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, 'crates/target') }
execFileSync('cargo', ['build', '--locked', '--release', '--manifest-path', 'crates/Cargo.toml', '-p', 'polygon-core', '--example', 'bench_profile_triangulation'], { cwd: root, env, stdio: 'inherit' })
const binary = resolve(root, 'crates/target/release/examples/bench_profile_triangulation')
const data = JSON.parse(execFileSync(binary, [], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }))
assert.deepEqual(fingerprints(), files, 'Sources changed during benchmark')
const key = row => `${row.kind}/${row.side}x${row.side}/${row.segments}`
const summary = [...new Set(data.samples.map(key))].map(id => {
  const samples = data.samples.filter(sample => key(sample) === id)
  assert.equal(new Set(samples.map(sample => sample.status)).size, 1, `Unstable status for ${id}`)
  const sorted = samples.map(sample => sample.wallMs).sort((a,b) => a-b)
  return { id, n: samples.length, status: samples[0].status, error: samples[0].error, triangles: samples[0].triangles, min: sorted[0], p50: sorted[Math.floor(sorted.length / 2)], max: sorted.at(-1) }
})
const report = {
  startedAt, source, binarySha256: hash(readFileSync(binary)),
  environment: { rustc: execFileSync('rustc', ['--version', '--verbose'], { encoding: 'utf8' }).trim(), platform: process.platform, arch: process.arch, release: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] },
  boundaries: ['Native release triangulate_profile only; excludes fixture construction, result validation, compile time, host transport and WASM.', 'Three warmups and nine sequential samples per fixture. No profiling or CPU isolation. Run without other builds/tests.', 'Every successful output verifies area, positive triangles, exact authored coordinates, every boundary segment and paired interior edges. Refusals stay explicit.', 'Includes actual polygon-core prism-profile construction before timing; compare fixtureSha256 across revisions.', 'Selected source closure and executable fingerprint, not a complete reproducible-build attestation.'],
  fixtureSha256: hash(JSON.stringify(data.fixtures)), ...data, summary,
}
if (values.out) {
  mkdirSync(dirname(resolve(values.out)), { recursive: true })
  writeFileSync(values.out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
  console.log(JSON.stringify({ summary, report: values.out }, null, 2))
} else console.log(JSON.stringify(report, null, 2))
