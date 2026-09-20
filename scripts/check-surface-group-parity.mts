import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import assert from 'node:assert/strict'
import {inferSurfaceIds} from '../src/services/meshSurfaceGroups'

const executable = process.argv[2]
if (!executable) throw new Error('Pass the built surface_group_parity executable path')
type Fixture = {vertices: number[]; indices: number[]; angle: number}
const fixtures: Fixture[] = []
let state = 0x12345678
function random() {
  state ^= state << 13; state ^= state >>> 17; state ^= state << 5
  return (state >>> 0) / 4294967296
}
for (let seed = 0; seed < 200; seed++) {
  const count = 3 + Math.floor(random() * 90)
  const vertices = Array.from({length: count}, () => [
    Math.floor(random() * 8), Math.floor(random() * 8), seed % 2 ? 0 : Math.floor(random() * 8),
    0, 0, 1,
  ]).flat()
  const indices = Array.from({length: 3 * Math.floor(random() * 160)}, () => Math.floor(random() * count))
  for (const angle of [0, 15, 30, 60]) fixtures.push({vertices, indices, angle})
}
for (const angle of [0, 30, 60]) {
  const vertices = [
    [0, 0, 0], [1, 0, 0], [0, 1, 0], [-0, 1, 0], [1, -0, 0], [1, 1, 0],
    ...Array.from({length: 1000}, (_, i) => [i + 10, 5, 0]),
  ].flatMap(p => [...p, 0, 0, 1])
  for (const indices of [[0, 1, 2, 3, 4, 5], [0, 1, 2, 4, 3, 5], [0, 1, 2, 1, 0, 0], []]) {
    fixtures.push({vertices, indices, angle})
  }
}
for (const angle of [0, 15, 30, 60]) {
  for (const offset of [-0.01, -1e-6, 0, 1e-6, 0.01]) {
    const bend = (angle + offset) * Math.PI / 180
    const vertices = Array.from(new Float32Array([
      [0, 0, 0], [1, 0, 0], [0, 1, 0], [0, -Math.cos(bend), Math.sin(bend)],
    ].flatMap(p => [...p, 0, 0, 1])))
    fixtures.push({vertices, indices: [0, 1, 2, 1, 0, 3], angle})
  }
}
fixtures.push(
  {vertices: [], indices: [0], angle: 30},
  {vertices: Array(18).fill(0), indices: [0, 1, 3], angle: 30},
  {vertices: Array(24).fill(0), indices: [0, 1, 4], angle: 30},
  {vertices: Array(5).fill(0), indices: [], angle: 30},
  {vertices: [], indices: [], angle: -1},
  {vertices: [], indices: [], angle: 61},
)
// JSON does not preserve signed zero or nonfinite values; native unit tests cover them.
const input = fixtures.map(f => JSON.stringify(f)).join('\n') + '\n'
const run = spawnSync(executable, [], {input, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024})
if (run.error) throw run.error
assert.equal(run.status, 0, run.stderr)
const results = run.stdout.trim().split('\n').map(line => JSON.parse(line))
assert.equal(results.length, fixtures.length)
fixtures.forEach((f, i) => {
  let expected: {ids: number[]} | {error: string}
  try {
    expected = {ids: Array.from(inferSurfaceIds(new Float32Array(f.vertices), new Uint32Array(f.indices), f.angle))}
  } catch (error) {
    expected = {error: (error as Error).message}
  }
  assert.deepEqual(results[i], expected, `fixture ${i}`)
})
const hash = (data: string | Buffer) => createHash('sha256').update(data).digest('hex')
console.log(JSON.stringify({
  cases: fixtures.length, parity: 'exact IDs', node: process.version,
  fixtureSha256: hash(input), executableSha256: hash(readFileSync(executable)),
  rustSourceSha256: hash(readFileSync('crates/geometry-bridge/src/mesh_surface_groups.rs')),
  hostSourceSha256: hash(readFileSync('src/services/meshSurfaceGroups.ts')),
  scope: 'native algorithm conformance only; no WASM or publication timing',
}, null, 2))
