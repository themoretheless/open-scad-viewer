import {performance} from 'node:perf_hooks'
import {languageRequest,warmLanguageKernel} from '../src/services/languages/kernel.ts'
await warmLanguageKernel()
const count=4096
const calls=Array.from({length:count},(_,i)=>({function:i%2?'cos':'sin',args:[i>>1]}))
function run(size:number){const start=performance.now();for(let i=0;i<count;i+=size)languageRequest(35,{calls:calls.slice(i,i+size)});return performance.now()-start}
run(1);run(1024)
for(const size of [1,2,32,256,1024]){
 const timings=Array.from({length:7},()=>run(size)).sort((a,b)=>a-b)
 console.log(JSON.stringify({batch:size,values:count,medianMs:timings[3],nsPerValue:timings[3]*1e6/count}))
}
const {evaluateDegreeScalar,evaluateDegreeBatch}=await import('../src/services/openScadDegreeMath.ts')
for(const [mode,run] of [
 ['production-pairs',()=>{for(const call of calls)evaluateDegreeScalar(call.function as 'sin'|'cos',call.args)}],
 ['adapter-batch',()=>evaluateDegreeBatch(calls as {function:'sin'|'cos';args:number[]}[])],
] as const){
 run()
 const timings=Array.from({length:7},()=>{const start=performance.now();run();return performance.now()-start}).sort((a,b)=>a-b)
 console.log(JSON.stringify({mode,values:count,medianMs:timings[3],nsPerValue:timings[3]*1e6/count}))
}
for(const mode of ['single-sin-abi','single-sin-adapter']){
 const timings=Array.from({length:7},()=>{const start=performance.now();for(let i=0;i<count;i++){
  if(mode==='single-sin-abi')languageRequest(35,{calls:[{function:'sin',args:[i]}]})
  else evaluateDegreeScalar('sin',[i])
 }return performance.now()-start}).sort((a,b)=>a-b)
 console.log(JSON.stringify({mode,values:count,medianMs:timings[3]}))
}
