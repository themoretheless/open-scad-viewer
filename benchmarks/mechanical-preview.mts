import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import os from 'node:os'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { compileModelGraph } from '../src/services/modelGraph'
import { createMechanicalDocument } from '../src/services/mechanicalGeneratorContract'
import { renderModelGraphPreviews } from '../src/mcp/modelGraphPreview'

const out = resolve(process.argv[2] ?? 'tmp/performance/mechanical-preview')
mkdirSync(out, { recursive: true })
const hash = (data: string | Uint8Array) => createHash('sha256').update(data).digest('hex')
const service = new HeadlessGeometryService()
const document = createMechanicalDocument({ kind: 'planetary_gears' })
const full = compileModelGraph(document)
const preview = compileModelGraph({ ...document, segments: 12 })
const rows: Array<{
  sample: number; fullMs: number; previewBuildMs: number; rasterMs: number;
  fullTriangles: number; previewTriangles: number; fullVolume: number;
  images: Array<{ view: string; sha256: string }>;
}> = []
for (let sample = -3; sample < 9; sample++) {
  const start = performance.now()
  const built = await service.compile(full.source, 'full')
  const fullMs = performance.now() - start
  assert.throws(() => renderModelGraphPreviews(built.meshes), /triangle limit/)
  const previewStart = performance.now()
  const display = await service.compile(preview.source, 'preview')
  const previewBuildMs = performance.now() - previewStart
  const rasterStart = performance.now()
  const images = renderModelGraphPreviews(display.meshes)
  const rasterMs = performance.now() - rasterStart
  assert.equal(images.length, 3)
  assert.equal(full.document.segments, 48)
  if (sample >= 0) rows.push({
    sample, fullMs, previewBuildMs, rasterMs,
    fullTriangles: built.meshes.reduce((n, mesh) => n + mesh.indices.length / 3, 0),
    previewTriangles: display.meshes.reduce((n, mesh) => n + mesh.indices.length / 3, 0),
    fullVolume: built.analysis.volume,
    images: images.map(image => ({ view: image.view, sha256: hash(image.png) })),
  })
  if (sample === 0) for (const image of images) writeFileSync(resolve(out, `${image.view}.png`), image.png)
}
for (const row of rows) {
  assert.equal(row.fullVolume, rows[0]!.fullVolume)
  assert.deepEqual(row.images, rows[0]!.images)
}
const paths = ['src/mcp/modelGraphGenerate.ts', 'src/mcp/modelGraphPreview.ts', 'src/generated/geometry-kernels/kernel_bg.wasm']
const report = {
  node: process.version, platform: process.platform, arch: process.arch, cpu: os.cpus()[0]?.model,
  warmups: 3, samples: 9, scope: 'Warm in-process full build plus additional display-only compile and raster; not end-to-end MCP latency.',
  fullSourceSha256: hash(full.source), previewSourceSha256: hash(preview.source),
  files: paths.map(path => ({ path, sha256: hash(readFileSync(path)) })),
  medians: Object.fromEntries((['fullMs', 'previewBuildMs', 'rasterMs'] as const).map(key => [key,
    rows.map(row => row[key]).sort((a, b) => a - b)[4],
  ])), rows,
}
writeFileSync(resolve(out, 'report.json'), JSON.stringify(report, null, 2))
console.log(JSON.stringify({ out, medians: report.medians, fullTriangles: rows[0]!.fullTriangles, previewTriangles: rows[0]!.previewTriangles }))
