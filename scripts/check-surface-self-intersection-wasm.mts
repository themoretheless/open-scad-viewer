import assert from 'node:assert/strict'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'
import {isSurfaceAuditReport} from '../src/services/surfaceSelfIntersectionProtocol.ts'
import {nonlinearSurfaceCases} from '../tools/browser-qualification/surface-self-fixtures.ts'
import { encodeBinary, decodeBinary } from '../src/services/valueBinaryCodec.ts'
const [wasmPath, directory, sourceCommit] = process.argv.slice(2)
assert(wasmPath && directory && /^[a-f0-9]{40}$/.test(sourceCommit ?? ''))
mkdirSync(directory, { recursive: true })
const bytes = readFileSync(wasmPath)
const module = await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module), [])
const instance = await WebAssembly.instantiate(module, {})
const w = instance.exports as unknown as { memory: WebAssembly.Memory; abi_alloc(n:number):number; abi_free(p:number,n:number):void; abi_request(op:number,p:number,n:number):bigint }
const cases: object[] = []
function request(v:unknown):any {
  const data = encodeBinary(v), original = data.slice(), ptr = w.abi_alloc(data.length)
  assert(ptr)
  new Uint8Array(w.memory.buffer, ptr, data.length).set(data)
  try {
    const packed = w.abi_request(0, ptr, data.length), p = Number(packed & 0xffffffffn), n = Number(packed >> 32n)
    try {
      const result = decodeBinary(new Uint8Array(w.memory.buffer, p, n))
      assert.deepEqual(new Uint8Array(w.memory.buffer, ptr, data.length), original)
      return result
    } finally { w.abi_free(p, n) }
  } finally { w.abi_free(ptr, data.length) }
}
const op = 'nurbs_surface_self_intersection_audit'
function run(name:string, surface:object, status:string, extra:object = {}):any {
  const v = { op, surface, maxSpans:100, ...extra }, before = JSON.stringify(v), start = performance.now()
  const reply = request(v)
  assert.equal(JSON.stringify(v), before)
  assert.equal(reply.ok, true, `${name}: ${JSON.stringify(reply.error)}`)
  const r = reply.value
  assert.equal(r.schema, 'nurbs-surface-self-intersection/1')
  assert.equal(r.status, status)
  assert.equal(r.presenceProven, status === 'present')
  assert.equal(r.absenceProven, status === 'absent')
  assert.equal(r.allContactBranchesEnumerated, false)
  assert(isSurfaceAuditReport(r))
  if (status === 'present') assert(r.exactWitness||r.certifiedCrossing)
  else { assert.equal(r.exactWitness, null); assert.equal(r.witness, null) }
  cases.push({name, status, elapsedMs:performance.now()-start,certificate:r.certifiedCrossing,inputIntact:true})
  return r
}
function reject(name:string, v:object):void {
  const start=performance.now(),reply=request(v)
  assert.equal(reply.ok,false,`${name}: ${JSON.stringify(reply)}`)
  assert.equal(typeof reply.error.code,'string')
  cases.push({name,error:reply.error.code,elapsedMs:performance.now()-start})
}
function graph():any { return { degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],controlPoints:[0,.5,1].map(x=>[[x,0,x*x],[x,1,x*x]]),weights:[[1,1],[1,1],[1,1]],periodicU:false,periodicV:false } }
// Surface codec uses the canonical camelCase wire fields.
run('global-absence',graph(),'absent')
const fold=graph();fold.controlPoints[2]=structuredClone(fold.controlPoints[0])
const f=run('exact-global-fold',fold,'present');assert.deepEqual(f.witness,[[.25,.5],[.75,.5]])
const local=graph();local.knotsU=[0,0,0,1,1,3,3,3]
local.controlPoints=[local.controlPoints[0],local.controlPoints[1],local.controlPoints[0],local.controlPoints[2],local.controlPoints[2]];local.weights=Array.from({length:5},()=>[1,1])
const r=run('non-dyadic-witness',local,'present');assert.equal(r.witness,null)
assert.deepEqual(r.exactWitness[0][0],{numerator:'1',denominator:'12'})
const large=structuredClone(local);large.knotsU=[0,0,0,.1,.1,3,3,3]
assert.deepEqual(run('large-exact-integer-strings',large,'present').exactWitness[0][0],{numerator:'3602879701896397',denominator:'432345564227567616'})
const projective=graph();projective.degreeU=3;projective.knotsU=[0,0,0,0,1,1,1,1]
projective.controlPoints=[0,1.5,4,0].map(x=>[[x,0,2*x],[x,1,2*x]]);projective.weights=[1,2,3,4].map(x=>[x,x])
run('asymmetric-projective-gauge',projective,'present')
const crossing=graph();crossing.degreeU=1;crossing.knotsU=[0,0,1,2,3,3]
crossing.controlPoints=[[0,0],[1,1],[0,1],[1,0]].map(([x,y])=>[[x,y,0],[x,y,2]]);crossing.weights=[1,3,2,1].map(x=>[x,x])
const c=run('rational-spatial-crossing',crossing,'present');assert.deepEqual(c.exactWitness[1][0],{numerator:'8',denominator:'9'})
const overlap=structuredClone(crossing);overlap.knotsU=[0,0,1,2,2];overlap.controlPoints=[[0,0],[3,0],[1,0]].map(([x,y])=>[[x,y,0],[x,y,2]]);overlap.weights=Array.from({length:3},()=>[1,1])
run('adjacent-backtracking-overlap',overlap,'present',{maxSpans:1})
const short=graph();short.degreeU=1;short.knotsU=[0,0,.5,1,1]
run('budget-unresolved',short,'unresolved',{maxSpans:1})
reject('forged-verdict',{op,surface:graph(),maxSpans:100,presenceProven:true})
reject('zero-budget',{op,surface:graph(),maxSpans:0})
const invalid=graph();invalid.weights[0][0]=-1
reject('invalid-source',{op,surface:invalid,maxSpans:100})
run('recovery-after-refusal',graph(),'absent')
for(const fixture of nonlinearSurfaceCases){
 const r=run(fixture.name,fixture.surface(),'present',{maxSpans:10000})
 assert.equal(r.witness,null);assert.equal(r.exactWitness,null)
 const c=r.certifiedCrossing;assert.equal(c?.method,'original-planar-krawczyk');assert(c.cells>0&&c.cells<=10000)
 const axis=fixture.axis??0
 for(let i=0;i<2;i++){assert(c.parameterBoxes[i][axis][0]<=fixture.parameters[i]!&&fixture.parameters[i]!<=c.parameterBoxes[i][axis][1]);assert.deepEqual(c.parameterBoxes[i][1-axis],[.5,.5])}
 for(let i=0;i<3;i++)assert(c.positionBounds[i][0]<=fixture.position[i]!&&fixture.position[i]!<=c.positionBounds[i][1])
}
const skew=nonlinearSurfaceCases[2]!.surface()
const bits=new DataView(new ArrayBuffer(8));for(const p of skew.controlPoints[1]!){bits.setFloat64(0,p[2]!,true);bits.setBigUint64(0,bits.getBigUint64(0,true)+1n,true);p[2]=bits.getFloat64(0,true)}
run('one-ulp-nonplanar-source-unresolved',skew,'unresolved',{maxSpans:10000})
const result={status:'passed',nodeVersion:process.version,sourceCommit,wasmPath:resolve(wasmPath),wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,imports:[],cases,installedPackageUpdated:false,scope:'Actual import-free raw WASM binary ABI; not packaged-WASM, Worker, editor or full parity qualification'}
writeFileSync(resolve(directory,'results.json'),JSON.stringify(result,null,2)+'\n')
console.log(JSON.stringify(result,null,2))
