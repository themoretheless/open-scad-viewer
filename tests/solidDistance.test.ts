import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {solidDistanceExpectation,validSolidDistance} from '../src/services/solidDistance'
const cases=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/solid-distance-2026-09-30/contract-fixtures.json',import.meta.url),'utf8')).cases
it('accepts native containment, invalid-volume and contact reports',()=>{
 for(const c of cases)expect(validSolidDistance(solidDistanceExpectation(c.request),c.result)).toBe(true)
 expect(cases[0].result.reason).toBe('material-containment')
 expect(cases[1].result.reason).toBe('volume-validity-unproven')
 expect(cases[2].result.reason).toBe('certified-boundary-contact')
})
it('rejects inconsistent proof, budgets, shell ownership and contact coordinates',()=>{
 const {request,result}=cases[0],e=solidDistanceExpectation(request)
 for(const patch of [{scope:'boundary-shells-bounded-joins'},{converged:false},{distanceIntervalMm:[1,2]},{materialOverlap:null},{visitedShellPairs:0},{totalShellPairs:2},{cells:request.maxCells+1},{validity:[]},{reason:'separated-volumes'}])expect(validSolidDistance(e,{...result,...patch})).toBe(false)
 for(const mutate of [
  (r:any)=>r.validity[0].exactAgreement=false,
  (r:any)=>r.validity[0].orientations[0].expectedOutward=false,
  (r:any)=>r.validity[0].orientations[0].outward=null,
  (r:any)=>r.validity[0].nestingRolesConsistent=null,
  (r:any)=>r.limits.validity.exactWork++,
 ]){const r=structuredClone(result);mutate(r);expect(validSolidDistance(e,r)).toBe(false)}
 const contact=cases[2],ce=solidDistanceExpectation(contact.request)
 for(const mutate of [
  (r:any)=>r.contact.faces[0]=999,
  (r:any)=>r.contact.firstUv[0]=[-1,-1],
  (r:any)=>r.contact.contractionUpper=0.5,
  (r:any)=>r.contact.pointIntervalMm[0]=[2,1],
  (r:any)=>r.contactPairsVisited=0,
  (r:any)=>r.contact=null,
 ]){const r=structuredClone(contact.result);mutate(r);expect(validSolidDistance(ce,r)).toBe(false)}
 const invalid=cases[1],ie=solidDistanceExpectation(invalid.request)
 expect(validSolidDistance(ie,{...invalid.result,distanceIntervalMm:[0,0]})).toBe(false)
 expect(validSolidDistance(ie,{...invalid.result,converged:true})).toBe(false)
})
it('terminates an obsolete solid-distance request and rejects incomplete proof in replies',async()=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const job={kind:'solidDistance' as const,options:cases[0].request}
 try{
  const first=client.run(job),cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run(job);await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:job.kind,ok:true,result:cases[0].result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:job.kind,ok:true,result:cases[0].result}})
  await expect(second).resolves.toMatchObject({distanceIntervalMm:[0,0]})
  const forged=client.run(job),rejected=expect(forged).rejects.toMatchObject({code:'CAD_PROTOCOL'}),port=ports.at(-1)
  port.onmessage({data:{version:1,id:port.request.id,kind:job.kind,ok:true,result:{...cases[0].result,validity:[]}}})
  await rejected
 }finally{client.dispose()}
})

it('validates actual WASM volume-distance responses without changing either input',async()=>{
 const {measureSolidDistance}=await import('../src/services/solidDistance')
 for(const c of cases){
  const before=JSON.stringify(c.request),r=measureSolidDistance(c.request)
  expect(validSolidDistance(solidDistanceExpectation(c.request),r)).toBe(true)
  expect(r.reason).toBe(c.result.reason)
  expect(r.distanceIntervalMm).toEqual(c.result.distanceIntervalMm)
  expect(JSON.stringify(c.request)).toBe(before)
 }
})
