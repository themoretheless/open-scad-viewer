import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {materialWallExpectation,validMaterialWall} from '../src/services/solidMaterialWall'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
const cases=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/material-wall/contract.json',import.meta.url),'utf8')).cases
it('accepts native full wall bounds and explicit refusal or wide intervals',()=>{
 for(const c of cases){
  expect(validMaterialWall(materialWallExpectation(c.request),c.result),c.name).toBe(true)
  expect(mainSolidResult(mainSolidExpectation({kind:'materialWall',options:c.request}),c.result),c.name).toBe(true)
 }
 expect(cases.map((c:any)=>c.result.converged)).toEqual([true,false,true,false,false])
 expect(cases[3].result.clearance.totalPairs).toBe(54)
})
it('rejects changed sources, groups, budgets and substituted surface-gap upper bounds',()=>{
 for(const c of cases){
  const e=materialWallExpectation(c.request)
  for(const mutate of [
   (r:any)=>r.faceGroups[0].push(r.faceGroups[1][0]),
   (r:any)=>r.maxDistanceCells++,
   (r:any)=>r.toleranceMm*=2,
   (r:any)=>r.clearance.totalPairs--,
   (r:any)=>r.clearance.evaluatedPairs=r.clearance.totalPairs+1,
   (r:any)=>r.clearance.lowerBoundMm=-1,
   (r:any)=>r.candidate.sourceModel.vertices[0].point[0]++,
   (r:any)=>r.candidate.normalAudit.maxSpans++,
   (r:any)=>r.converged=!r.converged,
  ]){const r=structuredClone(c.result);mutate(r);expect(validMaterialWall(e,r),c.name).toBe(false)}
  const changed=structuredClone(c.request);changed.faceGroups.reverse()
  expect(validMaterialWall(materialWallExpectation(changed),c.result),c.name).toBe(false)
 }
 const broad=cases[3],r=structuredClone(broad.result)
 r.intervalMm=[r.clearance.lowerBoundMm,r.clearance.upperBoundMm];r.converged=true
 expect(validMaterialWall(materialWallExpectation(broad.request),r)).toBe(false)
 const refused=cases[4],f=structuredClone(refused.result)
 f.intervalMm=[f.clearance.lowerBoundMm,f.clearance.upperBoundMm];f.converged=true
 expect(validMaterialWall(materialWallExpectation(refused.request),f)).toBe(false)
})
it('cancels obsolete face groups, ignores late replies and retries after a forged source',async()=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 try{
  const first=client.run({kind:'materialWall',options:cases[2].request})
  const cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run({kind:'materialWall',options:cases[3].request})
  await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:'materialWall',ok:true,result:cases[2].result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'materialWall',ok:true,result:cases[3].result}})
  await expect(second).resolves.toMatchObject({converged:false,reason:'material-thickness-bounds'})
  const third=client.run({kind:'materialWall',options:cases[2].request}),rejected=expect(third).rejects.toThrow()
  const forged=structuredClone(cases[2].result);forged.candidate.sourceModel.vertices[0].point[0]++
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'materialWall',ok:true,result:forged}})
  await rejected;expect(ports[1].terminated).toBe(true)
  const retry=client.run({kind:'materialWall',options:cases[2].request}),fresh=ports.at(-1)
  fresh.onmessage({data:{version:1,id:fresh.request.id,kind:'materialWall',ok:true,result:cases[2].result}})
  await expect(retry).resolves.toMatchObject({converged:true})
 }finally{client.dispose()}
})
