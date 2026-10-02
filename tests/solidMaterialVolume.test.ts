import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {materialExpectation,validMaterial} from '../src/services/solidMaterialVolume'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
const cases=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/material-segment/contract.json',import.meta.url),'utf8')).cases
const mode=(c:any)=>c.request.op==='cad_material_chord'?'chord' as const:'segment' as const
it('accepts original native material segments, chords and explicit refusals',()=>{
 for(const c of cases){
  expect(validMaterial(materialExpectation(c.request,mode(c)),c.result),c.name).toBe(true)
  const job={kind:mode(c)==='chord'?'materialChord' as const:'materialSegment' as const,options:c.request}
  expect(mainSolidResult(mainSolidExpectation(job),c.result),c.name).toBe(true)
 }
 expect(cases.map((c:any)=>c.result.proven)).toEqual([true,true,false,false,true,false,false])
})
it('rejects source, budget, volume, seed and endpoint proof mutations',()=>{
 const c=cases[1],e=materialExpectation(c.request,'chord')
 for(const [index,mutate] of [
  (r:any)=>r.sourceModel.vertices[0].point[0]++,
  (r:any)=>r.direction[0]++,
  (r:any)=>r.limits.pointCells++,
  (r:any)=>r.validity.exactAgreement=false,
  (r:any)=>r.validity.orientations[0].outward=null,
  (r:any)=>r.seed.inside=true,
  (r:any)=>r.seed.attempts[0].crossings.pop(),
  (r:any)=>r.boundary.contacts[0].face=999,
  (r:any)=>r.boundary.contacts[0].uv[0]=[-1,0],
  (r:any)=>r.boundary.contacts[0].parameter=[0,0.2],
  (r:any)=>r.boundary.contacts[1].parameter=r.boundary.contacts[0].parameter,
  (r:any)=>r.boundary.contacts.pop(),
  (r:any)=>r.boundary.cells=r.limits.segmentCells+1,
  (r:any)=>r.pointEnclosures=null,
  (r:any)=>r.lengthIntervalMm=[1,2],
  (r:any)=>r.normalAlignment='proven',
  (r:any)=>r.minimumWallThickness='proven',
 ].entries()){const r=structuredClone(c.result);mutate(r);expect(validMaterial(e,r),`mutation ${index}`).toBe(false)}
 const refused=cases[3]
 expect(validMaterial(materialExpectation(refused.request,'chord'),{...refused.result,proven:true})).toBe(false)
 const segment=cases[0],se=materialExpectation(segment.request,'segment')
 expect(validMaterial(se,{...segment.result,segment:null})).toBe(false)
 const r=structuredClone(segment.result);r.segment.boundaryFree=false
 expect(validMaterial(se,r)).toBe(false)
})
it.each([0,1])('cancels obsolete material job %i and rejects stale or forged replies',async(index)=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const c=cases[index],job={kind:index===1?'materialChord' as const:'materialSegment' as const,options:c.request}
 try{
  const first=client.run(job),cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run(job);await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:job.kind,ok:true,result:c.result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:job.kind,ok:true,result:c.result}})
  await expect(second).resolves.toMatchObject({proven:true,reason:c.result.reason})
  const third=client.run(job),rejected=expect(third).rejects.toThrow()
  const forged=structuredClone(c.result);forged.sourceModel.vertices[0].point[0]++
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:job.kind,ok:true,result:forged}})
  await rejected
 }finally{client.dispose()}
})
