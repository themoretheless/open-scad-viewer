import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { performance } from 'node:perf_hooks'
import { fileURLToPath } from 'node:url'
import { readSvgDocument, readSvgDocumentAsync } from '../src/services/svgDocument'
import identity from '../src/generated/geometry-kernels/identity'

const mode = process.argv[2]
if (mode) {
  assert.ok(mode === 'sync' || mode === 'async')
  const source = '<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><rect width="20" height="10" fill="red"/></svg>'
  const run = mode === 'sync' ? readSvgDocument : readSvgDocumentAsync
  const start = performance.now()
  const result = await run(source, {}, 'preview')
  const coldMs = performance.now() - start
  assert.ok(Math.abs(result.widthMm - 20) < 0.001)
  assert.ok(Math.abs(result.heightMm - 10) < 0.001)
  assert.ok(result.normalizedSvg.includes('<path'))
  const digest = (value: unknown) => createHash('sha256').update(JSON.stringify(value)).digest('hex')
  const resultSha256 = digest(result)
  for (let i = 0; i < 10; i++) assert.equal(digest(await run(source, {}, 'preview')), resultSha256)
  const samplesMs: number[] = []
  for (let i = 0; i < 31; i++) {
    const begin = performance.now(), response = await run(source, {}, 'preview')
    samplesMs.push(performance.now() - begin)
    assert.equal(digest(response), resultSha256)
  }
  console.log(JSON.stringify({ mode, coldMs, resultSha256, warmP50Ms: [...samplesMs].sort((a, b) => a - b)[15] }))
} else {
  const runs: { mode: string; coldMs: number; warmP50Ms: number; resultSha256: string }[] = []
  for (let i = 0; i < 9; i++) for (const selected of i % 2 ? ['async', 'sync'] : ['sync', 'async']) {
    const child = spawnSync(process.execPath, ['--import', 'tsx', fileURLToPath(import.meta.url), selected], { encoding: 'utf8', timeout: 30000 })
    assert.equal(child.status, 0, child.stderr)
    runs.push(JSON.parse(child.stdout))
  }
  assert.equal(new Set(runs.map(run => run.resultSha256)).size, 1)
  const median = (values: number[]) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]
  console.log(JSON.stringify({ node: process.version, artifact: identity,
    scope: 'Fresh Node process SVG preview: validation, embedded decode, verified compilation, instantiation and document preview; excludes imports, process startup, worker transport and UI',
    processesPerMode: 9, results: ['sync', 'async'].map(mode => {
      const selected = runs.filter(run => run.mode === mode)
      return { mode, coldP50Ms: median(selected.map(run => run.coldMs)), warmP50Ms: median(selected.map(run => run.warmP50Ms)) }
    }), runs }, null, 2))
}
