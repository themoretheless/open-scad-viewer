import {expect,it,vi} from 'vitest'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidJob,MainSolidRequest} from '../src/services/mainSolidProtocol'
class Port implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 terminate=vi.fn();postMessage=vi.fn<(message:MainSolidRequest)=>void>()
 reply(result:unknown){const {version,id,job}=this.postMessage.mock.lastCall![0];this.onmessage?.({data:{version,id,kind:job.kind,ok:true,result}} as MessageEvent)}
}
const surface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
const job:Extract<MainSolidJob,{kind:'g2RunoutSurface'}>={kind:'g2RunoutSurface',source:surface,plane:surface,
 targetPcurve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,.1],[1,.1]],weights:[1,1]},
 options:{sourceBoundary:'vMax',sourceTrace:{at:.9,along:[0,1]},gains:[.1,.1],transverseUv:[0,1]}}
function result(){return {patches:[surface],report:{method:'native-original-support-G2-runout/1',complete:true,withinTolerance:true,cells:12,exactWork:100,
 maxCells:100000,maxExactWork:1000000,tolerances:[1e-7,1e-5,1e-4],solidGeometryCertified:false,radiusLawCertified:false,interiorEmbeddingCertified:false,
 supportSeams:[0,1].map(source=>({patch:0,source,positionValidated:true,positionUpper:1e-8,inspection:{decision:'within-tolerance',regularityProven:true,cells:6,
 positionBounds:[0,1e-8],orientation:'same',branches:[]}})),internalSeams:[]}}}
// Real client lifecycle with a controlled port; this is not a WASM/UI proof.
it('cancels runout work, retries in a new realm and discards a captured late result',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p})
 try{
  const first=client.run(job),rejected=expect(first).rejects.toMatchObject({name:'AbortError'}),late=ports[0].onmessage!
  client.cancel();await rejected;expect(ports[0].terminate).toHaveBeenCalledOnce()
  const next=client.run(job),response=result()
  late({data:{version:1,id:1,kind:job.kind,ok:true,result:response}} as MessageEvent)
  ports[1].reply(response);await expect(next).resolves.toEqual(response)
 }finally{client.dispose()}
})
it('refuses a malformed surface and forged seam scope from the worker',async()=>{
 for(const mutate of [(v:any)=>v.patches[0].weights[0][0]=NaN,(v:any)=>v.report.supportSeams[1].source=0]){
  const port=new Port(),client=new MainSolidWorkerClient(()=>port)
  try{const promise=client.run(job),failed=expect(promise).rejects.toMatchObject({code:'CAD_PROTOCOL'}),response=structuredClone(result());mutate(response);port.reply(response);await failed}
  finally{client.dispose()}
 }
})
