import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {fileURLToPath} from 'node:url'
import {compileWasmArtifact, compileWasmArtifactSync} from '../src/services/wasmArtifact'

const modes = ['raw-async', 'verified-async', 'raw-sync', 'verified-sync'] as const
const artifact = readFileSync(new URL('../public/wasm/geometry-kernel.wasm', import.meta.url))
const identity = {byteLength: artifact.length, sha256: createHash('sha256').update(artifact).digest('hex')}
const mode = process.argv[2]
if (mode) {
  assert.ok(modes.includes(mode as typeof modes[number]))
  const start = performance.now()
  const module = mode === 'raw-async' ? await WebAssembly.compile(artifact)
    : mode === 'verified-async' ? await compileWasmArtifact(artifact, identity)
      : mode === 'raw-sync' ? new WebAssembly.Module(artifact) : compileWasmArtifactSync(artifact, identity)
  const durationMs = performance.now() - start
  assert.ok(WebAssembly.Module.exports(module).some(item => item.name === 'abi_request'))
  console.log(JSON.stringify({mode, durationMs}))
} else {
  const samples = Object.fromEntries(modes.map(mode => [mode, [] as number[]]))
  for (let i = 0; i < 9; i++) {
    const order = i % 2 ? [...modes].reverse() : modes
    for (const mode of order) {
      const child = spawnSync(process.execPath, ['--import', 'tsx', fileURLToPath(import.meta.url), mode], {
        encoding: 'utf8', timeout: 30_000,
      })
      assert.equal(child.status, 0, child.stderr)
      const sample = JSON.parse(child.stdout)
      assert.equal(sample.mode, mode)
      samples[mode].push(sample.durationMs)
    }
  }
  const results = Object.entries(samples).map(([mode, samplesMs]) => {
    const ordered = [...samplesMs].sort((a, b) => a - b)
    return {mode, p50Ms: ordered[4], minMs: ordered[0], maxMs: ordered[8], samplesMs}
  })
  console.log(JSON.stringify({node: process.version, platform: process.platform, arch: process.arch,
    artifact: identity, processesPerMode: 9, results,
    scope: 'First compilation in fresh Node processes; excludes file IO, module imports, identity setup, decompression, instantiation and browser/network time'}, null, 2))
}
