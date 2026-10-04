import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {MainSolidWorkerClient} from '../src/services/mainSolidWorkerClient'
import {selfIntersectionExpectation,validSelfIntersection} from '../src/services/solidSelfIntersection'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/surface-contact-native/face-contact-api-fixtures.json',import.meta.url),'utf8')).cases[0]
const {model,toleranceUv,op,...limits}=fixture.request
const expectation=selfIntersectionExpectation(model,toleranceUv,limits,6)
const report={...fixture.result,scope:'within-face-and-distinct-face-pairs',maxSpans:6,spans:6,allFacesInjective:true,absenceProven:true,faces:model.faces.map((_:unknown,face:number)=>({face,result:{proven:true,projection:[0,1],contractionUpper:0.01,spans:1,reason:'global-projection-contraction'}}))}
it('validates owned quotient proofs and their shared budget without certifying pairs',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/pole-quotient/combined-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 const e=selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),r=fixture.result
 expect(validSelfIntersection(e,r)).toBe(true)
 expect(r.allFacesInjective).toBe(true);expect(r.absenceProven).toBe(false)
 expect(r.faces.filter((f:any)=>f.quotientProof?.proven)).toHaveLength(2)
 for(const patch of [{collapsedEnd:1},{poleEdge:999},{poleVertex:999},{sourceFrame:null},{sourceFrame:[[0,0,0],[1,0,0],[0,1,0],[0,0,1]]},{cells:255},{cells:0},{proven:false},{weightedBounds:null},{weightedBounds:[0,1,0,1]},{dominanceMarginLower:Infinity},{dominanceMarginLower:1e10},{bandMarginsLower:[1]},{reason:'global-projection-contraction'}]){
  const copy=structuredClone(r);Object.assign(copy.faces[0].quotientProof,patch)
  expect(validSelfIntersection(e,copy)).toBe(false)
 }
 // Bind every surface control, including controls outside the projection frame.
 const alteredSource=structuredClone(model)
 alteredSource.faces[0].surface.controlPoints[1][1][2]+=0.001
 expect(validSelfIntersection(selfIntersectionExpectation(alteredSource,toleranceUv,limits,maxSpans),r)).toBe(false)
 const alteredReply=structuredClone(r)
 alteredReply.faces[0].quotientProof.sourceSurface.controlPoints[1][1][2]+=0.001
 expect(validSelfIntersection(e,alteredReply)).toBe(false)
 for(const mutate of [
  (s:any)=>{s.knotsU[0]-=0.001},
  (s:any)=>{s.knotsV[s.knotsV.length-1]+=0.001},
  (s:any)=>{s.degreeU+=1},
  (s:any)=>{s.periodicU=true},
  (s:any)=>{s.periodicV=true},
 ]){
  const copy=structuredClone(r);mutate(copy.faces[0].quotientProof.sourceSurface)
  expect(validSelfIntersection(e,copy)).toBe(false)
 }
 const changedWeight=structuredClone(r)
 changedWeight.faces[0].quotientProof.sourceSurface.weights[1][1]*=1.001
 expect(validSelfIntersection(e,changedWeight)).toBe(false)
 const removed=structuredClone(r);removed.faces[0].quotientProof=null
 expect(validSelfIntersection(e,removed)).toBe(false)
 const undeclared=structuredClone(model);undeclared.edges[r.faces[0].quotientProof.poleEdge].vertices[1]=1
 expect(validSelfIntersection(selfIntersectionExpectation(undeclared,toleranceUv,limits,maxSpans),r)).toBe(false)
 expect(validSelfIntersection(e,{...r,absenceProven:true})).toBe(false)
})
it('binds native polar proof coefficients to the source and leaves collapsed tips unresolved',()=>{
 const f=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/polar-injectivity/native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=f.request
 const e=selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),r=f.result
 expect(validSelfIntersection(e,r)).toBe(true)
 expect(r.faces.filter((f:any)=>f.result?.proven)).toHaveLength(25)
 expect(r.absenceProven).toBe(false)
 for(const patch of [{polarProjection:null},{polarProjection:[[1,0,0,0],[0,1,0,0],[0,0,1,0]]},{polarProjection:[[NaN,0,0,0],[0,1,0,0],[0,0,1,0]]},{spans:193},{spans:448},{contractionUpper:1},{projection:[0,1]},{proven:false}]){
  const copy=structuredClone(r);Object.assign(copy.faces[5].result,patch)
  expect(validSelfIntersection(e,copy)).toBe(false)
 }
 const forged=structuredClone(r)
 Object.assign(forged.faces[0].result,{proven:true,projection:[0,1],reason:'global-projection-contraction',spans:1,contractionUpper:0.01})
 expect(validSelfIntersection(e,forged)).toBe(false)
})
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

it('qualifies actual WASM complete cylinder face and pair proofs',async()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/curved-volume-2026-10-01/complete-cylinder-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 const {inspectSelfIntersection}=await import('../src/services/solidSelfIntersection')
 const before=JSON.stringify(model),r=inspectSelfIntersection(model,toleranceUv,limits,maxSpans)
 expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),r)).toBe(true)
 expect(r.allFacesInjective).toBe(true);expect(r.spans).toBe(102)
 expect(r.faces).toMatchObject(fixture.result.faces)
 expect(r.absenceProven).toBe(true);expect(JSON.stringify(model)).toBe(before)
})

it('validates native complete-cylinder pair classification with linear face proofs',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/curved-volume-2026-10-01/complete-cylinder-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),fixture.result)).toBe(true)
 expect(fixture.result.absenceProven).toBe(true)
})

it('checks native perspective face proofs against source coefficients and complete work counts',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/curved-volume-2026-10-01/projective-face-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 const e=selfIntersectionExpectation(model,toleranceUv,limits,maxSpans),r=fixture.result
 expect(validSelfIntersection(e,r)).toBe(true);expect(r.spans).toBe(968)
 expect(r.allFacesInjective).toBe(true);expect(r.absenceProven).toBe(false)
 for(const patch of [
  {projectiveProjection:null},{projectiveProjection:[[1,0,0,0],[0,1,0,0],[0,0,1,4]]},
  {projectiveProjection:[[1,0,0,0],[0,1,0,0],[0,0,1,NaN]]},
  {projectiveProjection:[[2,0,0,0],[0,1,0,0],[0,0,1,3]]},
  {projectiveProjection:[[1,0,0,0],[0,1,0,0]]},
  {spans:112},{spans:129},{projection:[0,1]},{linearProjection:[[1,1,0],[0,0,1]]},
  {contractionUpper:1},{reason:'global-linear-projection-contraction'},{proven:false}
 ]){
  const copy=structuredClone(r);Object.assign(copy.faces[0].result,patch)
  expect(validSelfIntersection(e,copy)).toBe(false)
 }
 expect(validSelfIntersection(e,{...r,absenceProven:true})).toBe(false)
})

it('authors actual WASM sphere equators with matching rational edge and trim traversals',async()=>{
 const {callGeometryRust}=await import('../src/services/geometry/kernel')
 const model=callGeometryRust<any>('brep_nurbs_sphere',{radius:3})
 expect(model.faces).toHaveLength(8);expect(model.edges).toHaveLength(12)
 for(const face of model.faces){
  const equator=model.loops[face.outer].coedges[1]
  expect(equator.pcurve.weights).toEqual([1,1,2])
  const weights=model.edges[equator.edge].curve.weights
  expect(equator.reversed?[...weights].reverse():weights).toEqual([1,1,2])
 }
})

it('qualifies actual WASM perspective face proofs and exhausted aggregate budgets',async()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/curved-volume-2026-10-01/projective-face-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 const {inspectSelfIntersection}=await import('../src/services/solidSelfIntersection')
 const before=JSON.stringify(model)
 for(const budget of [maxSpans,967,1]){
  const r=inspectSelfIntersection(model,toleranceUv,limits,budget)
  expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,budget),r)).toBe(true)
  expect(r.absenceProven).toBe(false);expect(r.allFacesInjective).toBe(budget===maxSpans)
  if(budget===maxSpans){expect(r.faces).toEqual(fixture.result.faces.map((f:any)=>({...f,quotientProof:null,result:{...f.result,polarProjection:null}})));expect(r.spans).toBe(968)}
  else expect(r.spans).toBe(budget)
 }
 expect(JSON.stringify(model)).toBe(before)
})

it('qualifies owned quotient charts in actual WASM and keeps exhausted coverage explicit',async()=>{
 const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-02/pole-quotient/combined-native.json',import.meta.url),'utf8'))
 const {model,toleranceUv,maxSpans,op,...limits}=fixture.request
 const {inspectSelfIntersection}=await import('../src/services/solidSelfIntersection')
 const before=JSON.stringify(model)
 for(const budget of [maxSpans,600,256,255]){
  const r=inspectSelfIntersection(model,toleranceUv,limits,budget)
  expect(validSelfIntersection(selfIntersectionExpectation(model,toleranceUv,limits,budget),r)).toBe(true)
  expect(r.absenceProven).toBe(false)
  expect(r.allFacesInjective).toBe(budget===maxSpans)
  if(budget===maxSpans){
   expect(r.faces.filter(f=>f.result?.proven)).toHaveLength(25)
   expect(r.faces.filter(f=>f.quotientProof?.proven)).toHaveLength(2)
   expect(r.faces).toEqual(fixture.result.faces)
  }else if(budget===256){
   expect(r.faces[0].quotientProof?.proven).toBe(true)
   expect(r.spans).toBe(256)
   expect(r.faces.slice(1).every(f=>f.result===null&&f.quotientProof===null)).toBe(true)
  }else if(budget===255)expect(r.faces[0].quotientProof).toBe(null)
 }
 expect(JSON.stringify(model)).toBe(before)
})

it('binds explicit boundary audit and exact contacts to source, budgets and ownership',()=>{
 const audit={exactWork:1000000,trimPairs:1000,trimCells:10000,trimDomainCells:100000}
 const e=selfIntersectionExpectation(model,toleranceUv,limits,6,audit)
 const r:any=structuredClone(report)
 r.boundaryEmbedding={proven:true,exactAgreement:true,exactJoins:true,exactWork:20,trimValid:true,positiveTrimWinding:true,trimPairs:3,trimCells:5,trimDomainCells:7,limits:audit,sourceModel:structuredClone(model)}
 expect(validSelfIntersection(e,r)).toBe(true)
 expect(validSelfIntersection(expectation,r)).toBe(false)
 const pair=r.pairs.find((p:any)=>p.status==='shared-boundary')
 expect(pair).toBeDefined()
 const edge=pair.sharedBoundary.edge
 pair.sharedBoundary={kind:'exact-hull',faces:pair.faces,edges:[edge],vertex:null,contactEnclosure:[[0,1],[0,1],[0,1]],joinedProof:null}
 expect(validSelfIntersection(e,r)).toBe(true)
 for(const patch of [{edges:[999]},{edges:[]},{vertex:999},{faces:[1,0]},{contactEnclosure:[[1,0],[0,1],[0,1]]},{joinedProof:{proven:true}}]){
  const copy=structuredClone(r);Object.assign(copy.pairs.find((p:any)=>p.status==='shared-boundary').sharedBoundary,patch)
  expect(validSelfIntersection(e,copy)).toBe(false)
 }
 for(const patch of [{exactAgreement:false},{exactJoins:false},{trimValid:false},{positiveTrimWinding:false},{proven:false},{exactWork:1000001},{trimCells:10001}]){
  const copy=structuredClone(r);Object.assign(copy.boundaryEmbedding,patch)
  expect(validSelfIntersection(e,copy)).toBe(false)
 }
 const changed=structuredClone(r);changed.boundaryEmbedding.sourceModel.faces[0].surface.controlPoints[0][0][0]+=0.01
 expect(validSelfIntersection(e,changed)).toBe(false)
 const stale=structuredClone(model);stale.faces[0].surface.weights[0][0]*=1.01
 expect(validSelfIntersection(selfIntersectionExpectation(stale,toleranceUv,limits,6,audit),r)).toBe(false)
 const budget=structuredClone(r);budget.boundaryEmbedding.limits.exactWork=1
 expect(validSelfIntersection(e,budget)).toBe(false)
})

it('cancels an audited request when the audit mode changes and ignores its late reply',async()=>{
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const base={kind:'selfIntersection' as const,model,toleranceUv,limits,maxSpans:6}
 try{
  const audited=client.run({...base,boundaryAudit:{exactWork:1000000,trimPairs:1000,trimCells:10000,trimDomainCells:100000}})
  const cancelled=expect(audited).rejects.toMatchObject({name:'AbortError'})
  const stale=ports[0].onmessage,old=ports[0].request
  const fresh=client.run(base);await cancelled
  expect(ports[0].terminated).toBe(true)
  let settled=false;void fresh.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:'selfIntersection',ok:true,result:report}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:'selfIntersection',ok:true,result:report}})
  await expect(fresh).resolves.toMatchObject({absenceProven:true})
 }finally{client.dispose()}
})
