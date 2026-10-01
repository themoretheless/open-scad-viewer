import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {selfIntersectionExpectation,validSelfIntersection} from '../src/services/solidSelfIntersection'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/surface-contact-native/face-contact-api-fixtures.json',import.meta.url),'utf8')).cases[0]
const {model,toleranceUv,op,...limits}=fixture.request
const expectation=selfIntersectionExpectation(model,toleranceUv,limits,6)
const report={...fixture.result,scope:'within-face-and-distinct-face-pairs',maxSpans:6,spans:6,allFacesInjective:true,absenceProven:true,faces:model.faces.map((_:unknown,face:number)=>({face,result:{proven:true,projection:[0,1],contractionUpper:0.01,spans:1,reason:'global-projection-contraction'}}))}
it('validates aggregate proof only when every face and pair is covered',()=>{
 expect(validSelfIntersection(expectation,report)).toBe(true)
 for(const patch of [{absenceProven:false},{allFacesInjective:false},{spans:5},{maxSpans:7},{faces:report.faces.slice(1)},{solidGeometryStatus:'certified'}])expect(validSelfIntersection(expectation,{...report,...patch})).toBe(false)
 for(const patch of [{contractionUpper:1},{contractionUpper:NaN},{projection:[0,0]},{spans:0},{reason:'work-limit'},{proven:false}]){
  const value=structuredClone(report);Object.assign(value.faces[0].result,patch)
  expect(validSelfIntersection(expectation,value)).toBe(false)
 }
})
it('keeps unvisited faces explicit and rejects a proof over an exhausted budget',()=>{
 const partial={...report,maxSpans:1,spans:1,allFacesInjective:false,absenceProven:false,faces:report.faces.map((f:any,i:number)=>i?{face:i,result:null}:f)}
 const e={...expectation,maxSpans:1}
 expect(validSelfIntersection(e,partial)).toBe(true)
 expect(validSelfIntersection(e,{...partial,absenceProven:true})).toBe(false)
 expect(validSelfIntersection(e,{...partial,faces:report.faces})).toBe(false)
})

it('terminates obsolete work and validates the current worker reply',async()=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const job={kind:'selfIntersection' as const,model,toleranceUv,limits,maxSpans:6}
 try{
  const first=client.run(job),cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run(job);await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:job.kind,ok:true,result:report}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:job.kind,ok:true,result:report}})
  await expect(second).resolves.toMatchObject({absenceProven:true,solidGeometryStatus:'not-certified'})
  const forged=client.run(job),rejected=expect(forged).rejects.toMatchObject({code:'CAD_PROTOCOL'})
  const port=ports.at(-1)
  port.onmessage({data:{version:1,id:port.request.id,kind:job.kind,ok:true,result:{...report,faces:[]}}})
  await rejected
 }finally{client.dispose()}
})

it('validates actual WASM reports for full coverage and each exhausted budget',async()=>{
 const {inspectSelfIntersection}=await import('../src/services/solidSelfIntersection')
 const before=JSON.stringify(model)
 for(const [maxSpans,maxPairs] of [[6,100],[1,100],[6,1]]){
  const l={...limits,maxPairs},r=inspectSelfIntersection(model,toleranceUv,l,maxSpans)
  expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,l,maxSpans),r)).toBe(true)
  expect(r.absenceProven).toBe(maxSpans===6&&maxPairs===100)
  expect(r.solidGeometryStatus).toBe('not-certified')
 }
 expect(JSON.stringify(model)).toBe(before)
})

it('runs real combined diagnostics through the worker after a rejected span budget',async()=>{
 const {createMainSolidWorkerHandler}=await import('../src/services/mainSolidWorkerRuntime')
 const messages:any[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 const job={kind:'selfIntersection' as const,model,toleranceUv,limits,maxSpans:6}
 await handle({version:1,id:1,job:{...job,maxSpans:0}})
 await handle({version:1,id:2,job})
 expect(messages[0]).toMatchObject({id:1,ok:false})
 expect(messages[1]).toMatchObject({id:2,kind:'selfIntersection',ok:true,result:{absenceProven:true}})
 expect(validSelfIntersection(expectation,messages[1].result)).toBe(true)
})

it('checks the linear basis and actual subdivision budget in face proofs',()=>{
 const e=selfIntersectionExpectation(model,toleranceUv,limits,22),r=structuredClone(report)
 r.maxSpans=22;r.spans=22
 Object.assign(r.faces[0].result,{projection:null,linearProjection:[[1,1,0],[0,0,1]],spans:17,reason:'global-linear-projection-contraction'})
 expect(validSelfIntersection(e,r)).toBe(true)
 for(const patch of [{linearProjection:[[1,1,0],[0,0,2]]},{linearProjection:[[1,NaN,0],[0,0,1]]},{linearProjection:null},{spans:16},{spans:23},{projection:[0,1]},{contractionUpper:1}]){
  const copy=structuredClone(r);Object.assign(copy.faces[0].result,patch);expect(validSelfIntersection(e,copy)).toBe(false)
 }
})

it('accepts actual native cylinder linear-projection reports without certifying all pairs',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/curved-volume-2026-10-01/linear-face-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),fixture.result)).toBe(true)
 expect(fixture.result.allFacesInjective).toBe(true)
 expect(fixture.result.absenceProven).toBe(false)
})

it('qualifies actual WASM linear face proofs while retaining unresolved cylinder pairs',async()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/curved-volume-2026-10-01/linear-face-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 const {inspectSelfIntersection}=await import('../src/services/solidSelfIntersection')
 const before=JSON.stringify(model),r=inspectSelfIntersection(model,toleranceUv,limits,maxSpans)
 expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),r)).toBe(true)
 expect(r.allFacesInjective).toBe(true);expect(r.spans).toBe(102)
 expect(r.faces).toEqual(fixture.result.faces)
 expect(r.absenceProven).toBe(false);expect(JSON.stringify(model)).toBe(before)
})
