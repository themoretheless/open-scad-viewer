import { performance } from 'node:perf_hooks'
import { selectionSurfaceIds } from '../src/services/meshSurfaceGroups'

function stripMesh(segments = 32_000) {
  const vertices = new Float32Array((segments + 1) * 2 * 6)
  const indices = new Uint32Array(segments * 6)
  for (let x = 0; x <= segments; x++) {
    vertices.set([x, 0, 0, 0, 0, 1, x, 1, 0, 0, 0, 1], x * 12)
    if (x < segments) {
      const low = x * 2
      indices.set([low, low + 2, low + 3, low, low + 3, low + 1], x * 6)
    }
  }
  return { vertices, indices }
}

const samples: number[] = []
const { vertices, indices } = stripMesh()
const warmIds = selectionSurfaceIds(vertices, indices)
if (new Set(warmIds).size !== 1) throw new Error(`Expected one surface group, got ${new Set(warmIds).size}`)
for (let i = 0; i < 9; i++) {
  ;(globalThis as typeof globalThis & { gc?: () => void }).gc?.()
  const start = performance.now()
  const ids = selectionSurfaceIds(vertices, indices)
  samples.push(performance.now() - start)
  if (new Set(ids).size !== 1) throw new Error(`Expected one surface group, got ${new Set(ids).size}`)
}
const transportedSamples: number[] = []
const transportedMesh = {
  indices,
  faceIds: warmIds,
}
for (let i = 0; i < 9; i++) {
  const start = performance.now()
  if (transportedMesh.faceIds.length !== transportedMesh.indices.length / 3) throw new Error('Invalid transported ids')
  transportedSamples.push(performance.now() - start)
}
samples.sort((left, right) => left - right)
transportedSamples.sort((left, right) => left - right)
console.log(JSON.stringify({
  benchmark: 'surface-groups-strip-64k',
  triangles: indices.length / 3,
  rustFallbackSamplesMs: samples.map(sample => Number(sample.toFixed(3))),
  rustFallbackMedianMs: Number(samples[Math.floor(samples.length / 2)]!.toFixed(3)),
  transportedNoopSamplesMs: transportedSamples.map(sample => Number(sample.toFixed(6))),
  transportedNoopMedianMs: Number(transportedSamples[Math.floor(transportedSamples.length / 2)]!.toFixed(6)),
}, null, 2))
