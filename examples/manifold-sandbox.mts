import {readFileSync} from 'node:fs'
import {
  booleanManifoldInKernel,
  checkManifoldInKernel,
  metricsManifoldInKernel,
  repairManifoldInKernel,
} from '../src/services/geometry/manifoldAnalysis'

interface Mesh {
  positions: Float64Array
  indices: Uint32Array
}

// Optional input: node --import tsx examples/manifold-sandbox.mts mesh.json
// JSON shape: {"positions": [x,y,z, ...], "indices": [a,b,c, ...]}.
function loadMesh(file: string): Mesh {
  const value = JSON.parse(readFileSync(file, 'utf8')) as {positions?: unknown, indices?: unknown}
  if (!Array.isArray(value.positions) || !value.positions.length
    || value.positions.some(entry => typeof entry !== 'number' || !Number.isFinite(entry))
    || !Array.isArray(value.indices) || !value.indices.length
    || value.indices.some(entry => !Number.isSafeInteger(entry) || entry < 0 || entry > 0xffffffff)) {
    throw new Error('Expected non-empty finite numeric positions and u32 indices')
  }
  return {positions: new Float64Array(value.positions), indices: new Uint32Array(value.indices)}
}

const file = process.argv[2]
const input: Mesh = file ? loadMesh(file) : {
  // Tetrahedron with its last face omitted, so Full repair has one hole to fill.
  positions: new Float64Array([0,0,0, 1,0,0, 0,1,0, 0,0,1]),
  indices: new Uint32Array([0,2,1, 0,1,3, 1,2,3]),
}

const before = checkManifoldInKernel(input.positions, input.indices)
const repaired = repairManifoldInKernel(input.positions, input.indices, 0, true)
if (!repaired.isManifold) throw new Error('Full repair left residual non-manifold defects')
const measured = metricsManifoldInKernel(repaired.positions, repaired.indices)

// Make a disjoint translated copy and union both solids.
let minX = Infinity, maxX = -Infinity
for (let index = 0; index < repaired.positions.length; index += 3) {
  minX = Math.min(minX, repaired.positions[index])
  maxX = Math.max(maxX, repaired.positions[index])
}
const offset = maxX - minX + 1
const shifted = repaired.positions.map((value, index) => index % 3 === 0 ? value + offset : value)
const union = booleanManifoldInKernel(
  repaired.positions, repaired.indices,
  shifted, repaired.indices,
  'union',
)
const unionMetrics = metricsManifoldInKernel(union.positions, union.indices)

console.log(JSON.stringify({
  source: file ?? 'built-in open tetrahedron',
  before: {
    isManifold: before.isManifold,
    isManifoldWithBoundary: before.isManifoldWithBoundary,
    boundaryEdges: before.boundaryEdges.length / 2,
  },
  repair: {
    isManifold: repaired.isManifold,
    filledHoles: repaired.filledHoles,
    filledTriangles: repaired.filledTriangles,
  },
  volume: measured.signedVolume,
  union: {
    isManifold: union.isManifold,
    volume: unionMetrics.signedVolume,
    components: unionMetrics.componentCount,
  },
}, null, 2))
