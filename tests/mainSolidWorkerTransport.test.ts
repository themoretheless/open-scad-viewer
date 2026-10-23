import {expect,it} from 'vitest'
import {prepareMainSolidTransfer} from '../src/services/mainSolidWorkerTransport'
import type {MainSolidResponse} from '../src/services/mainSolidProtocol'
import type {DirectDocument} from '../src/services/directModeling'
it.each(['restoreDocument','sceneEdit'] as const)('moves %s mesh buffers exactly once while preserving views, UVs and aliases',(kind)=>{
 const positions=new Float64Array([0,0,0,1,0,0,0,1,0]),indices=new Uint32Array([0,1,2]),uv=new Float64Array([0,0,1,0,0,1])
 const document:DirectDocument={version:1,sketches:[],bodies:[{id:'a',name:'A',mesh:{positions,indices,uv}},{id:'b',name:'B',mesh:{positions:positions.subarray(0),indices}}]}
 const response:MainSolidResponse={version:1,id:1,kind,ok:true,result:document}
 const prepared=prepareMainSolidTransfer(response)
 expect(prepared.transfer).toHaveLength(3)
 const received=structuredClone(prepared.response,{transfer:prepared.transfer})
 expect(positions.byteLength).toBe(0);expect(indices.byteLength).toBe(0);expect(uv.byteLength).toBe(0)
 if(!received.ok)throw Error('Unexpected failure')
 const bodies=(received.result as DirectDocument).bodies
 expect(Array.from(bodies[0].mesh.positions)).toEqual([0,0,0,1,0,0,0,1,0])
 expect(Array.from(bodies[0].mesh.indices)).toEqual([0,1,2]);expect(Array.from(bodies[0].mesh.uv!)).toEqual([0,0,1,0,0,1])
 expect(bodies[1].mesh.positions.buffer).toBe(bodies[0].mesh.positions.buffer)
 expect(bodies[1].mesh.indices.buffer).toBe(bodies[0].mesh.indices.buffer)
})
it('does not transfer unrelated scene results or failed responses',()=>{
 const document:DirectDocument={version:1,sketches:[],bodies:[{id:'a',name:'A',mesh:{positions:new Float64Array([0,0,0]),indices:new Uint32Array()}}]}
 const response:MainSolidResponse={version:1,id:1,kind:'pointEdit',ok:true,result:document}
 expect(prepareMainSolidTransfer(response).transfer).toEqual([])
 expect(document.bodies[0].mesh.positions.byteLength).toBe(24)
 expect(prepareMainSolidTransfer({version:1,id:2,kind:'restoreDocument',ok:false,error:{name:'Error',message:'invalid'}}).transfer).toEqual([])
})

it('rejects out-of-range or malformed profile provenance without throwing',async()=>{
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const expected={kind:'profilePrepare' as const,chainSegments:[2,1]}
 const source={chain:0,segment:1,reversed:true,connector:false}
 const result={document:{version:1,sketches:[],bodies:[]},id:'a',plane:{origin:[0,0,0],u:[1,0,0],v:[0,1,0]},report:{accepted:false,reason:'invalid-contour',points:[],defects:[],connectors:[],curveSources:[source]}}
 expect(mainSolidResult(expected,result)).toBe(true)
 for(const invalid of [{...source,chain:2},{...source,segment:2},{...source,reversed:'yes'},{chain:0,end:'middle',nextChain:1,reversed:false,connector:true},{chain:0,end:'end',nextChain:2,reversed:false,connector:true}]){
  expect(mainSolidResult(expected,{...result,report:{...result.report,curveSources:[invalid]}})).toBe(false)
 }
 expect(mainSolidResult(expected,{...result,report:{...result.report,profile:{loops:null}}})).toBe(false)
})
