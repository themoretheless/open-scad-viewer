import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {solveTruss, type TrussModel, type TrussResponse, type TrussWrenchModel, type TrussVector} from '../src/services/trussAnalysis'
import {encodeBinary} from '../src/services/valueBinaryCodec'

// Same supported-tripod workloads as mechanics-core/examples/bench_truss.rs.
function fixture(freeNodes: number): TrussModel {
  const model: TrussModel = {nodesMm:[[10,0,0],[0,10,0],[0,0,0]],
    members:[], restrained:[[true,true,true],[true,true,true],[true,true,true]],
    forcesN:[[0,0,0],[0,0,0],[0,0,0]]}
  for (let i=0; i<freeNodes; i++) {
    model.nodesMm.push([1,2,10+i/10])
    model.restrained.push([false,false,false])
    model.forcesN.push([1,-2,-3])
    for (let anchor=0; anchor<3; anchor++) {
      model.members.push({nodes:[anchor,i+3],youngMpa:2000,areaMm2:2})
    }
  }
  return model
}

function verify(model: TrussModel, result: TrussResponse) {
  assert.equal(result.freeDofs,(model.nodesMm.length-3)*3)
  assert.ok(result.maxRelativeResidual<1e-12)
  const sumForce=[0,0,0], sumMoment=[0,0,0]
  for (let i=0; i<model.nodesMm.length; i++) {
    const p=model.nodesMm[i], f=model.forcesN[i].map((x,k)=>x+result.reactionsN[i][k])
    for (let k=0;k<3;k++) sumForce[k]+=f[k]
    sumMoment[0]+=p[1]*f[2]-p[2]*f[1]
    sumMoment[1]+=p[2]*f[0]-p[0]*f[2]
    sumMoment[2]+=p[0]*f[1]-p[1]*f[0]
  }
  for (const value of [...sumForce,...sumMoment]) assert.ok(Math.abs(value)<1e-8)
  assert.ok(result.displacementsMm.flat().every(Number.isFinite))
  assert.equal(result.axialForcesN.length,model.members.length)
  for(let i=0;i<model.nodesMm.length-3;i++) {
    const z=10+i/10, total=-3/z, q0=(total-1)/10, q1=(2*total+2)/10
    const expected=[q0*Math.hypot(-9,2,z),q1*Math.hypot(1,-8,z),
      (total-q0-q1)*Math.hypot(1,2,z)]
    for(let k=0;k<3;k++) assert.ok(Math.abs(result.axialForcesN[i*3+k]-expected[k])<1e-9)
  }
}

const artifact=readFileSync('public/wasm/geometry-kernel.wasm')
let loaded=false
setOptionalWasmCompiler(async url=>{
  assert.equal(url,'/wasm/geometry-kernel.wasm'); loaded=true
  return WebAssembly.compile(artifact)
})
const coldStart=performance.now()
await warmGeometryKernel()
const coldMs=performance.now()-coldStart
assert.ok(loaded)
const results=[]
const warmups=Number(process.env.TRUSS_WARMUPS??200)
assert.ok(Number.isSafeInteger(warmups)&&warmups>=0&&warmups<=1000,'TRUSS_WARMUPS must be 0-1000')
for (const freeNodes of [1,40,122]) {
  const model=fixture(freeNodes)
  const {forcesN:_,...structure}=model
  const forceN:TrussVector=[0,0,0],momentNmm:TrussVector=[0,0,0]
  for(let i=3;i<model.nodesMm.length;i++) {
    const p=model.nodesMm[i],f=model.forcesN[i]
    for(let k=0;k<3;k++)forceN[k]+=f[k]
    momentNmm[0]+=p[1]*f[2]-p[2]*f[1];momentNmm[1]+=p[2]*f[0]-p[0]*f[2];momentNmm[2]+=p[0]*f[1]-p[1]*f[0]
  }
  const wrench:TrussWrenchModel={...structure,loads:[{nodes:Array.from({length:freeNodes},(_,i)=>i+3),originMm:[0,0,0],forceN,momentNmm}]}
  for(let i=0;i<warmups;i++)for(const input of [model,wrench])verify(model,solveTruss(input))
  const samplesMs:number[]=[],wrenchSamplesMs:number[]=[]
  for(let i=0;i<31;i++) {
    for(const mode of i%2?[1,0]:[0,1]) {
      const input=mode?wrench:model,start=performance.now(),result=solveTruss(input)
      ;(mode?wrenchSamplesMs:samplesMs).push(performance.now()-start)
      verify(model,result)
    }
  }
  const ordered=[...samplesMs].sort((a,b)=>a-b)
  const wrenchOrdered=[...wrenchSamplesMs].sort((a,b)=>a-b)
  results.push({nodes:model.nodesMm.length,members:model.members.length,
    freeDofs:freeNodes*3,p50Ms:ordered[15],p95Ms:ordered[29],samplesMs,
    wrenchP50Ms:wrenchOrdered[15],wrenchP95Ms:wrenchOrdered[29],wrenchSamplesMs,
    inputBytes:{nodal:encodeBinary({op:'truss_solve',...model}).byteLength,
      wrench:encodeBinary({op:'truss_solve_wrenches',...wrench}).byteLength}})
}
console.log(JSON.stringify({node:process.version,arch:process.arch,platform:process.platform,
  execArgv:process.execArgv,
  artifactBytes:artifact.length,artifactSha256:createHash('sha256').update(artifact).digest('hex'),
  scope:'alternating equivalent nodal/wrench models; complete warm WASM call, not UI or TS speedup',
  coldCompileAndInstantiateMs:coldMs,warmups,samples:31,results},null,2))
