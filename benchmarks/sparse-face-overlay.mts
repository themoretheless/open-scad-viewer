import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {cpus} from 'node:os'
import {buildFaceOverlayGeometry, buildFaceTriangleIndex} from '../src/services/meshSelectionOverlay'
import {identity} from '../src/services/math3d'

const median = (values: number[]) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]
const vertices = new Float32Array([0,0,0,0,0,1, 1,0,0,0,0,1, 0,1,0,0,0,1, 1,1,0,0,0,1])
const transform = identity()
const samples = []
// Experimental only: row remapping avoids max-ID-sized storage, but may delay
// the first hover. Keep this out of the renderer until startup cost is addressed.
function compactIndex(ids: Uint32Array) {
  const rows = new Map<number, number>(), counts: number[] = []
  for (const id of ids) {
    let row = rows.get(id)
    if (row === undefined) { row = rows.size; rows.set(id, row); counts.push(0) }
    counts[row]++
  }
  const offsets = new Uint32Array(rows.size + 1)
  for (let row = 0; row < rows.size; row++) offsets[row + 1] = offsets[row] + counts[row]
  const cursor = offsets.slice(0, -1), triangles = new Uint32Array(ids.length)
  for (let t = 0; t < ids.length; t++) triangles[cursor[rows.get(ids[t])!]++] = t
  return {triangleCount:ids.length, offsets, triangles, rows}
}
for (const triangleCount of [2_000, 100_000, 500_000]) {
  const indices = new Uint32Array(triangleCount * 3)
  const ids = new Uint32Array(triangleCount)
  for (let t = 0; t < triangleCount; t++) {
    indices.set(t % 2 ? [1,3,2] : [0,1,2], t * 3)
    ids[t] = 0x80000000 + Math.floor(t / 2) * 17
  }
  const queries = Array.from({length:100}, (_, i) => ((i * 7919) % (triangleCount / 2)) * 2)
  assert.equal(buildFaceTriangleIndex(ids, triangleCount), null)
  const buildMs = []
  for (let i = 0; i < 7; i++) {
    const start = performance.now()
    const built = compactIndex(ids)
    buildMs.push(performance.now() - start)
    assert.equal(built.rows!.size, triangleCount / 2)
  }
  const index = compactIndex(ids)
  const run = (indexed: boolean) => queries.map(t => buildFaceOverlayGeometry(
    vertices, indices, ids, transform, t, indexed ? index.rows.get(ids[t])! : ids[t], undefined, indexed ? index : null,
  ))
  const expected = run(false)
  assert.deepEqual(run(true), expected)
  for (let i = 0; i < 3; i++) { run(false); run(true) }
  const scanMs: number[] = [], indexedMs: number[] = []
  for (let i = 0; i < 9; i++) {
    for (const indexed of i % 2 ? [true, false] : [false, true]) {
      const start = performance.now()
      const result = run(indexed)
      ;(indexed ? indexedMs : scanMs).push(performance.now() - start)
      assert.deepEqual(result, expected)
    }
  }
  samples.push({triangleCount, faces:index.rows!.size, queries:queries.length,
    typedIndexBytes:index.offsets.byteLength + index.triangles.byteLength,
    mapEntries:index.rows!.size, buildMs, buildMedianMs:median(buildMs),
    scanMs, indexedMs, scanMedianMs:median(scanMs), indexedMedianMs:median(indexedMs)})
}
console.log(JSON.stringify({node:process.version, cpu:cpus()[0]?.model,
  sourceSha256:createHash('sha256').update(readFileSync(new URL('../src/services/meshSelectionOverlay.ts', import.meta.url))).digest('hex'),
  benchmarkSha256:createHash('sha256').update(readFileSync(new URL(import.meta.url))).digest('hex'),
  method:'Synthetic two-triangle faces with sparse uint32 IDs; 100 varying face queries per sample; 3 warmups, 9 alternating scan/indexed samples; exact overlay parity outside timing. Build cost includes allocation and GC; excludes rendering. Map storage is not included in typedIndexBytes.',
  samples}, null, 2))
