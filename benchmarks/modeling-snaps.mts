import { resolveModelingSnap, type SnapGeometry } from '../src/services/modelingSnaps'

type Case = { segments: number; runs: number }

const cases: Case[] = [
  { segments: 100, runs: 10_000 },
  { segments: 400, runs: 10_000 },
  { segments: 1_000, runs: 5_000 },
]

function geometryFor(segments: number): SnapGeometry {
  return {
    points: [],
    segments: Array.from({ length: segments }, (_, index) => {
      const x = index % 50
      const y = Math.floor(index / 50)
      return { a: [x, y, 0], b: [x + 0.9, y + 0.2, 0] }
    }),
  }
}

for (const { segments, runs } of cases) {
  const geometry = geometryFor(segments)
  const options = {
    project: (point: [number, number, number]): [number, number] => [point[0] * 20, point[1] * 20],
    radius: 12,
    grid: 1,
    geometry: true,
    guides: false,
  }
  for (let warmup = 0; warmup < 500; warmup++) {
    resolveModelingSnap([9.4, 9.1, 0], geometry, options)
  }
  const started = performance.now()
  let checksum = 0
  for (let run = 0; run < runs; run++) {
    checksum += resolveModelingSnap([9.4 + (run % 7) * 0.01, 9.1, 0], geometry, options).point[0]
  }
  const elapsedMs = performance.now() - started
  console.log(JSON.stringify({
    segments,
    runs,
    totalMs: Number(elapsedMs.toFixed(3)),
    avgUs: Number(((elapsedMs * 1_000) / runs).toFixed(3)),
    checksum: Number(checksum.toFixed(3)),
  }))
}
