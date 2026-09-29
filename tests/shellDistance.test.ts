import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {measureShellDistance,type ShellDistanceOptions} from '../src/services/solidMeasurements'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
const read=(name:string)=>JSON.parse(readFileSync(new URL(`./fixtures/${name}-distance.json`,import.meta.url),'utf8'))
const options=(f:any):ShellDistanceOptions=>({a:structuredClone(f.document.bodies[0].brep),b:structuredClone(f.document.bodies[1].brep),toleranceMm:.001,toleranceUv:1e-7,maxCells:1000000,maxDomainCells:8000000})
it('finds the global shell minimum over trimmed faces and rational spheres in WASM',()=>{
 for(const name of ['face','shell']){
  const f=read(name),o=options(f),before=JSON.stringify(o),r=measureShellDistance(o)
  expect(r).toMatchObject({converged:true,scope:'boundary-shells-bounded-joins',containment:'not-classified'})
  expect(r.distanceIntervalMm[0]).toBeLessThanOrEqual(f.expectedMm)
  expect(r.distanceIntervalMm[1]).toBeGreaterThanOrEqual(f.expectedMm)
  expect(r.distanceIntervalMm[1]!-r.distanceIntervalMm[0]).toBeLessThanOrEqual(.001)
  expect(r.pairs).toBe(o.a.faces.length*o.b.faces.length)
  expect(r.faces!.every((face,i)=>!![o.a,o.b][i].faces[face])).toBe(true)
  expect(mainSolidResult(mainSolidExpectation({kind:'shellDistance',options:o}),r)).toBe(true)
  expect(JSON.stringify(o)).toBe(before)
 }
})
it('keeps unknown upper bounds when the shared domain budget is exhausted',()=>{
 const o={...options(read('face')),maxCells:1,maxDomainCells:1},r=measureShellDistance(o)
 expect(r).toMatchObject({converged:false,reason:'domain-work-limit',points:null,faces:null,parameters:null,pointEnclosures:null})
 expect(r.distanceIntervalMm[1]).toBeNull()
 expect(mainSolidResult(mainSolidExpectation({kind:'shellDistance',options:o}),r)).toBe(true)
})
it('rejects invalid face identities, budgets, volume claims and false completion',()=>{
 const fixture=read('shell'),o=options(fixture),r=fixture.result,e=mainSolidExpectation({kind:'shellDistance',options:o})
 expect(mainSolidResult(e,r)).toBe(true)
 for(const patch of [{containment:'disjoint'},{scope:'filled-volumes'},{maxCells:1},{maxDomainCells:1},{pairs:1},{evaluatedPairs:r.pairs+1},{faces:[99999,0]},{parameters:[[999,0],[0,0]]},{faces:null},{points:null},{distanceIntervalMm:[0,null]},{reason:'work-limit'},{converged:false},{domainCells:0}])expect(mainSolidResult(e,{...r,...patch})).toBe(false)
})
it('recovers from rejected shell budgets in the worker',async()=>{
 const messages:any[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m)),o=options(read('shell'))
 await handler({version:1,id:1,job:{kind:'shellDistance',options:{...o,maxCells:0}}})
 expect(messages[0]).toMatchObject({id:1,ok:false})
 await handler({version:1,id:2,job:{kind:'shellDistance',options:o}})
 expect(messages[1]).toMatchObject({id:2,ok:true,result:{converged:true,containment:'not-classified'}})
})
