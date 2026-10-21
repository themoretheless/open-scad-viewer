import {expect,it,vi} from 'vitest'
import {existsSync} from 'node:fs'
// This suite deliberately uses the qualified stage, without promoting the viewer package.
vi.mock('../src/generated/geometry-kernels/bytes',async()=>{
 const {readFileSync,existsSync}=await import('node:fs')
 if(!existsSync('output/qualification/runout-wasm-2026-10-16/src/generated/geometry-kernels/bytes.ts'))return vi.importActual('../src/generated/geometry-kernels/bytes')
 const text=readFileSync('output/qualification/runout-wasm-2026-10-16/src/generated/geometry-kernels/bytes.ts','utf8')
 const prefix="export default '",start=text.indexOf(prefix)+prefix.length,end=text.lastIndexOf("'")
 if(start<prefix.length||end<=start)throw Error('Invalid staged package')
 return {default:text.slice(start,end)}
})
vi.mock('../src/generated/geometry-kernels/identity',async()=>{
 const {readFileSync,existsSync}=await import('node:fs')
 if(!existsSync('output/qualification/runout-wasm-2026-10-16/runout-abi.json'))return vi.importActual('../src/generated/geometry-kernels/identity')
 const {sha256,byteLength}=JSON.parse(readFileSync('output/qualification/runout-wasm-2026-10-16/runout-abi.json','utf8'))
 return {default:{sha256,byteLength}}
})
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
import {mainSolidExpectation,mainSolidResult,type MainSolidJob,type MainSolidResponse} from '../src/services/mainSolidProtocol'
it.skipIf(!existsSync('output/qualification/runout-wasm-2026-10-16/runout-abi.json'))('runs the real runout worker handler against staged WASM and recovers after refusal',async()=>{
 const source={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
 const plane={...source,controlPoints:source.controlPoints.map(row=>row.map(p=>[p[0],1,p[1]]))}
 const job:Extract<MainSolidJob,{kind:'g2RunoutSurface'}>={kind:'g2RunoutSurface',source,plane,
  targetPcurve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,.1],[1,.1]],weights:[1,1]},
  options:{sourceBoundary:'vMax',sourceTrace:{at:.9,along:[0,1]},gains:[.1,.1],transverseUv:[0,1]}}
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 await handle({version:1,id:1,job})
 const first=messages[0];expect(first).toMatchObject({ok:true,kind:job.kind,result:{report:{complete:true,withinTolerance:true,solidGeometryCertified:false}}})
 if(!first.ok)throw Error('WASM runout failed')
 expect(mainSolidResult(mainSolidExpectation(job),first.result)).toBe(true)
 await handle({version:1,id:2,job:{...job,options:{...job.options,transverseUv:[0,0]}}})
 expect(messages[1]).toMatchObject({ok:false,kind:job.kind})
 await handle({version:1,id:3,job})
 expect(messages[2]).toMatchObject({ok:true,result:{report:{withinTolerance:true}}})
 const zero={...job,options:{...job.options,maxCells:0,maxExactWork:0}}
 await handle({version:1,id:4,job:zero})
 expect(messages[3]).toMatchObject({ok:true,result:{report:{complete:false,withinTolerance:false}}})
},30000)
