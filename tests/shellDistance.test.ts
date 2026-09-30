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

it('reserves work for every pair of rotated nested shells',()=>{
 const base=JSON.parse(readFileSync(new URL('./fixtures/boundary-agreement.json',import.meta.url),'utf8')).cases.find((c:any)=>c.name==='normal').document.bodies[0].brep
 const transform=(scale:number,offset:number)=>{
  const m=structuredClone(base),c=Math.cos(.37),s=Math.sin(.37)
  const point=(p:number[])=>{const x=p[0]*scale+offset,y=p[1]*scale+offset;return [c*x-s*y+13,s*x+c*y-7,p[2]*scale+offset+3]}
  for(const v of m.vertices)v.point=point(v.point)
  for(const e of m.edges)e.curve.controlPoints=e.curve.controlPoints.map(point)
  for(const f of m.faces)f.surface.controlPoints=f.surface.controlPoints.map((row:number[][])=>row.map(point))
  return m
 }
 const o:ShellDistanceOptions={a:transform(6,2),b:transform(10,0),toleranceMm:1e-4,toleranceUv:1e-7,maxCells:10000,maxDomainCells:100000},before=JSON.stringify(o),r=measureShellDistance(o)
 expect(r.distanceIntervalMm[0]).toBeGreaterThan(0)
 expect(r.distanceIntervalMm[0]).toBeLessThanOrEqual(2)
 expect(r.distanceIntervalMm[1]).not.toBeNull()
 expect(r.distanceIntervalMm[1]!).toBeGreaterThanOrEqual(2-1e-12)
 expect(r.cells).toBeLessThanOrEqual(o.maxCells)
 expect(r.domainCells).toBeLessThanOrEqual(o.maxDomainCells)
 expect(r.containment).toBe('not-classified')
 expect(mainSolidResult(mainSolidExpectation({kind:'shellDistance',options:o}),r)).toBe(true)
 expect(JSON.stringify(o)).toBe(before)
})
