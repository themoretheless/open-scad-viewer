// Qualification transport only. Reconstruction and every proof run in Rust WASM.
import assert from 'node:assert/strict'
import {readFileSync, writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {build} from 'esbuild'

const [wasmFile, baselineFile, outputFile, stageDirectory] = process.argv.slice(2)
assert(wasmFile && baselineFile && outputFile && stageDirectory,
  'Supply raw WASM, original four-body ABI receipt, output receipt and frozen stage')
const sha = bytes => createHash('sha256').update(bytes).digest('hex')
const codec = resolve(stageDirectory, 'src/services/valueBinaryCodec.ts')
const bundled = await build({entryPoints: [codec],
  bundle: true, write: false, platform: 'node', format: 'esm'})
const {encodeBinary, decodeBinary} = await import('data:text/javascript;base64,' + Buffer.from(bundled.outputFiles[0].contents).toString('base64'))
const bytes = readFileSync(wasmFile), baselineBytes = readFileSync(baselineFile)
const baseline = JSON.parse(baselineBytes)
assert.equal(baseline.completed, true)
assert.equal(baseline.cases.length, 4)
const {exports: wasm} = new WebAssembly.Instance(new WebAssembly.Module(bytes))
function request(value) {
  const encoded = encodeBinary(value), ptr = wasm.abi_alloc(encoded.length)
  assert(ptr, 'WASM request allocation failed')
  new Uint8Array(wasm.memory.buffer, ptr, encoded.length).set(encoded)
  try {
    const packed = wasm.abi_request(0, ptr, encoded.length)
    const responsePtr = Number(packed & 0xffffffffn), size = Number(packed >> 32n)
    let envelope
    try { envelope = decodeBinary(new Uint8Array(wasm.memory.buffer, responsePtr, size)) }
    finally { wasm.abi_free(responsePtr, size) }
    assert.equal(envelope.ok, true, JSON.stringify(envelope.error))
    return envelope.value
  } finally { wasm.abi_free(ptr, encoded.length) }
}
const report = {schema: 'closed-frenet-arc-abi/1',
  scope: 'Exact supplied WASM ABI and original source/full-global report parity; no packaged Rush/UI or all-mode proof',
  sha256: sha(bytes), byteLength: bytes.length, originalReceiptSha256: sha(baselineBytes),
  codecSourceSha256: sha(readFileSync(codec)), completed: false, cases: []}
for (const original of baseline.cases) {
  const source = structuredClone(original.source)
  const result = request({op: 'brep_nurbs_progressive_profile_body_reconstruct_stations', ...source})
  const retained = result.stationReconstruction, model = retained.reconstruction.model
  assert.deepEqual(source, original.source)
  assert.equal(retained.boundaryCertificate.continuousBound, true)
  assert.equal(retained.boundaryCertificate.withinBudget, true)
  assert.deepEqual(retained.boundaryCertificate, original.boundaryCertificate)
  assert.deepEqual(retained.profileSmoothness, original.profileSmoothness)
  assert.equal(model.faces.length, original.faceCount)
  const volume = request({op: 'brep_sweep_volume_charts_audit', model, capFaces: [],
    toleranceUv: 1e-8, maxExactWork: 1000000, maxTrimPairs: 10000, maxTrimCells: 100000, maxTrimDomainCells: 1000000,
    maxSpans: 1024, maxLinearCells: 100000, maxPairs: 20000, maxCells: 200000, maxDomainCells: 1000000,
    cellsPerPair: 1000, domainCellsPerPair: 10000, nestingPairs: 1000, nestingCells: 100000, nestingDomainCells: 1000000,
    orientationCells: 100000, orientationDomainCells: 1000000, orientationSpans: 100,
    capBudgets: {maxWalls: 1024, maxExactWork: 1000000, maxChartCells: 1000, maxTrimPairs: 100000, maxTrimCells: 100000, maxTrimDomainCells: 1000000}})
  assert.equal(volume.solidGeometryCertified, true)
  assert.deepEqual(volume, original.volume)
  report.cases.push({periodic: original.periodic, hollow: original.hollow, source,
    boundaryCertificate: retained.boundaryCertificate, profileSmoothness: retained.profileSmoothness,
    faceCount: model.faces.length, volume})
  writeFileSync(outputFile, JSON.stringify(report, null, 2) + '\n')
  console.log(JSON.stringify({periodic: original.periodic, hollow: original.hollow, faceCount: model.faces.length, completeReportParity: true}))
}
report.completed = true
writeFileSync(outputFile, JSON.stringify(report, null, 2) + '\n')
