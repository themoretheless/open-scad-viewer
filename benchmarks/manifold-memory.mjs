#!/usr/bin/env node
// Lifecycle diagnostic, not a latency benchmark: retains probes and forces GC.
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'
import { fileURLToPath } from 'node:url'
import { cpuFixtures } from './cpu-fixtures.mjs'

assert.equal(typeof global.gc, 'function', 'Run node --expose-gc --import tsx benchmarks/manifold-memory.mjs')
const root = fileURLToPath(new URL('../', import.meta.url))
const startedAt = new Date().toISOString()
const out = process.argv[2] ?? path.join(root, 'tmp', 'performance', `manifold-memory-${startedAt.replace(/[:.]/g, '-')}.json`)
const fixture = cpuFixtures.find(item => item.id === 'dense-sphere')
const fingerprintPaths = ['benchmarks/manifold-memory.mjs', 'benchmarks/cpu-fixtures.mjs', 'src/services/manifoldGeometryKernel.ts', 'src/services/openscadParser.ts', 'src/services/meshTopology.ts', 'node_modules/manifold-3d/manifold.js', 'node_modules/manifold-3d/manifold.wasm', 'node_modules/manifold-3d/lib/garbage-collector.js', 'package-lock.json']
const fingerprint = () => Promise.all(fingerprintPaths.map(async relative => ({ path: relative, sha256: createHash('sha256').update(await readFile(path.join(root, relative))).digest('hex') })))
const source = await fingerprint()
const wasmMemories = []
const instantiate = WebAssembly.instantiate
WebAssembly.instantiate = async function (...args) {
  const result = await Reflect.apply(instantiate, WebAssembly, args)
  const instance = result instanceof WebAssembly.Instance ? result : result.instance
  for (const value of Object.values(instance.exports)) if (value instanceof WebAssembly.Memory) wasmMemories.push(value)
  return result
}
let module
let parseOpenSCAD
try {
  const parser = await import('../src/services/openscadParser.ts')
  parseOpenSCAD = parser.parseOpenSCAD
  module = await parser.getWasm()
} finally { WebAssembly.instantiate = instantiate }
assert.ok(wasmMemories.length, 'Must capture the actual instantiated WASM memory')

const realPrototype = Object.getPrototypeOf(module.Manifold.sphere(1, 8))
const probes = []
for (const name of ['translate', 'calculateNormals']) {
  const method = realPrototype[name]
  realPrototype[name] = function (...args) {
    const result = Reflect.apply(method, this, args)
    probes.push({ name, object: result })
    return result
  }
}
const rows = []
let expectedHash
for (let iteration = 0; iteration < 15; iteration++) {
  let result = await parseOpenSCAD(fixture.source, { quality: 'full' })
  assert.deepEqual(result.warnings, [])
  const hash = createHash('sha256')
  for (const mesh of result.meshes) {
    hash.update(new Uint8Array(mesh.vertices.buffer, mesh.vertices.byteOffset, mesh.vertices.byteLength))
    hash.update(new Uint8Array(mesh.indices.buffer, mesh.indices.byteOffset, mesh.indices.byteLength))
  }
  const geometrySha256 = hash.digest('hex')
  expectedHash ??= geometrySha256
  assert.equal(geometrySha256, expectedHash)
  result = undefined
  global.gc()
  await new Promise(resolve => setImmediate(resolve))
  global.gc()
  rows.push({ iteration, memory: process.memoryUsage(), wasmMemoryBytes: wasmMemories.map(memory => memory.buffer.byteLength), probes: probes.map(probe => ({ operation: probe.name, deleted: probe.object.isDeleted() })) })
  probes.length = 0
}
assert.deepEqual(await fingerprint(), source, 'Measured sources changed during the memory diagnostic')
const report = { schemaVersion: 1, kind: 'manifold-memory-diagnostic', startedAt, completedAt: new Date().toISOString(), iterations: rows.length, environment: { node: process.version, platform: process.platform, architecture: process.arch, osRelease: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] }, fixture, source, sourceUnchangedAtEnd: true, geometrySha256: expectedHash, allProbedHandlesReleased: rows.every(row => row.probes.every(probe => probe.deleted)), tail5WasmCapacityStable: new Set(rows.slice(-5).map(row => JSON.stringify(row.wasmMemoryBytes))).size === 1, realPrototypeIsExportedPrototype: realPrototype === module.Manifold.prototype, wasmHeapObservation: 'Actual WebAssembly.Memory capacity captured during module instantiation; allocated native bytes/live Embind totals are not exported by this build. Stable capacity alone does not prove no native leak.', gcPolicy: 'Output dropped and JS GC forced twice per iteration. Native result probes stay referenced until after recording isDeleted; this is a lifecycle diagnostic, not timing evidence.', rows }
await mkdir(path.dirname(path.resolve(out)), { recursive: true })
await writeFile(out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
console.log(JSON.stringify({ report: path.resolve(out), first: rows[0], last: rows.at(-1) }, null, 2))
