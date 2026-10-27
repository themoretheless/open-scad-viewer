import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import identity from '../src/generated/geometry-kernels/identity'
import {
  booleanManifoldInKernel,
  checkManifoldInKernel,
  metricsManifoldInKernel,
  repairManifoldInKernel,
} from '../src/services/geometry/manifoldAnalysis'

const publicBytes = readFileSync('public/wasm/geometry-kernel.wasm')
const generatedBytes = readFileSync('src/generated/geometry-kernels/kernel_bg.wasm')
assert.deepEqual(generatedBytes, publicBytes, 'Generated and public manifold WASM differ')
const artifact = {
  sha256: createHash('sha256').update(publicBytes).digest('hex'),
  byteLength: publicBytes.byteLength,
}
assert.deepEqual(identity, artifact, 'Packaged manifold WASM identity differs from its bytes')

const module = await WebAssembly.compile(publicBytes)
assert.deepEqual(WebAssembly.Module.imports(module), [], 'Geometry WASM must remain self-contained')
const exports = new Map(WebAssembly.Module.exports(module).map(entry => [entry.name, entry.kind]))
assert.equal(exports.get('memory'), 'memory')
const manifoldExports = [
  'abi_alloc',
  'abi_free',
  'abi_array_field',
  'abi_array_free',
  'abi_manifold_check',
  'abi_manifold_repair',
  'abi_manifold_metrics',
  'abi_manifold_boolean',
]
for (const name of manifoldExports) assert.equal(exports.get(name), 'function', `Missing ${name}`)

const positions = new Float64Array([
  0, 0, 0,
  1, 0, 0,
  0, 1, 0,
  0, 0, 1,
])
const indices = new Uint32Array([
  0, 2, 1,
  0, 1, 3,
  1, 2, 3,
  2, 0, 3,
])
const checked = checkManifoldInKernel(positions, indices)
assert.equal(checked.isManifold, true)
assert.equal(checked.componentCount, 1)

const measured = metricsManifoldInKernel(positions, indices)
assert.ok(Math.abs(measured.signedVolume - 1 / 6) < 1e-12)
assert.equal(measured.watertight, true)
assert.equal(measured.components[0]?.genus, 0)

const repaired = repairManifoldInKernel(positions, indices.slice(0, 9), 0, true)
assert.equal(repaired.isManifold, true)
assert.equal(repaired.filledHoles, 1)
assert.equal(repaired.filledTriangles, 1)
assert.ok(Math.abs(metricsManifoldInKernel(repaired.positions, repaired.indices).signedVolume - 1 / 6) < 1e-12)

const shifted = positions.map((value, index) => index % 3 === 0 ? value + 2 : value)
const united = booleanManifoldInKernel(positions, indices, shifted, indices, 'union')
assert.equal(united.isManifold, true)
const unionMetrics = metricsManifoldInKernel(united.positions, united.indices)
assert.equal(unionMetrics.componentCount, 2)
assert.ok(Math.abs(unionMetrics.signedVolume - 1 / 3) < 1e-12)

console.log(JSON.stringify({
  status: 'passed',
  artifact,
  manifoldExports,
  tetrahedron: {
    checked: checked.isManifold,
    repaired: repaired.isManifold,
    volume: measured.signedVolume,
    disjointUnionVolume: unionMetrics.signedVolume,
    disjointUnionComponents: unionMetrics.componentCount,
  },
}, null, 2))
