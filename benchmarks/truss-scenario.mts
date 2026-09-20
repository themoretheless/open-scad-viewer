import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {resolveTrussScenario, type TrussScenario} from '../src/services/trussScenario'
import {solveTruss, type TrussVector} from '../src/services/trussAnalysis'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {compileWasmArtifact} from '../src/services/wasmArtifact'

const artifact=readFileSync('public/wasm/geometry-kernel.wasm')
setOptionalWasmCompiler(async (_,identity)=>{assert.ok(identity);return compileWasmArtifact(artifact,identity)})
await warmGeometryKernel()
function fixture(freeNodes:number,caseCount:number):TrussScenario {
  const nodesMm:TrussVector[]=[[10,0,0],[0,10,0],[0,0,0]],members:TrussScenario['members']=[]
  const restrained:[boolean,boolean,boolean][]=[[true,true,true],[true,true,true],[true,true,true]]
  const forceN:TrussVector=[0,0,0],momentNmm:TrussVector=[0,0,0]
  for(let i=0;i<freeNodes;i++) {
    const p:TrussVector=[1,2,10+i/10],f:TrussVector=[1,-2,-3]
    nodesMm.push(p);restrained.push([false,false,false])
    for(let anchor=0;anchor<3;anchor++)members.push({nodes:[anchor,i+3],youngMpa:2000,areaMm2:2})
    for(let k=0;k<3;k++)forceN[k]+=f[k]
    momentNmm[0]+=p[1]*f[2]-p[2]*f[1];momentNmm[1]+=p[2]*f[0]-p[0]*f[2];momentNmm[2]+=p[0]*f[1]-p[1]*f[0]
  }
  return {nodesMm,members,cases:Array.from({length:caseCount},(_,i)=>({id:`case-${i}`,restrained,
    loads:[{nodes:Array.from({length:freeNodes},(_,j)=>j+3),originMm:[0,0,0],forceN,momentNmm}]})),
    combinations:[{id:'combined',terms:Array.from({length:caseCount},(_,i)=>({caseId:`case-${i}`,factor:1/caseCount}))}],activeId:'combined'}
}
const stats=(samplesMs:number[])=>{const sorted=[...samplesMs].sort((a,b)=>a-b);return {p50Ms:sorted[15],p95Ms:sorted[29],samplesMs}}
const results=[]
for(const [freeNodes,caseCount] of [[1,1],[40,4],[122,32]]) {
  const input=fixture(freeNodes,caseCount),resolved=resolveTrussScenario(input),expected=solveTruss(resolved.model)
  assert.equal(expected.freeDofs,freeNodes*3)
  for(let i=0;i<freeNodes;i++) {
    const z=10+i/10,total=-3/z,q0=(total-1)/10,q1=(2*total+2)/10
    const forces=[q0*Math.hypot(-9,2,z),q1*Math.hypot(1,-8,z),(total-q0-q1)*Math.hypot(1,2,z)]
    for(let k=0;k<3;k++)assert.ok(Math.abs(expected.axialForcesN[3*i+k]-forces[k])<1e-8)
  }
  for(let i=0;i<200;i++){solveTruss(resolved.model);solveTruss(resolveTrussScenario(input).model)}
  const direct:number[]=[],scenario:number[]=[],assembly:number[]=[]
  for(let i=0;i<31;i++) {
    for(const compile of i%2?[true,false]:[false,true]) {
      const start=performance.now(),result=solveTruss(compile?resolveTrussScenario(input).model:resolved.model)
      ;(compile?scenario:direct).push(performance.now()-start)
      assert.deepEqual(result,expected)
    }
    const start=performance.now()
    let last=resolved
    for(let j=0;j<100;j++)last=resolveTrussScenario(input)
    assembly.push((performance.now()-start)/100)
    assert.deepEqual(last,resolved)
  }
  results.push({nodes:input.nodesMm.length,members:input.members.length,cases:caseCount,loads:resolved.model.loads.length,
    direct:stats(direct),scenario:stats(scenario),assembly:stats(assembly),
    resultSha256:createHash('sha256').update(JSON.stringify(expected)).digest('hex')})
}
console.log(JSON.stringify({node:process.version,arch:process.arch,platform:process.platform,warmups:200,samples:31,
  artifactSha256:createHash('sha256').update(artifact).digest('hex'),
  scope:'Scenario validation/snapshot plus warm native WASM solve vs pre-resolved identical model; no worker, rendering or speedup claim',results},null,2))
