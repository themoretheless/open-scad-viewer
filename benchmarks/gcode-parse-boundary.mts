import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {parseGcodePreview, type GcodePreviewResult} from '../src/services/geometry/polygon'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {compileWasmArtifact} from '../src/services/wasmArtifact'

const artifact = readFileSync(process.env.GCODE_WASM_PATH ?? 'public/wasm/geometry-kernel.wasm')
let loaded = false
setOptionalWasmCompiler(async (url, identity) => {
  assert.equal(url, '/wasm/geometry-kernel.wasm')
  loaded = true
  assert.ok(identity, 'Benchmark requires the build artifact identity; use its matching checkout for historical bytes')
  return compileWasmArtifact(artifact, identity)
})
const start = performance.now()
await warmGeometryKernel()
const coldCompileAndInstantiateMs = performance.now() - start
assert.ok(loaded)

function verify(result: GcodePreviewResult, moves: number) {
  assert.equal(result.moves.length, moves + 1)
  assert.equal(result.layers, 1)
  assert.equal(result.printDistanceMm, moves)
  assert.ok(Math.abs(result.extrusionMm - moves * 0.01) < 1e-7)
  assert.ok(Math.abs(result.estimatedTimeS - moves / 10) < 1e-7)
  assert.equal(result.moves.at(-1)?.layerIndex, 0)
}

const results = []
for (const moves of [100, 10_000, 80_000]) {
  let gcode = 'G90\nM83\nG1 X0 Y0 Z0.2 F600\n'
  for (let i = 0; i < moves; i++) gcode += `G1 X${(i + 1) % 2} Y0 E0.01 F600\n`
  assert.ok(Buffer.byteLength(gcode) < 4 * 1024 * 1024)
  for (let i = 0; i < 50; i++) verify(parseGcodePreview(gcode), moves)
  const samplesMs = []
  for (let i = 0; i < 31; i++) {
    const begin = performance.now()
    const result = parseGcodePreview(gcode)
    samplesMs.push(performance.now() - begin)
    verify(result, moves)
  }
  const ordered = [...samplesMs].sort((a, b) => a - b)
  results.push({moves, inputBytes: Buffer.byteLength(gcode), p50Ms: ordered[15], p95Ms: ordered[29], samplesMs})
}
console.log(JSON.stringify({node: process.version, arch: process.arch, platform: process.platform,
  execArgv: process.execArgv, artifactBytes: artifact.length,
  artifactSha256: createHash('sha256').update(artifact).digest('hex'), coldCompileAndInstantiateMs,
  scope: 'complete warm gcode_preview WASM call including value transport, not UI or file IO',
  warmups: 50, samples: 31, results}, null, 2))
