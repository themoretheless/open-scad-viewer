import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {inspectBoundaryAgreement} from '../src/services/solidBoundaryAgreement'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const fixtures=JSON.parse(readFileSync(new URL('./fixtures/boundary-agreement.json',import.meta.url),'utf8')).cases
it('finds the unsampled boundary defect in WASM and retains model identities',()=>{
 for(const f of fixtures){
  const before=JSON.stringify(f.model),r=inspectBoundaryAgreement(f.model,10000)
  expect(r).toMatchObject({...f.result})
  expect(mainSolidResult(mainSolidExpectation({kind:'boundaryAgreement',model:f.model,maxCells:10000}),r)).toBe(true)
  expect(r.lines.length).toBe(f.name==='defect'?1:0)
  expect(JSON.stringify(f.model)).toBe(before)
 }
})
it('retains every use when the budget cannot finish',()=>{
 const r=inspectBoundaryAgreement(fixtures[0].model,1)
 expect(r).toMatchObject({complete:false,allWithinTolerance:false,unresolvedCount:23,cells:1})
 expect(r.uses).toHaveLength(24)
})
it('rejects swapped identities, false success, forged witnesses and invalid lines',()=>{
 const f=fixtures[1],r=inspectBoundaryAgreement(f.model,10000),e=mainSolidExpectation({kind:'boundaryAgreement',model:f.model,maxCells:10000})
 for(const patch of [{complete:false},{allWithinTolerance:true},{mismatchCount:0},{unresolvedCount:1},{cells:0},{cells:10001},{maxCells:1},{toleranceMm:1},{solidGeometryStatus:'certified'},{uses:r.uses.slice(1)},{uses:[...r.uses].reverse()},{lines:[]},{lines:[{edge:999,points:[[0,0,0],[1,1,1]]}]}])expect(mainSolidResult(e,{...r,...patch})).toBe(false)
 const i=r.uses.findIndex(u=>u.status==='mismatch')
 for(const patch of [{face:999},{edge:999},{parameter:null},{parameter:2},{distanceIntervalMm:[0,1]},{distanceIntervalMm:[1,0]}]){
  const uses=r.uses.map((u,j)=>j===i?{...u,...patch}:u)
  expect(mainSolidResult(e,{...r,uses})).toBe(false)
 }
})
it('recovers in the worker after a bad budget',async()=>{
 const messages:any[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m)),model=fixtures[1].model
 await handler({version:1,id:1,job:{kind:'boundaryAgreement',model,maxCells:0}})
 await handler({version:1,id:2,job:{kind:'boundaryAgreement',model,maxCells:10000}})
 expect(messages[0]).toMatchObject({ok:false,id:1})
 expect(messages[1]).toMatchObject({ok:true,id:2,result:{complete:true,allWithinTolerance:false,mismatchCount:2}})
})
it('cancels boundary work and refuses its captured late response after restart',async()=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const model=fixtures[0].model,job={kind:'boundaryAgreement' as const,model,maxCells:10000},result=inspectBoundaryAgreement(model,10000)
 try{
  const first=client.run(job),cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,request=ports[0].request
  const second=client.run(job);await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:request.id,kind:'boundaryAgreement',ok:true,result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'boundaryAgreement',ok:true,result}})
  await expect(second).resolves.toMatchObject({allWithinTolerance:true})
 }finally{client.dispose()}
})
