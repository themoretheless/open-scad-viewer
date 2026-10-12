// Qualification transport only. Reconstruction and every proof run in Rust WASM.
import assert from 'node:assert/strict'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'
import {readFileSync, writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {build} from 'esbuild'

const [wasmFile, captureFile, outputFile, stageDirectory] = process.argv.slice(2)
assert(wasmFile && captureFile && outputFile && stageDirectory,
  'Supply raw WASM, original Rust-captured FixedNormal request, output receipt and frozen stage')
const sha = bytes => createHash('sha256').update(bytes).digest('hex')
const codec = resolve(stageDirectory, 'src/services/valueBinaryCodec.ts')
const bundled = await build({entryPoints: [codec],
  bundle: true, write: false, platform: 'node', format: 'esm'})
const {encodeBinary, decodeBinary} = await import('data:text/javascript;base64,' + Buffer.from(bundled.outputFiles[0].contents).toString('base64'))
const bytes = readFileSync(wasmFile), baselineBytes = readFileSync(captureFile)
const captured = JSON.parse(baselineBytes)
assert.equal(captured.orientation, "fixed_normal")
assert.equal(captured.spacing, "arc_length")
assert.equal(captured.path.periodic, true)
assert.equal(captured.loops.length, 2)
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
const report = {schema:'periodic-fixed-normal-arc-abi/1', scope:'Four original Rust-captured plain/affine solid/hollow sources; Rust WASM proofs; no UI or all-mode proof', sha256:sha(bytes), sourceCaptureSha256:sha(baselineBytes), codecSourceSha256:sha(readFileSync(codec)),completed:false,cases:[]}
for (const joint of [false,true]) for (const hollow of [false,true]) {
  const source = structuredClone(captured)
  if (!joint) { delete source.axis_scale; delete source.center_law }
  if (!hollow) source.loops = source.loops.slice(0,1)
  const original = structuredClone(source)
  const result = request(source), retained = result.stationReconstruction, model=retained.reconstruction.model
  assert.deepEqual(source,original)
  assert.equal(retained.boundaryCertificate.continuousBound,true)
  assert.equal(retained.boundaryCertificate.withinBudget,true)
  assert.equal(retained.boundaryCertificate.budget,0.5)
  assert.equal(retained.profileSmoothness.profile.exactG1G2Certified,true)
  assert.equal(retained.profileSmoothness.station.stationG2Certified,true)
  assert.equal(model.faces.length,hollow?256:128)
  assert.equal(model.shells.length,hollow?2:1)
  const volume = request({op: 'brep_sweep_volume_charts_audit', model, capFaces: [],
    toleranceUv: 1e-8, maxExactWork: 1000000, maxTrimPairs: 10000, maxTrimCells: 100000, maxTrimDomainCells: 1000000,
    maxSpans: 1024, maxLinearCells: 100000, maxPairs: 20000, maxCells: 200000, maxDomainCells: 1000000,
    cellsPerPair: 1000, domainCellsPerPair: 10000, nestingPairs: 1000, nestingCells: 100000, nestingDomainCells: 1000000,
    orientationCells: 100000, orientationDomainCells: 1000000, orientationSpans: 100,
    capBudgets: {maxWalls: 1024, maxExactWork: 1000000, maxChartCells: 1000, maxTrimPairs: 100000, maxTrimCells: 100000, maxTrimDomainCells: 1000000}})
  assert.equal(volume.solidGeometryCertified, true)
  assert.equal(volume.allFacesInjective,true)
  assert.equal(volume.allPairsClassified,true)
  assert.equal(volume.retainedWallCharts.allChartsCertified,true)
  const pairCover=checkCompleteFacePairCover(volume.boundary,model.faces.length)
  report.cases.push({joint,hollow,pairCover,source,boundaryCertificate:retained.boundaryCertificate,profileSmoothness:retained.profileSmoothness,faceCount:model.faces.length,volume})
  writeFileSync(outputFile,JSON.stringify(report,null,2)+'\n')
  console.log(JSON.stringify({joint,hollow,faces:model.faces.length,fullGlobalCertified:true}))
}
report.completed=true
writeFileSync(outputFile,JSON.stringify(report,null,2)+'\n')
