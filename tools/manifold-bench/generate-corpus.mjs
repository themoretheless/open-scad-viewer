#!/usr/bin/env node
// Deterministically regenerates differential-corpus.json. Run with --check to
// fail when the committed corpus differs from the generated one.
import { readFile, writeFile } from 'node:fs/promises'

const box = (origin, size, rotate) => (rotate ? { origin, size, rotate } : { origin, size })

// Box mesh with the same vertex order and outward winding as the Rust oracle.
function boxMesh([x, y, z], [sx, sy, sz]) {
  return {
    positions: [
      x,
      y,
      z,
      x + sx,
      y,
      z,
      x + sx,
      y + sy,
      z,
      x,
      y + sy,
      z,
      x,
      y,
      z + sz,
      x + sx,
      y,
      z + sz,
      x + sx,
      y + sy,
      z + sz,
      x,
      y + sy,
      z + sz,
    ],
    indices: [
      0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 1, 2, 6, 1, 6, 5, 3, 0, 4, 3, 4, 7,
    ],
  }
}

// Ear-clipping triangulation of a simple counter-clockwise polygon.
function earClip(polygon) {
  const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
  const inside = (p, a, b, c) => cross(a, b, p) > 0 && cross(b, c, p) > 0 && cross(c, a, p) > 0
  const rest = polygon.map((_, i) => i)
  const triangles = []
  while (rest.length > 3) {
    const k = rest.findIndex((v, i) => {
      const [u, w] = [rest[(i + rest.length - 1) % rest.length], rest[(i + 1) % rest.length]]
      const [a, b, c] = [polygon[u], polygon[v], polygon[w]]
      return cross(a, b, c) > 0 && rest.every(o => o === u || o === v || o === w || !inside(polygon[o], a, b, c))
    })
    if (k < 0) throw new Error('polygon is not simple and counter-clockwise')
    triangles.push([rest[(k + rest.length - 1) % rest.length], rest[k], rest[(k + 1) % rest.length]])
    rest.splice(k, 1)
  }
  triangles.push(rest)
  return triangles
}

// Extruded simple counter-clockwise polygon as a closed outward mesh.
// Coordinates stay dyadic so the f32 transport to manifold-3d is exact.
function prism(polygon, z0, z1) {
  const n = polygon.length
  const positions = []
  for (const z of [z0, z1]) for (const [x, y] of polygon) positions.push(x, y, z)
  const indices = []
  for (const [a, b, c] of earClip(polygon)) {
    indices.push(a, c, b)
    indices.push(n + a, n + b, n + c)
  }
  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n
    indices.push(i, j, n + j, i, n + j, n + i)
  }
  return { positions, indices }
}

const shift = (mesh, offset) => ({
  positions: mesh.positions.map((v, i) => v + offset[i % 3]),
  indices: mesh.indices,
})

const L = [
  [0, 0],
  [4, 0],
  [4, 1],
  [1, 1],
  [1, 4],
  [0, 4],
]
const U = [
  [0, 0],
  [5, 0],
  [5, 4],
  [4, 4],
  [4, 1],
  [1, 1],
  [1, 4],
  [0, 4],
]
const STAR = [
  [0, -1],
  [2, -2],
  [1, 0],
  [2, 2],
  [0, 1],
  [-2, 2],
  [-1, 0],
  [-2, -2],
]
const T = [
  [0, 0],
  [1, 0],
  [1, 2],
  [2.5, 2],
  [2.5, 3],
  [-1.5, 3],
  [-1.5, 2],
  [0, 2],
]

const cases = []
const add = (id, a, b, extra = {}) => cases.push({ id, a, b, ...extra })
const axes = [
  [0, 'x'],
  [1, 'y'],
  [2, 'z'],
]
const withAxis = (base, axis, value) => base.map((v, i) => (i === axis ? value : v))

// The original eight hand-written cases.
add('overlapping-unit-boxes', box([0, 0, 0], [2, 2, 2]), box([1, 1, 1], [2, 2, 2]))
add('disjoint-boxes', box([0, 0, 0], [1, 1, 1]), box([3, 0, 0], [2, 1, 1]))
add('contained-box', box([0, 0, 0], [4, 4, 4]), box([1, 1, 1], [1, 2, 1]))
add('identical-boxes', box([-1, -2, -3], [2, 4, 6]), box([-1, -2, -3], [2, 4, 6]))
add('negative-coordinate-overlap', box([-4, -3, -2], [3, 4, 5]), box([-2, -2, -1], [4, 2, 3]))
add('rectangular-corner-overlap', box([0, 0, 0], [5, 2, 1]), box([4, 1, 0.5], [2, 3, 2]))
add('thin-overlap', box([0, 0, 0], [3, 3, 0.25]), box([1, 1, -0.25], [3, 3, 0.5]))
add('different-scale-overlap', box([0, 0, 0], [0.5, 0.5, 0.5]), box([0.25, 0.25, 0.25], [2, 2, 2]))

// Coplanar faces: shared face planes along each axis, partial and full.
for (const [axis, name] of axes) {
  add(`coplanar-flush-min-${name}`, box([0, 0, 0], [3, 3, 3]), box(withAxis([1, 1, 1], axis, 0), [1, 1, 1]))
  add(`coplanar-flush-max-${name}`, box([0, 0, 0], [3, 3, 3]), box(withAxis([1, 1, 1], axis, 2), [1, 1, 1]))
  add(`coplanar-straddle-${name}`, box([0, 0, 0], [3, 3, 3]), box(withAxis([0.5, 0.5, 0.5], axis, 2), [2, 2, 2]))
  add(`coplanar-slab-${name}`, box([0, 0, 0], [3, 3, 3]), box([0, 0, 0], withAxis([3, 3, 3], axis, 1)))
}
add('coplanar-two-faces', box([0, 0, 0], [2, 2, 2]), box([0, 0, 0.5], [1, 1, 1]))
add('coplanar-three-faces', box([0, 0, 0], [2, 2, 2]), box([0, 0, 0], [1, 1, 1]))
add('coplanar-offset-same-height', box([0, 0, 0], [2, 2, 1]), box([1, 1, 0], [2, 2, 1]))

// Touching faces, edges and vertices (zero-volume intersection). Policy
// (plan item 35): a face-touching intersection is empty, never a flat
// zero-volume sheet (manifold-3d returns one on some axes); a union touching
// only along an edge or at a vertex is not a 2-manifold and is refused
// (manifold-3d returns it with duplicated seam vertices).
const emptyIntersection = { expect: { ops: { intersection: 'empty' } } }
const refusedUnion = { expect: { ops: { union: 'refused' } } }
for (const [axis, name] of axes) {
  add(
    `touching-face-full-${name}`,
    box([0, 0, 0], [1, 1, 1]),
    box(withAxis([0, 0, 0], axis, 1), [1, 1, 1]),
    emptyIntersection,
  )
  add(
    `touching-face-partial-${name}`,
    box([0, 0, 0], [1, 1, 1]),
    box(withAxis([0.5, 0.5, 0.5], axis, 1), [1, 1, 1]),
    emptyIntersection,
  )
  add(`touching-edge-${name}`, box([0, 0, 0], [1, 1, 1]), box(withAxis([1, 1, 1], axis, 0), [1, 1, 1]), refusedUnion)
}
add('touching-vertex', box([0, 0, 0], [1, 1, 1]), box([1, 1, 1], [1, 1, 1]), refusedUnion)
add('touching-vertex-negative', box([0, 0, 0], [1, 1, 1]), box([-1, -1, -1], [1, 1, 1]), refusedUnion)

// Nested solids.
for (const [i, [offset, size]] of [
  [0.5, 1],
  [1, 2],
  [0.25, 3.5],
  [1.5, 0.5],
].entries()) {
  add(`nested-${i}`, box([0, 0, 0], [4, 4, 4]), box([offset, offset, offset], [size, size, size]))
  add(`nested-reversed-${i}`, box([offset, offset, offset], [size, size, size]), box([0, 0, 0], [4, 4, 4]))
}
add('nested-rotated-45', box([0, 0, 0], [8, 8, 8]), box([4, 2, 2], [2, 2, 2], [0, 0, 45]))

// Disjoint solids.
for (const [i, gap] of [0.5, 2, 10, 1 / 1024].entries()) {
  add(`disjoint-gap-${i}`, box([0, 0, 0], [1, 1, 1]), box([1 + gap, 0, 0], [1, 1, 1]))
}
add('disjoint-diagonal', box([0, 0, 0], [1, 1, 1]), box([2, 2, 2], [1, 1, 1]))
add('disjoint-far', box([-100, -100, -100], [1, 1, 1]), box([100, 100, 100], [1, 1, 1]))

// Identical operands.
add('identical-unit', box([0, 0, 0], [1, 1, 1]), box([0, 0, 0], [1, 1, 1]))
add('identical-rotated', box([0, 0, 0], [1, 2, 3], [0, 0, 30]), box([0, 0, 0], [1, 2, 3], [0, 0, 30]))
add('identical-l-prism', { mesh: prism(L, 0, 1) }, { mesh: prism(L, 0, 1) })

// Thin slivers.
for (const [i, t] of [1 / 64, 1 / 1024, 1 / 16384].entries()) {
  add(`sliver-overlap-${i}`, box([0, 0, 0], [2, 2, 2]), box([2 - t, 0.5, 0.5], [1, 1, 1]))
  add(`sliver-plate-${i}`, box([0, 0, 0], [2, 2, 2]), box([0.5, 0.5, 1], [1, 1, t]))
  add(`sliver-gap-${i}`, box([0, 0, 0], [1, 1, 1]), box([1 + t, 0, 0], [1, 1, 1]))
}

// Tiny and huge scales.
for (const [i, s] of [1 / 1024, 1 / 128, 1 / 16, 64, 1024, 16384].entries()) {
  add(`scale-overlap-${i}`, box([0, 0, 0], [2 * s, 2 * s, 2 * s]), box([s, s, s], [2 * s, 2 * s, 2 * s]))
  add(`scale-nested-${i}`, box([0, 0, 0], [4 * s, 4 * s, 4 * s]), box([s, s, s], [s, s, s]))
}
add('scale-mixed-huge-tiny', box([0, 0, 0], [4096, 4096, 4096]), box([1, 1, 1], [1 / 64, 1 / 64, 1 / 64]))
add('scale-offset-far', box([1e5, 1e5, 1e5], [2, 2, 2]), box([1e5 + 1, 1e5 + 1, 1e5 + 1], [2, 2, 2]))

// Rotated copies.
for (const angle of [15, 30, 45, 60, 90, 180]) {
  add(`rotated-z-${angle}`, box([0, 0, 0], [2, 2, 2]), box([1, 1, 0.5], [2, 2, 1], [0, 0, angle]))
}
for (const angle of [20, 45]) {
  add(`rotated-x-${angle}`, box([0, 0, 0], [2, 2, 2]), box([0.5, 1, 1], [1, 2, 2], [angle, 0, 0]))
  add(`rotated-y-${angle}`, box([0, 0, 0], [2, 2, 2]), box([1, 0.5, 1], [2, 1, 2], [0, angle, 0]))
}
add('rotated-xyz', box([0, 0, 0], [2, 2, 2]), box([1, 1, 1], [1.5, 1.5, 1.5], [30, 40, 50]))
add('rotated-both', box([0, 0, 0], [2, 2, 2], [0, 0, 10]), box([0.5, 0.5, 0.5], [2, 2, 2], [10, 0, 0]))

// Non-convex shapes.
add('l-prism-box-overlap', { mesh: prism(L, 0, 2) }, box([0.5, 0.5, 0.5], [3, 3, 1]))
add('l-prism-notch-fill', { mesh: prism(L, 0, 1) }, box([1, 1, 0], [3, 3, 1]))
add('l-prism-self-shifted', { mesh: prism(L, 0, 2) }, { mesh: shift(prism(L, 0, 2), [0.5, 0.5, 0.5]) })
add('u-prism-bridge', { mesh: prism(U, 0, 2) }, box([-1, 2, 0.5], [7, 1, 1]))
add('u-prism-inside-gap', { mesh: prism(U, 0, 2) }, box([1.5, 1.5, 0.5], [2, 2, 1]))
// The box faces pass exactly through the star's reflex vertices (±1, 0); the
// kernel refuses that difference instead of returning a non-manifold result,
// while manifold-3d returns one. The clear variant avoids the vertex contact.
add('star-prism-box', { mesh: prism(STAR, 0, 1) }, box([-1, 0, -0.5], [2, 2, 2]), {
  expect: { ops: { difference: 'refused' } },
})
add('star-prism-box-clear', { mesh: prism(STAR, 0, 1) }, box([-0.5, -0.5, -0.5], [1, 3, 2]))
add('star-prism-shifted-copy', { mesh: prism(STAR, 0, 1) }, { mesh: shift(prism(STAR, 0, 1), [0.25, 0.5, 0.25]) })
add('t-prism-crossbar-cut', { mesh: prism(T, 0, 1) }, box([-2, 1.5, -1], [5, 1, 3]))
add('t-prism-l-prism', { mesh: prism(T, 0, 1) }, { mesh: shift(prism(L, 0, 2), [-0.5, 0.5, -0.5]) })

// Defective meshes. `expect.repaired` is the pre-repair outcome every
// operation must report and `reference` the clean solid manifold-3d receives
// in place of the defective operand; `expect.refused` cases must be rejected
// by every operation rather than returned non-manifold.
const cube = boxMesh([0, 0, 0], [2, 2, 2])
const other = box([1, 1, 1], [2, 2, 2])
const repairedA = { expect: { repaired: [true, false] }, reference: { a: { mesh: cube } } }
const refused = { expect: { refused: true } }
const flip = indices => indices.map((_, i, a) => a[i - (i % 3) + [0, 2, 1][i % 3]])
add(
  'defect-flipped-face',
  { mesh: { positions: cube.positions, indices: [...flip(cube.indices.slice(0, 6)), ...cube.indices.slice(6)] } },
  other,
  repairedA,
)
add('defect-inside-out', { mesh: { positions: cube.positions, indices: flip(cube.indices) } }, other, repairedA)
add(
  'defect-duplicated-face',
  { mesh: { positions: cube.positions, indices: [...cube.indices, ...cube.indices.slice(0, 3)] } },
  other,
  refused,
)
const soup = { positions: [], indices: [] }
for (const v of cube.indices) {
  soup.indices.push(soup.positions.length / 3)
  soup.positions.push(...cube.positions.slice(v * 3, v * 3 + 3))
}
add('defect-unwelded-soup', { mesh: soup }, other, repairedA)
add('defect-open-boundary', { mesh: { positions: cube.positions, indices: cube.indices.slice(6) } }, other, repairedA)
add(
  'defect-degenerate-triangle',
  { mesh: { positions: [...cube.positions, 1, 0, 0], indices: [...cube.indices, 0, 8, 1] } },
  other,
  refused,
)
add(
  'defect-two-sided-fin',
  { mesh: { positions: [...cube.positions, 1, 1, 5], indices: [...cube.indices, 4, 5, 8, 4, 8, 5] } },
  other,
  refused,
)
const far = boxMesh([2, 2, 2], [2, 2, 2])
add(
  'defect-vertex-pinched-cubes',
  {
    mesh: {
      positions: [...cube.positions, ...far.positions],
      indices: [...cube.indices, ...far.indices.map(i => i + 8)],
    },
  },
  box([10, 0, 0], [1, 1, 1]),
  refused,
)

const text = `${JSON.stringify({ schemaVersion: 1, cases }, null, 2)}\n`
const target = new URL('./differential-corpus.json', import.meta.url)
if (process.argv.includes('--check')) {
  if ((await readFile(target, 'utf8')) !== text)
    throw new Error('differential-corpus.json is stale; run npm run corpus:generate')
  console.log(`differential corpus up to date: ${cases.length} cases`)
} else {
  await writeFile(target, text)
  console.log(`wrote ${cases.length} cases`)
}
