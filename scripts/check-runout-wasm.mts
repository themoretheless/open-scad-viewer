import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec.ts'
import {decodePacked,writeLinear} from '../src/services/wasmHost.ts'

const path=process.argv[2]
if(!path)throw Error('Provide the staged geometry WASM path')
const bytes=readFileSync(path)
const {instance}=await WebAssembly.instantiate(bytes,{})
const wasm=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
function request(value:unknown):any {
 const bytes=encodeBinary(value),ptr=writeLinear(wasm.memory,wasm.abi_alloc,bytes)
 try{return decodePacked(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,bytes.length))}
 finally{wasm.abi_free(ptr,bytes.length)}
}
const floor={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
const plane={...floor,controlPoints:floor.controlPoints.map(row=>row.map(p=>[p[0],1,p[1]]))}
const value={op:'brep_g2_runout_surface',source:floor,plane,sourceBoundary:'vMax',sourceTrace:{at:.9,along:[0,1]},
 targetPcurve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,.1],[1,.1]],weights:[1,1]},transverseUv:[0,1],gains:[.1,.1],reverseU:[false,false],
 maxPatches:256,maxCells:100000,maxExactWork:1000000,tolerances:[1e-7,1e-5,1e-4]}
const start=performance.now(),result=request(value),elapsedMs=performance.now()-start
assert.equal(result.ok,true,JSON.stringify(result))
const r=result.value.report
assert.equal(r.method,'native-original-support-G2-runout/1')
assert.equal(r.withinTolerance,true);assert.equal(r.complete,true)
assert.equal(r.solidGeometryCertified,false);assert.equal(r.radiusLawCertified,false)
assert.equal(r.supportSeams.length,2)
assert(r.supportSeams.every((s:any)=>s.positionValidated&&s.inspection.decision==='within-tolerance'))
assert(r.cells<=100000&&r.exactWork<=1000000)
const exhausted=request({...value,maxCells:0,maxExactWork:0})
assert.equal(exhausted.ok,true);assert.equal(exhausted.value.report.complete,false);assert.equal(exhausted.value.report.withinTolerance,false)
assert.equal(request({...value,maxExactWork:1000001}).ok,false)
const report={sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length,elapsedMs,
 nativeSupportSeamsG2:true,zeroBudgetRemainsUnresolved:true,excessBudgetRefused:true,closedBodyClaim:false,
 workerUiQualification:false,report:r}
if(process.argv[4]){
 const dir=process.argv[4],supports=JSON.parse(readFileSync(`${dir}/supports.json`,'utf8'))
 const targetPcurve=JSON.parse(readFileSync(`${dir}/target-pcurve.json`,'utf8'))
 const manifest=JSON.parse(readFileSync(`${dir}/manifest.json`,'utf8'))
 const started=performance.now()
 const curved=request({...value,source:supports[0],plane:supports[1],targetPcurve,sourceBoundary:'vMin',
  sourceTrace:{at:manifest.sourceTraceAt,along:manifest.sourceTraceAlong},transverseUv:manifest.planeTransverseUv,gains:[.01,.1]})
 assert.equal(curved.ok,true,JSON.stringify(curved))
 assert.equal(curved.value.report.withinTolerance,true)
 assert.equal(curved.value.report.complete,true)
 assert.equal(curved.value.report.solidGeometryCertified,false)
 assert(curved.value.report.supportSeams.every((s:any)=>s.positionValidated&&s.inspection.decision==='within-tolerance'))
 Object.assign(report,{curved:{elapsedMs:performance.now()-started,report:curved.value.report}})
}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report,null,2))
