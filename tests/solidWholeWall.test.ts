import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {wholeWallExpectation,validWholeWall} from '../src/services/solidWholeWall'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
const cases=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/whole-wall/contract.json',import.meta.url),'utf8')).cases
it('accepts native full wall bounds and explicit refusal or wide intervals',()=>{
 for(const c of cases){
  expect(validWholeWall(wholeWallExpectation(c.request),c.result),c.name).toBe(true)
  expect(mainSolidResult(mainSolidExpectation({kind:'wholeWall',options:c.request}),c.result),c.name).toBe(true)
 }
 expect(cases.map((c:any)=>c.result.converged)).toEqual([true,false,true,false,false,false,false])
 expect(cases[0].result.coverage.totalPairs).toBe(21)
})
it('cancels obsolete bodies, ignores late replies and retries after a forged source',async()=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 try{
  const first=client.run({kind:'wholeWall',options:cases[2].request})
  const cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run({kind:'wholeWall',options:cases[3].request})
  await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:'wholeWall',ok:true,result:cases[2].result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'wholeWall',ok:true,result:cases[3].result}})
  await expect(second).resolves.toMatchObject({converged:false,reason:'whole-wall-bounds'})
  const third=client.run({kind:'wholeWall',options:cases[2].request}),rejected=expect(third).rejects.toThrow()
  const forged=structuredClone(cases[2].result);forged.candidate.sourceModel.vertices[0].point[0]++
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'wholeWall',ok:true,result:forged}})
  await rejected;expect(ports[1].terminated).toBe(true)
  const retry=client.run({kind:'wholeWall',options:cases[2].request}),fresh=ports.at(-1)
  fresh.onmessage({data:{version:1,id:fresh.request.id,kind:'wholeWall',ok:true,result:cases[2].result}})
  await expect(retry).resolves.toMatchObject({converged:true})
 }finally{client.dispose()}
})
it('rejects missing original face pairs and forged coverage, source or convergence',()=>{
 for(const c of cases){
  const e=wholeWallExpectation(c.request)
  for(const mutate of [
   (r:any)=>r.coverage.pairs.shift(),
   (r:any)=>r.coverage.pairs[0].faces[0]++,
   (r:any)=>r.coverage.totalPairs--,
   (r:any)=>r.coverage.enumerationComplete=!r.coverage.enumerationComplete,
   (r:any)=>r.coverage.lowerBoundMm=-1,
   (r:any)=>r.coverage.planeControls=r.coverageLimits.maxPlaneControls+1,
   (r:any)=>r.coverageLimits.maxFacePairs++,
   (r:any)=>r.candidate.sourceModel.vertices[0].point[0]++,
   (r:any)=>r.converged=!r.converged,
  ]){const r=structuredClone(c.result);mutate(r);expect(validWholeWall(e,r),c.name).toBe(false)}
 }
 const r=structuredClone(cases[3].result);r.coverage.pairs.find((p:any)=>p.reason==='self-pair-unresolved').lowerBoundMm=10
 expect(validWholeWall(wholeWallExpectation(cases[3].request),r)).toBe(false)
})
