// Explicit versioned capture. Builds never invoke this script or rewrite an oracle.
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, linkSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs'
import { basename, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { verifyPackedWasmChunk } from './verify-packed-wasm.mjs'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const pathFor = path => join(root, path)
const digest = bytes => createHash('sha256').update(bytes).digest('hex')
const hashFile = path => digest(readFileSync(pathFor(path)))
export const PREVIOUS_ORACLE = 'tests/fixtures/own-rust-cad-oracle-v1.json'
export const PREVIOUS_ORACLE_SHA256 = '4564124cc236974f353d66944cff953f4ae4fbd71f546d5c10310bf645c0a9c7'
export const ORACLE_REVIEW = 'docs/qualification/own-rust-cad-oracle-v2-review.md'
export const ORACLE_OUTPUT = 'tests/fixtures/own-rust-cad-oracle-v2.json'
export const ORACLE_CASES = Object.freeze([
  { id: 'colored-transform', source: 'color("#336699cc") translate([1,2,3]) cube([2,3,4]);', quality: 'full' },
  { id: 'boolean-difference', source: 'difference(){ cube([4,4,4], center=true); sphere(r=1,$fn=16); }', quality: 'full' },
  { id: 'repeated-loop-identities', source: 'module peg(x=0){translate([x,0,0]) sphere(r=1,$fn=12);} for(i=[0,2,0]) peg(i);', quality: 'full' },
  { id: 'reduced-preview', source: 'sphere(r=5,$fn=96);', quality: 'preview' },
  { id: 'reduced-preview-full-companion', source: 'sphere(r=5,$fn=96);', quality: 'full' },
])
const MIGRATED_CASES = new Set(['colored-transform', 'boolean-difference'])
const IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
const PALETTE = [[0.26, 0.52, 0.96, 1], [0.96, 0.52, 0.26, 1], [0.26, 0.86, 0.56, 1]]
const cubeOperation = 'op:root/call%3Acolor%230/children/call%3Atranslate%230/children/call%3Acube%230'
const cubeEntity = `entity:root>op:root/call%3Acolor%230>op:root/call%3Acolor%230/children/call%3Atranslate%230>${cubeOperation}`
const sphereOperation = 'op:root/call%3Asphere%230'
const repeatedOperation = 'op:root/module%3Apeg%230/body/call%3Atranslate%230/children/call%3Asphere%230'

function previousOracle() {
  assert.equal(hashFile(PREVIOUS_ORACLE), PREVIOUS_ORACLE_SHA256, 'The v1 oracle must remain byte-immutable')
  return JSON.parse(readFileSync(pathFor(PREVIOUS_ORACLE), 'utf8'))
}

function near(actual, expected, label, absolute = 1e-9, relative = 1e-9) {
  assert.ok(Number.isFinite(actual) && Math.abs(actual - expected) <= absolute + relative * Math.abs(expected),
    `${label}: expected ${expected}, received ${actual}`)
}

function values(view) {
  assert.ok(view?.bytes instanceof Uint8Array, 'Expected independent oracle byte view')
  assert.ok(view.type === 'Float32Array' || view.type === 'Uint32Array', 'Unexpected byte view type')
  assert.equal(view.bytes.byteLength, view.length * 4)
  const data = new DataView(view.bytes.buffer, view.bytes.byteOffset, view.bytes.byteLength)
  return Array.from({ length: view.length }, (_, i) => view.type === 'Float32Array'
    ? data.getFloat32(i * 4, true) : data.getUint32(i * 4, true))
}

function expectedMesh(id, index) {
  let entityId, operation, start, end, label, triangles, vertices, crease, min, max
  if (id === 'colored-transform') {
    entityId = cubeEntity; operation = cubeOperation; start = 38; end = 52; label = 'cube()'
    triangles = 12; vertices = 24; crease = 12; min = [1, 2, 3]; max = [3, 5, 7]
  } else if (id === 'boolean-difference') {
    entityId = 'entity:root>op:root/call%3Adifference%230'
    triangles = 236; vertices = 138; crease = 12; min = [-2, -2, -2]; max = [2, 2, 2]
  } else if (id === 'repeated-loop-identities') {
    const occurrence = ['0#0', '2#0', '0#1'][index]
    entityId = `entity:root>op:root/call%3Afor%230>loop:i=${occurrence}>op:root/call%3Afor%230/children/call%3Apeg%230>op:root/module%3Apeg%230/body/call%3Atranslate%230>${repeatedOperation}`
    operation = repeatedOperation; start = 35; end = 54; label = 'sphere()'
    triangles = 120; vertices = 62; crease = 24
    min = [index === 1 ? 1 : -1, -1, -1]; max = [index === 1 ? 3 : 1, 1, 1]
  } else {
    entityId = `entity:root>${sphereOperation}`; operation = sphereOperation; start = 0; end = 19; label = 'sphere()'
    const segments = id === 'reduced-preview' ? 48 : 96
    triangles = segments * (segments - 2); vertices = segments * (segments / 2 - 1) + 2
    crease = 0; min = [-5, -5, -5]; max = [5, 5, 5]
  }
  return {
    entityId, triangles, vertices, min, max,
    color: id === 'colored-transform' ? [0.2, 0.4, 0.6, 0.8] : PALETTE[index],
    topology: { boundary: 0, crease, nonManifold: 0, degenerate: 0 },
    provenance: [{ triangleStart: 0, triangleEnd: triangles, backside: false,
      source: operation ? { id: start, operationId: operation, instanceId: entityId,
        originalId: index, start, end, label } : null }],
  }
}

// Independent integration over the published Float32 triangles. No production
// geometry helpers or reported topology/metrics are used to compute this result.
function measureMesh(mesh) {
  const vertex = values(mesh.vertices), indices = values(mesh.indices)
  const points = [], welded = [], pointIds = new Map(), adjacency = [], edges = new Map()
  const bounds = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] }
  for (let i = 0; i < vertex.length; i += 6) {
    const point = vertex.slice(i, i + 3), normal = vertex.slice(i + 3, i + 6)
    assert.ok([...point, ...normal].every(Number.isFinite), 'Nonfinite render vertex')
    near(Math.hypot(...normal), 1, 'Unit render normal', 1e-6, 0)
    for (let axis = 0; axis < 3; axis++) {
      bounds.min[axis] = Math.min(bounds.min[axis], point[axis])
      bounds.max[axis] = Math.max(bounds.max[axis], point[axis])
    }
    const key = point.join(',')
    if (!pointIds.has(key)) { pointIds.set(key, points.length); points.push(point); adjacency.push(new Set()) }
    welded.push(pointIds.get(key))
  }
  const triangles = []
  for (let i = 0; i < indices.length; i += 3) {
    const render = indices.slice(i, i + 3)
    assert.ok(render.every(index => Number.isInteger(index) && index >= 0 && index < welded.length), 'Invalid index')
    const ids = render.map(index => welded[index]), [a, b, c] = ids.map(index => points[index])
    const ab = b.map((v, k) => v - a[k]), ac = c.map((v, k) => v - a[k])
    const cross = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]]
    const area = Math.hypot(...cross) / 2
    assert.ok(area > 0 && Number.isFinite(area), 'Degenerate published triangle')
    for (const index of render) {
      const normal = vertex.slice(index * 6 + 3, index * 6 + 6)
      assert.ok(normal.reduce((sum, v, k) => sum + v * cross[k], 0) > 0, 'Normal opposes triangle winding')
    }
    const volume = (a[0] * (b[1] * c[2] - b[2] * c[1])
      - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6
    triangles.push({ ids, area, volume })
    for (let k = 0; k < 3; k++) {
      const from = ids[k], to = ids[(k + 1) % 3]
      assert.notEqual(from, to, 'Collapsed welded triangle')
      adjacency[from].add(to); adjacency[to].add(from)
      const key = from < to ? `${from},${to}` : `${to},${from}`
      const edge = edges.get(key) ?? { count: 0, balance: 0 }
      edge.count++; edge.balance += from < to ? 1 : -1; edges.set(key, edge)
    }
  }
  for (const edge of edges.values()) assert.deepEqual(edge, { count: 2, balance: 0 }, 'Closed consistently oriented edge incidence')
  const componentIds = new Map(), components = []
  for (let start = 0; start < points.length; start++) {
    if (componentIds.has(start)) continue
    const id = components.length, stack = [start], component = { vertices: 0, edges: 0, triangles: 0, volume: 0, area: 0 }
    componentIds.set(start, id)
    while (stack.length) {
      const point = stack.pop(); component.vertices++; component.edges += adjacency[point].size
      for (const next of adjacency[point]) if (!componentIds.has(next)) { componentIds.set(next, id); stack.push(next) }
    }
    component.edges /= 2; components.push(component)
  }
  for (const triangle of triangles) {
    const component = components[componentIds.get(triangle.ids[0])]
    component.triangles++; component.volume += triangle.volume; component.area += triangle.area
  }
  for (const component of components) assert.equal(component.vertices - component.edges + component.triangles, 2, 'Closed genus-zero component')
  return { bounds, components: components.sort((a, b) => a.volume - b.volume),
    volume: components.reduce((sum, item) => sum + item.volume, 0), area: components.reduce((sum, item) => sum + item.area, 0) }
}

/** Frozen semantic requirements remain independent of newly captured hashes. */
export function assertOwnCadSemantics(fixture, outcome) {
  const previous = previousOracle()[fixture.id]
  assert.ok(previous && ORACLE_CASES.some(item => item.id === fixture.id && item.source === fixture.source && item.quality === fixture.quality), 'Unknown or modified oracle source')
  assert.equal(outcome.tag, 'success', fixture.id)
  const result = outcome.success
  assert.equal(result.quality, fixture.quality)
  assert.equal(result.reduced, fixture.id === 'reduced-preview')
  assert.deepEqual(result.warnings, fixture.id === 'reduced-preview' ? ['$fn=96 was clamped to 48 for preview rendering'] : [])
  near(result.volume, previous.volume, `${fixture.id} reported volume`)
  near(result.surfaceArea, previous.surfaceArea, `${fixture.id} reported area`)
  const count = fixture.id === 'repeated-loop-identities' ? 3 : 1
  assert.equal(result.meshes.length, count); assert.equal(result.scene.entities.length, count)
  assert.equal(result.scene.assets.length, count === 3 ? 2 : 1)
  const measurements = result.meshes.map((mesh, index) => {
    const expected = expectedMesh(fixture.id, index)
    assert.equal(mesh.entityId, expected.entityId); assert.deepEqual(mesh.color, expected.color)
    assert.deepEqual(mesh.provenance, expected.provenance); assert.deepEqual(mesh.topology, expected.topology)
    assert.equal(mesh.indices.length / 3, expected.triangles); assert.equal(mesh.vertices.length / 6, expected.vertices)
    assert.equal(mesh.faceIds.length, expected.triangles); assert.deepEqual(values(mesh.transform), IDENTITY)
    const measured = measureMesh(mesh)
    assert.deepEqual(measured.bounds, { min: expected.min, max: expected.max })
    const expectedVolumes = fixture.id === 'boolean-difference' ? [previous.volume - 64, 64] : [previous.volume / count]
    assert.equal(measured.components.length, expectedVolumes.length)
    measured.components.forEach((component, i) => near(component.volume, expectedVolumes[i], 'Independent component volume', 1e-6, 2e-7))
    return measured
  })
  near(measurements.reduce((sum, mesh) => sum + mesh.volume, 0), previous.volume, 'Independent total volume', 1e-6, 2e-7)
  near(measurements.reduce((sum, mesh) => sum + mesh.area, 0), previous.surfaceArea, 'Independent total area', 1e-6, 2e-7)
  if (count === 3) {
    assert.equal(new Set(result.meshes.map(mesh => mesh.entityId)).size, 3)
    assert.equal(result.meshes[0].geometryAssetId, result.meshes[2].geometryAssetId)
    assert.notEqual(result.meshes[0].geometryAssetId, result.meshes[1].geometryAssetId)
  }
  return measurements
}

function sourcePaths(directory) {
  return readdirSync(pathFor(directory), { withFileTypes: true }).flatMap(entry => {
    if (['target', 'generated', 'node_modules', '.git'].includes(entry.name)) return []
    const path = `${directory}/${entry.name}`
    if (path === 'src/core/ownRustCadEvidence.ts') return [] // separately generated release evidence
    return entry.isDirectory() ? sourcePaths(path) : entry.isFile() ? [path] : []
  })
}

export function oracleSourceFingerprint() {
  const paths = [...sourcePaths('src'), ...sourcePaths('crates'),
    'scripts/record-own-cad-oracle.mjs', 'scripts/build-geometry-kernels.mjs', 'scripts/build-wasm-brotli.mjs',
    'scripts/wasm-base85.mjs', 'scripts/verify-packed-wasm.mjs',
    'tests/support/referenceLegacyDirectEvaluatorOracle.ts', 'tests/legacyDirectEvaluatorOracle.test.ts',
    PREVIOUS_ORACLE, ORACLE_REVIEW, 'package.json', 'package-lock.json', 'rust-toolchain.toml',
    'tsconfig.json', 'tsconfig.mcp.json', 'vite.config.ts', 'vitest.config.ts', 'THIRD_PARTY_NOTICES.md'].sort()
  return { sha256: digest(paths.map(path => `${path}\0${hashFile(path)}\n`).join('')), files: paths.length }
}

function artifactFingerprint() {
  const wasm = readFileSync(pathFor('src/generated/geometry-kernels/kernel_bg.wasm'))
  const packed = readFileSync(pathFor('src/generated/geometry-kernels/bytes.ts'), 'utf8')
  verifyPackedWasmChunk(packed, wasm, 'geometry')
  return { wasmSha256: digest(wasm), packedSha256: digest(packed), decoderSha256: hashFile('src/generated/wasm-brotli/bytes.ts') }
}

async function capture() {
  const { parseOpenSCAD } = await import('../src/services/openscadParser.ts')
  const { referenceCaptureLegacyOutcome, referenceLegacyMeshBytes, referenceLegacySceneBytes, referenceLegacySha256 } = await import('../tests/support/referenceLegacyDirectEvaluatorOracle.ts')
  const previous = previousOracle(), cases = {}
  for (const fixture of ORACLE_CASES) {
    const outcome = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(fixture.source, { quality: fixture.quality }))
    const measurements = assertOwnCadSemantics(fixture, outcome)
    const mesh = referenceLegacyMeshBytes(outcome), scene = referenceLegacySceneBytes(outcome)
    const snapshot = { source: fixture.source, quality: fixture.quality,
      meshLength: mesh.length, meshHash: referenceLegacySha256(mesh), sceneLength: scene.length, sceneHash: referenceLegacySha256(scene),
      volume: outcome.success.volume, surfaceArea: outcome.success.surfaceArea, measurements }
    // The proven migration keeps record sizes and every unaffected snapshot.
    for (const field of ['meshLength', 'sceneLength']) assert.equal(snapshot[field], previous[fixture.id][field], `${fixture.id}.${field}`)
    if (!MIGRATED_CASES.has(fixture.id)) for (const field of ['meshHash', 'sceneHash']) assert.equal(snapshot[field], previous[fixture.id][field], `${fixture.id}.${field} is outside this migration`)
    cases[fixture.id] = snapshot
  }
  return cases
}

/** Publish a complete file without replacing an existing baseline, even in a race. */
export function publishOracleExclusive(outputPath, artifact, verifyStable) {
  const temporary = join(dirname(outputPath), `.${basename(outputPath)}.${randomUUID()}.tmp`)
  writeFileSync(temporary, `${JSON.stringify(artifact, null, 2)}\n`, { flag: 'wx' })
  try {
    verifyStable()
    linkSync(temporary, outputPath) // atomic complete-file publication; EEXIST preserves the old baseline
  } finally {
    unlinkSync(temporary)
  }
}

function record() {
  assert.ok(!existsSync(pathFor(ORACLE_OUTPUT)), `Refusing to overwrite ${ORACLE_OUTPUT}; a later migration needs a new reviewed version`)
  previousOracle()
  const source = oracleSourceFingerprint()
  const toolchain = { rustc: execFileSync('rustc', ['--version'], { cwd: root, encoding: 'utf8' }).trim(),
    cargo: execFileSync('cargo', ['--version'], { cwd: root, encoding: 'utf8' }).trim() }
  execFileSync(process.execPath, ['scripts/build-geometry-kernels.mjs'], { cwd: root, stdio: 'inherit', timeout: 300_000 })
  assert.deepEqual(oracleSourceFingerprint(), source, 'Sources changed during build; no oracle was written')
  const artifacts = artifactFingerprint()
  const outputs = []
  for (let i = 0; i < 2; i++) {
    const output = execFileSync(process.execPath, ['--import', 'tsx', 'scripts/record-own-cad-oracle.mjs', '--capture'],
      { cwd: root, encoding: 'utf8', timeout: 60_000, maxBuffer: 4 * 1024 * 1024 })
    outputs.push(JSON.parse(output))
    assert.deepEqual(oracleSourceFingerprint(), source, 'Sources changed during capture; no oracle was written')
    assert.deepEqual(artifactFingerprint(), artifacts, 'WASM/decoder changed during capture; no oracle was written')
  }
  assert.deepEqual(outputs[0], outputs[1], 'Fresh-process captures were not deterministic; no oracle was written')
  const artifact = { id: 'own-rust-cad-oracle-v2', generatedAt: new Date().toISOString(),
    previous: { path: PREVIOUS_ORACLE, sha256: PREVIOUS_ORACLE_SHA256 }, review: ORACLE_REVIEW,
    scope: 'Five fixed own-Rust direct evaluator cases; current LME1/LSE1 bytes, independently checked geometry and retained semantic metadata. Not Manifold binary compatibility or whole-application qualification.',
    source, artifacts, verification: { freshProcesses: 2, deterministic: true, semanticContract: 'own-rust-cad-oracle-v2' },
    environment: { platform: process.platform, architecture: process.arch, node: process.version, ...toolchain }, cases: outputs[0] }
  publishOracleExclusive(pathFor(ORACLE_OUTPUT), artifact, () => {
    assert.deepEqual(oracleSourceFingerprint(), source, 'Sources changed before publication; no oracle was written')
    assert.deepEqual(artifactFingerprint(), artifacts, 'WASM/decoder changed before publication; no oracle was written')
  })
  console.log(`Recorded ${ORACLE_OUTPUT}; retained v1 and verified two independent process captures.`)
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2)
  if (args.length === 1 && args[0] === '--capture') console.log(JSON.stringify(await capture()))
  else if (args.length === 2 && args[0] === '--version' && args[1] === '2') record()
  else throw new Error('Usage: node scripts/record-own-cad-oracle.mjs --version 2')
}
