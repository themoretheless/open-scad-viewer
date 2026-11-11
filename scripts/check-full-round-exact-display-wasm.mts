import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { encodeBinary, decodeBinary } from '../src/app/valueBinaryCodec.ts'
import { lowerOpenSCADToSemanticProgram } from '../src/language/semantic/semanticProgramLowerer.ts'

const [wasmPath, directory, nativePath] = process.argv.slice(2)
assert(wasmPath && directory && nativePath, 'Expected WASM, report directory and native frozen artifact')
mkdirSync(directory, { recursive: true })
const bytes = readFileSync(wasmPath), module = await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module), [])
const instance = await WebAssembly.instantiate(module, {})
const wasm = instance.exports as unknown as { memory: WebAssembly.Memory; abi_alloc(n:number):number; abi_free(p:number,n:number):void; abi_request(op:number,p:number,n:number):bigint }
function request(value: unknown): any {
  const data = encodeBinary(value), ptr = wasm.abi_alloc(data.length)
  assert(ptr)
  new Uint8Array(wasm.memory.buffer, ptr, data.length).set(data)
  try {
    const packed = wasm.abi_request(0, ptr, data.length), p = Number(packed & 0xffffffffn), n = Number(packed >> 32n)
    assert(p && n)
    try { return decodeBinary(new Uint8Array(wasm.memory.buffer, p, n)) }
    finally { wasm.abi_free(p, n) }
  } finally { wasm.abi_free(ptr, data.length) }
}
const cases: object[] = []
function run(name: string, value: unknown, ok: boolean): any {
  const start = performance.now(), result = request(value)
  assert.equal(result.ok, ok, `${name}: ${JSON.stringify(result.error)}`)
  const row = { name, ok, elapsedMs: performance.now() - start, error: result.error }
  cases.push(row); console.log(JSON.stringify(row))
  return result.value
}
const source = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\nfull_round_exact(faces=[5,4,3]) cube([4,8,6]);\n'
const trusted = lowerOpenSCADToSemanticProgram(source), program = trusted.program
writeFileSync(resolve(directory, 'source.scad'), source)
writeFileSync(resolve(directory, 'source-program.json'), JSON.stringify(program,null,2)+'\n')
const graph = { ...program.core, op:'brep_graph', envelope:program, source:program.source, sourceText:source,
  provenance:program.provenance, tessellationIntents:program.tessellationIntents, diagnostics:program.diagnostics,
  maxNodes:4, maxBytes:4194304 }
const result = run('source-attested-graph', graph, true)
const geometry = result.outcomes[0].geometry, artifact = geometry.artifact
assert.equal(geometry.representation, 'rational-brep')
assert.deepEqual(geometry.model, artifact.geometry)
const frozen = JSON.stringify(artifact)
const volumeErrors: number[] = []
function checkMesh(mesh: any, owner: any) {
  assert.equal(mesh.representation, 'display-mesh')
  assert.equal(mesh.sourceRepresentation, 'rational-brep')
  assert.equal(mesh.geometricErrorCertified, false)
  assert.equal(mesh.report.errorBoundCertified, false)
  assert.equal(mesh.report.closed, true)
  assert.equal(mesh.report.orientationConflicts, 0)
  assert.equal(mesh.report.degenerateTriangles, 0)
  assert.deepEqual(mesh.topologyFaceIds, owner.geometry.topologyIds.faces)
  assert.equal(mesh.faceIds.length * 3, mesh.displayIndices.length)
  assert(mesh.faceIds.every((id:number) => Number.isInteger(id) && id >= 0 && id < mesh.topologyFaceIds.length))
  assert(mesh.displayVertices.every(Number.isFinite))
  assert(mesh.surfaceArea > 0 && Number.isFinite(mesh.surfaceArea))
}
for (const segments of [2,8,16]) {
  const mesh = run(`exact-display-${segments}`, {op:'cad_full_round_exact_display',artifact,segments}, true)
  checkMesh(mesh, artifact)
  volumeErrors.push(6*(24+2*Math.PI)-mesh.report.signedVolumeMm3)
  if (segments === 16) writeFileSync(resolve(directory, 'box-display.json'), JSON.stringify(mesh)+'\n')
}
assert(volumeErrors.every(e => e > 0))
assert(volumeErrors[2] < volumeErrors[1] && volumeErrors[1] < volumeErrors[0])
assert(volumeErrors[2] < 0.1 && volumeErrors[2] < volumeErrors[0]/20)
assert.equal(JSON.stringify(artifact), frozen)
const nativeBytes = readFileSync(nativePath), native = JSON.parse(nativeBytes.toString('utf8'))
writeFileSync(resolve(directory,'native-artifact.json'),nativeBytes)
const restored = run('native-frozen-artifact-restore', {op:'cad_full_round_exact_restore',artifact:native}, true)
assert.deepEqual(restored.geometry, native.geometry)
const nativeDisplay = run('native-rational-artifact-display', {op:'cad_full_round_exact_display',artifact:native,segments:8}, true)
checkMesh(nativeDisplay, native)
for (const [name, segments, maxTriangles] of [['zero-segments',0,20000],['too-many-segments',33,20000],['triangle-budget',8,2]] as const) {
  run(name,{op:'cad_full_round_exact_display',artifact,segments,maxTriangles},false)
}
const forged = structuredClone(artifact)
forged.geometry.vertices[0].point[0] = ['999','1']
run('forged-exact-geometry',{op:'cad_full_round_exact_display',artifact:forged,segments:8},false)
run('changed-source-text',{...graph,sourceText:'// changed\n'},false)
const badClosure = structuredClone(graph)
badClosure.capabilityClosure = badClosure.capabilityClosure.filter(cap => cap !== 'operation.full-round-exact')
;(badClosure.envelope.core as any).capabilityClosure = badClosure.capabilityClosure
run('missing-operation-capability',badClosure,false)
const report = {schema:'full-round-exact-display-wasm/1',passed:true,cases,volumeErrors,
  wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,
  sourceSha256:program.source.sha256,nativeArtifactSha256:createHash('sha256').update(nativeBytes).digest('hex'),
  scope:'Actual ABI, source-attested graph, display, native rational restore and refusals. No application scene, packaged kernel, worker or exact mesh error/mass certification.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
writeFileSync(resolve(directory,'wasm-box-artifact.json'),JSON.stringify(artifact)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,wasmSha256:report.wasmSha256}))
