import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {pathToFileURL, fileURLToPath} from 'node:url'
import {resolve} from 'node:path'

const source = process.argv[2] ? pathToFileURL(resolve(process.argv[2])) : new URL('../src/services/meshSurfaceGroups.ts', import.meta.url)
const {inferSurfaceIds} = await import(source.href)
const hash = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const bytes = (value: Float32Array | Uint32Array) => new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
const samples = []
for (const {side, unused} of [{side:32,unused:0},{side:128,unused:0},{side:200,unused:0},{side:32,unused:100000}]) {
  const vertices = new Float32Array(((side + 1) ** 2 + unused) * 6)
  for(let i=(side+1)**2;i<vertices.length/6;i++)vertices[i*6]=i
  const indices = new Uint32Array(side * side * 6)
  for (let y = 0; y <= side; y++) for (let x = 0; x <= side; x++) {
    const offset = (y * (side + 1) + x) * 6
    vertices[offset] = x
    vertices[offset + 1] = y
    vertices[offset + 2] = Math.sin(x / 8) * Math.cos(y / 8) * 2
  }
  for (let y = 0; y < side; y++) for (let x = 0; x < side; x++) {
    const a = y * (side + 1) + x, b = a + 1, c = a + side + 1, d = c + 1
    indices.set([a,b,c,b,d,c], (y * side + x) * 6)
  }
  const expected = inferSurfaceIds(vertices, indices)
  for (let i = 0; i < 2; i++) assert.deepEqual(inferSurfaceIds(vertices, indices), expected)
  const times = []
  for (let i = 0; i < 9; i++) {
    const start = performance.now()
    const result = inferSurfaceIds(vertices, indices)
    times.push(performance.now() - start)
    assert.deepEqual(result, expected)
  }
  samples.push({side, unusedVertices: unused, triangles: indices.length / 3, inputSha256: hash(Buffer.concat([bytes(vertices), bytes(indices)])),
    idsSha256: hash(bytes(expected)), groups: new Set(expected).size,
    medianMs: [...times].sort((a,b) => a-b)[4], samplesMs: times})
}
console.log(JSON.stringify({node: process.version, arch: process.arch, source: fileURLToPath(source),
  sourceSha256: hash(readFileSync(source)),
  method: 'Deterministic wavy grids; 2 warmups, 9 samples; uncached grouping only; fixture creation and parity checks excluded; includes allocation/GC.', samples}, null, 2))
