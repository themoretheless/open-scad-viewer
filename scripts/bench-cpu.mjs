#!/usr/bin/env node
// Deliberately separate instrumented profile passes from uninstrumented timings.
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync, spawn } from 'node:child_process'
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { Session } from 'node:inspector'
import os from 'node:os'
import path from 'node:path'
import { performance, PerformanceObserver, constants } from 'node:perf_hooks'
import { fileURLToPath } from 'node:url'
import { cpuFixtures } from '../benchmarks/cpu-fixtures.mjs'

const root = fileURLToPath(new URL('../', import.meta.url))
const script = fileURLToPath(import.meta.url)
const operationNames = ['build', 'bvh', 'edges-position-weld', 'asset-hash', 'result-validation', 'scene-publication', 'inspection', 'stl']
const turn = () => new Promise(resolve => setImmediate(resolve))
const sha256 = value => createHash('sha256').update(value).digest('hex')

function optionsFromArgs(args) {
  const options = { iterations: 9, warmups: 2, fixtures: cpuFixtures.map(f => f.id), profiles: [], profileIterations: 3, timeoutMs: 180_000 }
  for (let index = 0; index < args.length; index++) {
    const arg = args[index]
    if (arg === '--help' || arg === '-h') return { help: true }
    if (arg === '--quick') { options.iterations = 3; options.warmups = 1; continue }
    const [key, inline] = arg.split('=', 2)
    const value = inline ?? args[++index]
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${key}`)
    if (key === '--iterations') options.iterations = Number(value)
    else if (key === '--warmups') options.warmups = Number(value)
    else if (key === '--fixtures') options.fixtures = value.split(',')
    else if (key === '--profiles') options.profiles = value.split(',')
    else if (key === '--profile-iterations') options.profileIterations = Number(value)
    else if (key === '--timeout-ms') options.timeoutMs = Number(value)
    else if (key === '--out') options.out = path.resolve(value)
    else if (key === '--worker') options.worker = value
    else throw new Error(`Unknown argument ${key}`)
  }
  for (const key of ['iterations', 'profileIterations']) assert.ok(Number.isSafeInteger(options[key]) && options[key] >= 1 && options[key] <= 100, `${key} must be 1..100`)
  assert.ok(Number.isSafeInteger(options.warmups) && options.warmups >= 0 && options.warmups <= 20, 'warmups must be 0..20')
  assert.ok(Number.isSafeInteger(options.timeoutMs) && options.timeoutMs >= 1_000 && options.timeoutMs <= 900_000, 'timeout-ms must be 1000..900000')
  assert.ok(options.fixtures.length && new Set(options.fixtures).size === options.fixtures.length, 'Select unique fixture IDs')
  for (const id of options.fixtures) assert.ok(cpuFixtures.some(f => f.id === id), `Unknown fixture ${id}`)
  for (const name of options.profiles) assert.ok(operationNames.includes(name), `Unknown profile operation ${name}`)
  assert.equal(new Set(options.profiles).size, options.profiles.length, 'Select unique profile operations')
  return options
}

function stats(values) {
  const sorted = [...values].sort((a, b) => a - b)
  assert.ok(sorted.length && sorted.every(Number.isFinite), 'Measurements must be finite')
  const at = percentile => sorted[Math.max(0, Math.ceil(percentile * sorted.length) - 1)]
  return { n: sorted.length, min: sorted[0], p50: at(0.5), p95: at(0.95), max: sorted.at(-1), mean: sorted.reduce((a, b) => a + b, 0) / sorted.length }
}

function memory() { return process.memoryUsage() }
function difference(after, before) { return Object.fromEntries(Object.keys(before).map(key => [key, after[key] - before[key]])) }
function bytes(view) { return new Uint8Array(view.buffer, view.byteOffset, view.byteLength) }

async function sourceSnapshot() {
  const paths = []
  async function visit(relative) {
    for (const entry of await readdir(path.join(root, relative), { withFileTypes: true })) {
      const child = path.posix.join(relative, entry.name)
      if (entry.isDirectory()) await visit(child)
      else if (entry.isFile()) paths.push(child)
    }
  }
  for (const directory of ['src', 'scripts', 'benchmarks']) await visit(directory)
  for (const name of await readdir(root)) {
    if (/^(package(-lock)?\.json|tsconfig.*\.json|vite\.config\..*|env\.d\.ts)$/.test(name)) paths.push(name)
  }
  paths.push('crates/Cargo.lock')
  await visit('crates/geometry-bridge/src')
  await visit('crates/polygon-core/src')
  const files = await Promise.all(paths.sort().map(async name => ({ path: name, sha256: sha256(await readFile(path.join(root, name))) })))
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 }).trim()
  return {
    head: git('rev-parse', 'HEAD'),
    status: git('status', '--short', '--untracked-files=all'),
    inputSha256: sha256(JSON.stringify(files)),
    files,
  }
}

function buildSignature(result) {
  assert.ok(result.meshes.length > 0 && Number.isFinite(result.volume) && result.volume > 0, 'Build must produce a finite solid')
  assert.deepEqual(result.warnings, [], 'Fixture warnings are benchmark failures')
  const hash = createHash('sha256')
  for (const mesh of result.meshes) {
    assert.ok(mesh.vertices.length && mesh.indices.length && mesh.indices.length % 3 === 0, 'Invalid mesh')
    hash.update(bytes(mesh.vertices)); hash.update(bytes(mesh.indices))
  }
  return { sha256: hash.digest('hex'), meshes: result.meshes.length, vertices: result.meshes.reduce((n, m) => n + m.vertices.length / 6, 0), triangles: result.meshes.reduce((n, m) => n + m.indices.length / 3, 0), geometryBytes: result.meshes.reduce((n, m) => n + m.vertices.byteLength + m.indices.byteLength, 0), volume: result.volume, surfaceArea: result.surfaceArea }
}

function summarize(samples) {
  const result = {}
  for (const name of operationNames) {
    const selected = samples.filter(sample => sample.operation === name)
    result[name] = {
      wallMs: stats(selected.map(s => s.wallMs)),
      cpuUserMs: stats(selected.map(s => s.cpuUserMs)),
      cpuSystemMs: stats(selected.map(s => s.cpuSystemMs)),
      gcCount: selected.reduce((n, s) => n + s.gc.length, 0),
      gcDurationMs: selected.reduce((n, s) => n + s.gc.reduce((total, event) => total + event.durationMs, 0), 0),
      netHeapUsedBytes: stats(selected.map(s => s.memoryDelta.heapUsed)),
      netExternalBytes: stats(selected.map(s => s.memoryDelta.external)),
      netArrayBuffersBytes: stats(selected.map(s => s.memoryDelta.arrayBuffers)),
    }
    if (name === 'build') result[name].phases = Object.fromEntries(['parseMs', 'initializeMs', 'evaluateMs', 'analyzeMs'].map(phase => [phase, stats(selected.map(s => s.phases[phase]))]))
  }
  return result
}

function frameLabel(frame) {
  return `${frame.functionName || '(anonymous)'} ${frame.url || '(native)'}:${frame.lineNumber + 1}:${frame.columnNumber + 1}`
}

async function profileOperation(name, operation, validate, options) {
  const session = new Session()
  session.connect()
  const post = (method, params = {}) => new Promise((resolve, reject) => session.post(method, params, (error, result) => error ? reject(error) : resolve(result)))
  try {
    global.gc()
    await post('HeapProfiler.enable')
    await post('HeapProfiler.startSampling', { samplingInterval: 32_768, includeObjectsCollectedByMajorGC: true, includeObjectsCollectedByMinorGC: true })
    let profileOutput
    for (let index = 0; index < options.profileIterations; index++) profileOutput = await operation()
    const { profile: allocation } = await post('HeapProfiler.stopSampling')
    validate(profileOutput)
    profileOutput = undefined
    const allocations = []
    function walk(node, stack) {
      const nextStack = [...stack, frameLabel(node.callFrame)]
      if (node.selfSize) allocations.push({ sampledBytes: node.selfSize, stack: nextStack })
      for (const child of node.children) walk(child, nextStack)
    }
    walk(allocation.head, [])
    allocations.sort((a, b) => b.sampledBytes - a.sampledBytes)
    const heapFile = `${name}.heapprofile`
    await writeFile(path.join(options.out, heapFile), JSON.stringify(allocation))
    // A new pass prevents allocation-sampler overhead from contaminating the CPU profile.
    await post('HeapProfiler.disable')
    global.gc()
    await post('Profiler.enable')
    await post('Profiler.setSamplingInterval', { interval: 1_000 })
    await post('Profiler.start')
    for (let index = 0; index < options.profileIterations; index++) profileOutput = await operation()
    const { profile: cpu } = await post('Profiler.stop')
    validate(profileOutput)
    profileOutput = undefined
    await post('Profiler.disable')
    const cpuFile = `${name}.cpuprofile`
    await writeFile(path.join(options.out, cpuFile), JSON.stringify(cpu))
    const sampleCounts = new Map()
    for (const id of cpu.samples ?? []) sampleCounts.set(id, (sampleCounts.get(id) ?? 0) + 1)
    const byFunction = new Map()
    for (const node of cpu.nodes) if (sampleCounts.has(node.id)) {
      const frame = frameLabel(node.callFrame)
      byFunction.set(frame, (byFunction.get(frame) ?? 0) + sampleCounts.get(node.id))
    }
    const hotFunctions = [...byFunction].map(([frame, samples]) => ({ frame, samples })).sort((a, b) => b.samples - a.samples)
    return { operation: name, iterations: options.profileIterations, heapFile, cpuFile, allocationSamplingIntervalBytes: 32_768, collectedObjectsIncluded: true, sampledAllocationBytes: allocations.reduce((n, item) => n + item.sampledBytes, 0), topAllocationStacks: allocations.slice(0, 20), cpuSamplingIntervalMicroseconds: 1_000, cpuSamples: cpu.samples?.length ?? 0, topCpuFunctions: hotFunctions.slice(0, 25) }
  } finally { session.disconnect() }
}

async function worker(options) {
  assert.equal(typeof global.gc, 'function', 'Worker requires --expose-gc')
  const fixture = cpuFixtures.find(item => item.id === options.worker)
  assert.ok(fixture, 'Worker fixture does not exist')
  await mkdir(options.out, { recursive: true })
  const sourceStart = await sourceSnapshot()
  const importStart = performance.now()
  const [{ parseOpenSCAD }, { buildMeshBvh }, { extractSemanticEdges }, { geometryAssetId, geometrySceneFromMeshes, geometrySceneTransferables }, { inspectMesh }, { buildBinaryStl }, { isGeometryEvaluationResultPayload }] = await Promise.all([
    import('../src/services/openscadParser.ts'), import('../src/services/meshBvh.ts'), import('../src/services/meshTopology.ts'), import('../src/core/scene.ts'), import('../src/services/meshInspection.ts'), import('../src/services/meshExport.ts'),
    import('../src/services/geometryWorkerProtocol.ts'),
  ])
  const moduleImportMs = performance.now() - importStart
  const gcEvents = []
  const observer = new PerformanceObserver(list => {
    for (const event of list.getEntries()) gcEvents.push({ startMs: event.startTime, durationMs: event.duration, kind: event.detail.kind, forced: (event.detail.flags & constants.NODE_PERFORMANCE_GC_FLAGS_FORCED) !== 0 })
  })
  observer.observe({ entryTypes: ['gc'] })
  let meshes
  let lastBuild
  let expectedBuild
  const operations = {
    build: () => parseOpenSCAD(fixture.source, { quality: 'full' }),
    bvh: () => meshes.map(mesh => buildMeshBvh(mesh.vertices, mesh.indices)),
    'edges-position-weld': () => meshes.map(mesh => extractSemanticEdges(mesh.vertices, mesh.indices, { creaseAngleDegrees: 30 })),
    'asset-hash': () => meshes.map(mesh => geometryAssetId(mesh.vertices, mesh.indices)),
    'result-validation': () => isGeometryEvaluationResultPayload(lastBuild),
    'scene-publication': () => { const scene = geometrySceneFromMeshes(meshes); return { scene, transferables: geometrySceneTransferables(scene) } },
    inspection: () => meshes.map((mesh, index) => inspectMesh(mesh, index)),
    stl: () => buildBinaryStl(meshes, 'CPU benchmark'),
  }
  const expectedOutputs = new Map()
  function validate(name, value) {
    let signature
    if (name === 'build') {
      signature = buildSignature(value)
      if (!expectedBuild) expectedBuild = signature
      else assert.deepEqual(signature, expectedBuild, 'Geometry must be deterministic across iterations')
      meshes = value.meshes
      lastBuild = value
      return
    }
    if (name === 'stl') {
      const triangleCount = new DataView(value.buffer, value.byteOffset, value.byteLength).getUint32(80, true)
      assert.ok(triangleCount > 0 && triangleCount <= expectedBuild.triangles)
      assert.equal(value.byteLength, 84 + triangleCount * 50)
      signature = { bytes: value.byteLength, triangles: triangleCount, sha256: sha256(value) }
    } else if (name === 'scene-publication') {
      assert.equal(value.scene.entities.length, meshes.length)
      signature = { assets: value.scene.assets.length, entities: value.scene.entities.length, transferBytes: value.transferables.reduce((n, buffer) => n + buffer.byteLength, 0), buffers: value.transferables.length }
    } else if (name === 'result-validation') { assert.equal(value, true, 'Deep result validation failed'); signature = value }
    else if (name === 'asset-hash') signature = value
    else if (name === 'inspection') {
      assert.ok(value.every(item => item.bounds && item.triangles > 0))
      signature = value
    } else {
      const hash = createHash('sha256')
      for (const item of value) {
        for (const field of name === 'bvh' ? ['bounds', 'nodes', 'triangles'] : ['indices']) hash.update(bytes(item[field]))
      }
      signature = hash.digest('hex')
    }
    if (!expectedOutputs.has(name)) expectedOutputs.set(name, signature)
    else assert.deepEqual(signature, expectedOutputs.get(name), `${name} output changed`)
  }
  async function measure(name, iteration) {
    const before = memory()
    const cpuStart = process.cpuUsage()
    const startMs = performance.now()
    const value = await operations[name]()
    const endMs = performance.now()
    const cpu = process.cpuUsage(cpuStart)
    const after = memory()
    // Validation/checksums and observer delivery happen outside the measured interval.
    validate(name, value)
    await turn(); await turn()
    const sample = { operation: name, iteration, wallMs: endMs - startMs, cpuUserMs: cpu.user / 1000, cpuSystemMs: cpu.system / 1000, memoryBefore: before, memoryAfter: after, memoryDelta: difference(after, before), gc: gcEvents.filter(event => event.startMs >= startMs && event.startMs < endMs) }
    if (name === 'build') sample.phases = value.timings
    return sample
  }
  global.gc()
  const beforeCold = memory()
  const coldBuild = await measure('build', 0)
  for (let iteration = 0; iteration < options.warmups; iteration++) {
    for (const name of operationNames) validate(name, await operations[name]())
  }
  global.gc()
  await turn(); await turn()
  const beforeSamples = memory()
  const samples = []
  for (let iteration = 0; iteration < options.iterations; iteration++) {
    for (const name of operationNames) samples.push(await measure(name, iteration))
  }
  const afterSamples = memory()
  const outputSignatures = Object.fromEntries(expectedOutputs)
  const profiles = []
  for (const name of options.profiles) profiles.push(await profileOperation(name, operations[name], value => validate(name, value), options))
  meshes = undefined
  lastBuild = undefined
  global.gc(); await turn(); global.gc(); await turn()
  const afterReleaseAndGc = memory()
  observer.disconnect()
  const sourceEnd = await sourceSnapshot()
  assert.equal(sourceEnd.inputSha256, sourceStart.inputSha256, 'Source changed during benchmark; discard and rerun with a stable checkout')
  const result = {
    schemaVersion: 1, kind: 'cpu-fixture', fixture: { ...fixture, sourceSha256: sha256(fixture.source), sourceBytes: Buffer.byteLength(fixture.source) },
    source: sourceStart, sourceUnchangedAtEnd: true, moduleImportMs, geometry: expectedBuild, coldBuild,
    iterations: options.iterations, warmups: options.warmups, stageSummaries: summarize(samples), samples,
    memory: { beforeCold, beforeSamples, afterSamples, afterReleaseAndGc, releasedRetainedDeltaBytes: difference(afterReleaseAndGc, beforeSamples), maxRssBytes: process.resourceUsage().maxRSS * 1024 },
    outputSignatures, profiles,
  }
  await writeFile(path.join(options.out, 'result.json'), `${JSON.stringify(result, null, 2)}\n`)
}

async function runChild(fixture, options) {
  const out = path.join(options.out, fixture.id)
  const args = ['--expose-gc', '--import', 'tsx', script, '--worker', fixture.id, '--out', out, '--iterations', String(options.iterations), '--warmups', String(options.warmups), '--profile-iterations', String(options.profileIterations)]
  if (options.profiles.length) args.push('--profiles', options.profiles.join(','))
  await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, args, { cwd: root, stdio: ['ignore', 'inherit', 'inherit'] })
    const timer = setTimeout(() => { child.kill('SIGKILL'); reject(new Error(`${fixture.id} exceeded ${options.timeoutMs} ms`)) }, options.timeoutMs)
    child.on('error', error => { clearTimeout(timer); reject(error) })
    child.on('exit', (code, signal) => { clearTimeout(timer); code === 0 ? resolve() : reject(new Error(`${fixture.id} failed: exit=${code} signal=${signal}`)) })
  })
  return JSON.parse(await readFile(path.join(out, 'result.json'), 'utf8'))
}

async function main() {
  const options = optionsFromArgs(process.argv.slice(2))
  if (options.help) {
    console.log(`CPU/Manifold benchmark (Node only; no browser Worker or GPU timing)
node scripts/bench-cpu.mjs [--quick] [--iterations 9] [--warmups 2]
  [--fixtures ${cpuFixtures.map(f => f.id).join(',')}]
  [--profiles build,stl] [--profile-iterations 3] [--timeout-ms 180000] [--out directory]
Profile operations: ${operationNames.join(', ')}
--profiles saves allocation and CPU profiles in separate instrumented passes.
No wall-clock pass/fail thresholds. Invalid or changed outputs fail the run.`)
    return
  }
  if (options.worker) { await worker(options); return }
  const startedAt = new Date().toISOString()
  options.out ??= path.join(root, 'tmp', 'performance', `cpu-${startedAt.replace(/[:.]/g, '-')}`)
  // Do not overwrite an earlier run or unrelated user output.
  await mkdir(path.dirname(options.out), { recursive: true })
  await mkdir(options.out, { recursive: false })
  const source = await sourceSnapshot()
  const manifest = {
    schemaVersion: 1, kind: 'cpu-benchmark', startedAt, invocation: [process.execPath, ...process.argv.slice(1)], options, source,
    environment: { node: process.version, versions: process.versions, platform: process.platform, architecture: process.arch, osRelease: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))], logicalCpus: os.cpus().length, availableParallelism: os.availableParallelism(), totalMemoryBytes: os.totalmem(), loadAverageAtStart: os.loadavg(), execArgv: process.execArgv, gc: 'explicit major collection outside timing batches only' },
    boundaries: [
      'Node/V8 main process and browser subset Manifold WASM, full quality; not official OpenSCAD or browser end-to-end latency.',
      'Fresh process per fixture. Cold build includes first kernel initialization, excludes Node startup and module import (reported separately).',
      'Sequential uninstrumented stage wall time and process CPU usage. Forced GC only outside timing batches; natural GC remains included.',
      'Process CPU includes V8 background threads and can exceed wall time. Node CPU/heap profile frame coordinates follow tsx-generated code; use function names and source maps when inspecting them.',
      'Build phases are production parser timings. Evaluate includes cooperative event-loop yields; analyze includes normals, mesh copies, BVH, semantic edges and hashes.',
      'Other operations replay actual output meshes independently; their times do not partition analyzeMs and must not be summed into build time.',
      'Edges replay uses exact-position welding because the published mesh omits Manifold weld hints; production extraction uses those hints.',
      'Scene publication covers the scene adapter plus transferable collection/validation; excludes worker transport, host reconciliation, Vue and GPU.',
      'Result validation uses the standalone result validator with the same deep mesh/BVH checks as Worker-v5 publication; excludes the Worker envelope and transport.',
      'Memory deltas are net process snapshots, not total allocations or exact peaks. external includes arrayBuffers; do not sum these fields. WASM/native allocator internals are not measured.',
      'After-release GC snapshots also include benchmark metadata, JIT/runtime caches and profiles; they do not prove a leak or retained application geometry.',
      'Allocation profiles sample JS heap allocation including collected objects at 32 KiB intervals; estimates are not exact allocated bytes and omit typed-array backing stores and WASM allocations.',
      'Allocation and CPU profiles are separate passes after timing; final output validation/checksums occur after each profiler stops.',
      'Percentiles use nearest rank. Small sample p95 is descriptive only; no CI timing thresholds or production capacity claims.',
    ],
  }
  await writeFile(path.join(options.out, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`)
  const results = []
  try {
    for (const id of options.fixtures) {
      const fixture = cpuFixtures.find(item => item.id === id)
      console.log(`Benchmarking ${id} (${options.iterations} samples, ${options.warmups} warmups)...`)
      const result = await runChild(fixture, options)
      assert.equal(result.source.inputSha256, source.inputSha256, 'Source changed between fixtures; discard and rerun')
      results.push(result)
      console.log(`  ${result.geometry.triangles.toLocaleString()} triangles; build p50 ${result.stageSummaries.build.wallMs.p50.toFixed(2)} ms, analyze ${result.stageSummaries.build.phases.analyzeMs.p50.toFixed(2)} ms, STL ${result.stageSummaries.stl.wallMs.p50.toFixed(2)} ms`)
    }
    assert.equal((await sourceSnapshot()).inputSha256, source.inputSha256, 'Source changed during the benchmark suite')
    await writeFile(path.join(options.out, 'report.json'), `${JSON.stringify({ ...manifest, completedAt: new Date().toISOString(), status: 'complete', results }, null, 2)}\n`)
    console.log(`Report: ${path.join(options.out, 'report.json')}`)
  } catch (error) {
    await writeFile(path.join(options.out, 'failure.json'), `${JSON.stringify({ status: 'failed', message: error.message, completedFixtures: results.map(r => r.fixture.id) }, null, 2)}\n`)
    throw error
  }
}

main().catch(error => { console.error(error.stack ?? error); process.exitCode = 1 })
