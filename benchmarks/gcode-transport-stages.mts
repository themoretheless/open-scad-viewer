import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {warmGeometryKernel, kernelRuntime, decodeNurbsResult} from '../src/services/geometry/kernel'
import {parseGcodePreview, type GcodePreviewResult} from '../src/services/geometry/polygon'
import {encodeBinary, decodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear, packedSize} from '../src/services/wasmHost'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {compileWasmArtifact} from '../src/services/wasmArtifact'
import {unpackGcodePreview,type PackedGcodePreview} from '../src/services/gcodePreviewTransport'

const artifact=readFileSync('public/wasm/geometry-kernel.wasm')
let loaded=false
setOptionalWasmCompiler(async(url,identity)=>{
  assert.equal(url,'/wasm/geometry-kernel.wasm')
  assert.ok(identity)
  loaded=true
  return compileWasmArtifact(artifact,identity)
})
await warmGeometryKernel()
assert.ok(loaded)
const runtime = kernelRuntime(), wasm = runtime.exports
const warmups = 20, samples = 31
const quantiles = (values: number[]) => {
  const ordered = [...values].sort((a,b)=>a-b)
  return {p50Ms:ordered[15],p95Ms:ordered[29],samplesMs:values}
}
type FlatPreview = Omit<GcodePreviewResult,'moves'> & {moveRows:number[]}
function flatten({moves,...metadata}:GcodePreviewResult):FlatPreview {
  return {...metadata,moveRows:moves.flatMap(m=>[m.x,m.y,m.z,m.e,m.feedrateMmS,m.layerIndex,Number(m.extruded)])}
}
function expand({moveRows,...metadata}:FlatPreview):GcodePreviewResult {
  assert.equal(moveRows.length%7,0)
  const moves:GcodePreviewResult['moves']=new Array(moveRows.length/7)
  for(let i=0,j=0;i<moveRows.length;i+=7,j++)moves[j]={x:moveRows[i],y:moveRows[i+1],z:moveRows[i+2],e:moveRows[i+3],feedrateMmS:moveRows[i+4],layerIndex:moveRows[i+5],extruded:moveRows[i+6]===1}
  return {...metadata,moves}
}
function invoke(gcode:string,packedMoves=false) {
  const start=performance.now(), bytes=encodeBinary({op:'gcode_preview',gcode,packedMoves}), encoded=performance.now()
  const pointer=writeLinear(runtime.memory,n=>wasm.abi_alloc(n),bytes), copied=performance.now()
  assert.ok(pointer)
  try {
    const packed=wasm.abi_request(0,pointer,bytes.length), invoked=performance.now()
    const wire=decodeNurbsResult<GcodePreviewResult|PackedGcodePreview>(runtime.takeResponse(packed))
    const result=packedMoves?unpackGcodePreview(wire as PackedGcodePreview):wire as GcodePreviewResult, decoded=performance.now()
    return {result,responseBytes:packedSize(packed),requestBytes:bytes.length,
      encodeMs:encoded-start,copyMs:copied-encoded,rustAndSerializeMs:invoked-copied,
      decodeAndFreeMs:decoded-invoked,totalMs:decoded-start}
  } finally {wasm.abi_free(pointer,bytes.length)}
}

const results=[]
for(const moves of [100,10_000,80_000]) {
  let gcode='G90\nM83\nG1 X0 Y0 Z0.2 F600\n'
  for(let i=0;i<moves;i++)gcode+=`G1 X${(i+1)%2} Y0 E0.01 F600\n`
  const expected=parseGcodePreview(gcode)
  assert.equal(expected.moves.length,moves+1)
  assert.equal(expected.printDistanceMm,moves)
  assert.ok(Math.abs(expected.extrusionMm-moves*.01)<1e-7)
  const first=invoke(gcode)
  const packedFirst=invoke(gcode,true)
  assert.deepEqual(first.result,expected)
  assert.deepEqual(packedFirst.result,expected)
  for(let i=0;i<warmups;i++){invoke(gcode);invoke(gcode,true)}
  const sample=(packed:boolean)=>{
    const row=invoke(gcode,packed)
    assert.equal(row.result.moves.length,moves+1)
    assert.equal(row.result.estimatedTimeS,expected.estimatedTimeS)
    assert.deepEqual(row.result.moves.at(-1),expected.moves.at(-1))
    const {result:_,...timings}=row
    return timings
  }
  const stages:ReturnType<typeof sample>[]=[],packedStages:ReturnType<typeof sample>[]=[]
  for(let i=0;i<samples;i++)for(const packed of i%2?[true,false]:[false,true]){
    ;(packed?packedStages:stages).push(sample(packed))
  }
  const summarize=(rows:typeof stages)=>Object.fromEntries(['encodeMs','copyMs','rustAndSerializeMs','decodeAndFreeMs','totalMs'].map(key=>[key,quantiles(rows.map(row=>row[key as keyof typeof row] as number))]))
  // Encoding and flattening are intentionally outside decoder timing. This
  // prototype does not measure a Rust packed serializer or worker transfer.
  const objectBytes=encodeBinary(expected),flatBytes=encodeBinary(flatten(expected))
  const decodeObject=()=>decodeBinary(objectBytes) as GcodePreviewResult
  const decodeFlat=()=>expand(decodeBinary(flatBytes) as FlatPreview)
  assert.deepEqual(decodeObject(),expected)
  assert.deepEqual(decodeFlat(),expected)
  for(let i=0;i<warmups;i++){decodeObject();decodeFlat()}
  const objectMs:number[]=[],flatMs:number[]=[]
  for(let i=0;i<samples;i++) {
    for(const flat of i%2?[true,false]:[false,true]) {
      const start=performance.now(),result=flat?decodeFlat():decodeObject(),elapsed=performance.now()-start
      assert.equal(result.moves.length,moves+1)
      ;(flat?flatMs:objectMs).push(elapsed)
    }
  }
  results.push({moves,inputSha256:createHash('sha256').update(gcode).digest('hex'),
    resultSha256:createHash('sha256').update(JSON.stringify(expected)).digest('hex'),
    requestBytes:first.requestBytes,responseBytes:first.responseBytes,
    stages:summarize(stages),packedStages:summarize(packedStages),packedResponseBytes:packedFirst.responseBytes,
    codecPrototype:{objectBytes:objectBytes.length,flatBytes:flatBytes.length,
      objectDecode:quantiles(objectMs),flatDecodeAndObjectRebuild:quantiles(flatMs)}})
}
console.log(JSON.stringify({node:process.version,platform:process.platform,arch:process.arch,
  artifactSha256:createHash('sha256').update(artifact).digest('hex'),warmups,samples,
  scope:'Alternating warm legacy/packed WASM calls including object rebuild, plus separate JS codec prototype; no worker transfer, request-free timing, allocation profile or UI speedup claim',results},null,2))
