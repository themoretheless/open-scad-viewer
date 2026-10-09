import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {MainSolidWorkerClient} from '../src/services/mainSolidWorkerClient'
const fixtures=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/surface-contact-native/face-contact-api-fixtures.json',import.meta.url),'utf8')).cases
function job(q:any){const {model,toleranceUv,op,...limits}=q;return {kind:'faceContacts' as const,model,toleranceUv,limits}}
it('accepts native full and partial reports without turning visited into disjoint',()=>{
 for(const f of fixtures)for(const [q,r] of [[f.request,f.result],[f.partialRequest,f.partialResult]])expect(mainSolidResult(mainSolidExpectation(job(q)),r)).toBe(true)
})
it('rejects forged identities, coverage, counts, budgets and output truncation',()=>{
 for(const f of fixtures){
  const r=f.result,e=mainSolidExpectation(job(f.request))
  for(const patch of [{allPairsDisjoint:true},{allPairsVisited:false},{visitedPairs:0},{nextPair:[0,1]},{cells:r.cells+1},{domainCells:r.domainCells+1},{unresolvedBoxCount:r.unresolvedBoxCount+1},{exportedBoxCount:99},{boxesTruncated:!r.boxesTruncated},{solidGeometryStatus:'certified'},{pairs:[...r.pairs].reverse()},{limits:{...r.limits,maxCells:1}}])expect(mainSolidResult(e,{...r,...patch})).toBe(false)
  for(const patch of [{faces:[0,999]},{cells:-1},{status:'safe'},{unresolvedBoxCount:-1},{unresolvedBoxes:[[[[0,2],[0,1]],[[0,1],[0,1]]]]}]){
   const pairs=r.pairs.map((p:any,i:number)=>i===0?{...p,...patch}:p)
   expect(mainSolidResult(e,{...r,pairs})).toBe(false)
  }
 }
})
it('rejects malformed or out-of-domain contact witnesses',()=>{
 const f=fixtures[1],r=f.result,e=mainSolidExpectation(job(f.request)),w=r.pairs[0].witness
 for(const witness of [null,{...w,contractionUpper:0.5},{...w,pointIntervalMm:[[NaN,1],[0,1],[0,1]]},{...w,firstUv:[[0,2],[0,1]]},{...w,secondUv:[[1,0],[0,1]]}]){
  const pairs=r.pairs.map((p:any,i:number)=>i===0?{...p,witness}:p)
  expect(mainSolidResult(e,{...r,pairs})).toBe(false)
 }
})
it('terminates cancelled contact work and ignores a captured stale reply',async()=>{
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const f=fixtures[1],j=job(f.request)
 try {
  const first=client.run(j),cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run(j);await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:'faceContacts',ok:true,result:f.result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'faceContacts',ok:true,result:f.result}})
  await expect(second).resolves.toMatchObject({contactPairCount:1})
 }finally{client.dispose()}
})
it('matches native fixtures through the production WASM operation',async()=>{
 const {inspectFaceContacts}=await import('../src/services/solidFaceContacts')
 for(const f of fixtures)for(const [q,r] of [[f.request,f.result],[f.partialRequest,f.partialResult]]){
  const j=job(q),before=JSON.stringify(j.model),result=inspectFaceContacts(j.model,j.toleranceUv,j.limits)
  expect(result).toEqual({...r,groupedPairs:0,groupCells:0,disjointGroups:[]})
  expect(mainSolidResult(mainSolidExpectation(j),result)).toBe(true)
  expect(JSON.stringify(j.model)).toBe(before)
 }
})
it('worker recovers after a rejected budget using the real WASM command',async()=>{
 const {createMainSolidWorkerHandler}=await import('../src/services/mainSolidWorkerRuntime')
 const messages:any[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m)),j=job(fixtures[1].request)
 await handle({version:1,id:1,job:{...j,limits:{...j.limits,maxCells:0}}})
 await handle({version:1,id:2,job:j})
 expect(messages[0]).toMatchObject({id:1,ok:false})
 expect(messages[1]).toMatchObject({id:2,kind:'faceContacts',ok:true,result:{contactPairCount:1,visitedPairs:15}})
})

it('rejects forged shared-edge certificates and incomplete classification flags',()=>{
 const f=fixtures[0],r=f.result,e=mainSolidExpectation(job(f.request)),i=r.pairs.findIndex((p:any)=>p.status==='shared-boundary'),c=r.pairs[i].sharedBoundary
 expect(r.sharedBoundaryPairCount).toBe(12)
 for(const patch of [{sharedBoundaryPairCount:11},{allPairsClassified:false},{unresolvedPairCount:1}])expect(mainSolidResult(e,{...r,...patch})).toBe(false)
 for(const sharedBoundary of [null,{...c,edge:999},{...c,planarFace:c.sidedFace},{...c,planarFace:999}]){
  const pairs=r.pairs.map((p:any,j:number)=>j===i?{...p,sharedBoundary}:p)
  expect(mainSolidResult(e,{...r,pairs})).toBe(false)
 }
})
it('accepts native opposite-side evidence and rejects forged roles or kind',()=>{
 const f=fixtures.find((c:any)=>c.name==='curved-shared-boundary'),r=f.result,e=mainSolidExpectation(job(f.request)),i=r.pairs.findIndex((p:any)=>p.sharedBoundary?.kind==='opposite-sides')
 expect(i).toBeGreaterThanOrEqual(0)
 expect(mainSolidResult(e,r)).toBe(true)
 const c=r.pairs[i].sharedBoundary
 for(const sharedBoundary of [{...c,kind:'planar-face'},{...c,kind:'unknown'},{...c,faces:[...c.faces].reverse()},{...c,faces:[0,999]},{...c,edge:999}]){
  const pairs=r.pairs.map((p:any,j:number)=>j===i?{...p,sharedBoundary}:p)
  expect(mainSolidResult(e,{...r,pairs})).toBe(false)
 }
})
