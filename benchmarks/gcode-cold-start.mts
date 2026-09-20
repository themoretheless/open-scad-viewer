import assert from 'node:assert/strict'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {performance} from 'node:perf_hooks'
import {fileURLToPath} from 'node:url'
import {executeGcodePreview, executeGcodePreviewAsync} from '../src/services/gcodePreviewRuntime'
import type {GcodePreviewRequest, GcodePreviewResponse} from '../src/services/gcodePreviewProtocol'
import identity from '../src/generated/geometry-kernels/identity'

const mode = process.argv[2]
if (mode) {
  assert.ok(mode === 'sync' || mode === 'async')
  const request: GcodePreviewRequest = {version: 1, id: 1, job: {kind: 'parse',
    gcode: 'G90\nM83\nG1 X0 Y0 Z0.2 F600\n' + Array.from({length: 100}, (_, i) => `G1 X${(i + 1) % 2} E0.01 F600\n`).join('')}}
  const run = mode === 'sync' ? executeGcodePreview : executeGcodePreviewAsync
  const verify = (response: GcodePreviewResponse) => {
    if (!response.ok) throw new Error(response.error)
    assert.equal(response.result.preview.moves.length, 101)
    assert.equal(response.result.preview.printDistanceMm, 100)
    assert.ok(Math.abs(response.result.preview.extrusionMm - 1) < 1e-9)
    return createHash('sha256').update(JSON.stringify(response)).digest('hex')
  }
  const start = performance.now()
  const result = await run(request)
  const coldMs = performance.now() - start
  const resultSha256 = verify(result)
  for (let i = 0; i < 10; i++) assert.equal(verify(await run(request)), resultSha256)
  const samplesMs = []
  for (let i = 0; i < 31; i++) {
    const begin = performance.now(), response = await run(request)
    samplesMs.push(performance.now() - begin)
    assert.equal(verify(response), resultSha256)
  }
  console.log(JSON.stringify({mode, coldMs, resultSha256, warmP50Ms: [...samplesMs].sort((a,b) => a-b)[15], samplesMs}))
} else {
  const runs: {mode: string; coldMs: number; warmP50Ms: number; resultSha256: string; samplesMs: number[]}[] = []
  for (let i = 0; i < 9; i++) for (const mode of i % 2 ? ['async', 'sync'] : ['sync', 'async']) {
    const child = spawnSync(process.execPath, ['--import', 'tsx', fileURLToPath(import.meta.url), mode], {encoding: 'utf8', timeout: 30000})
    assert.equal(child.status, 0, child.stderr)
    runs.push(JSON.parse(child.stdout))
  }
  assert.equal(new Set(runs.map(run => run.resultSha256)).size, 1)
  const median = (values: number[]) => [...values].sort((a,b) => a-b)[Math.floor(values.length / 2)]
  console.log(JSON.stringify({node: process.version, platform: process.platform, arch: process.arch, artifact: identity,
    scope: 'Fresh Node process first G-code execution: validation, embedded decode, verified compilation, instantiation and 100-move parse; excludes module imports, process startup, worker transport and UI',
    processesPerMode: 9, results: ['sync', 'async'].map(mode => {
      const selected = runs.filter(run => run.mode === mode)
      return {mode, coldP50Ms: median(selected.map(run => run.coldMs)), warmP50Ms: median(selected.map(run => run.warmP50Ms))}
    }), runs}, null, 2))
}
