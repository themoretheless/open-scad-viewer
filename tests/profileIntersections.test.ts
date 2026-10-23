import {describe,it,expect} from 'vitest'
import {inspectProfileIntersections} from '../src/services/geometry/profileIntersections'
import type {NurbsCurve} from '../src/services/nurbsCurve'
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1]})
describe('retained profile intersection Rust/WASM boundary',()=>{
  it('locates a rational crossing while retaining source geometry and indices',()=>{
    const arch:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,1],[2,0]],weights:[1,.8,1]}
    const loops=[[arch],[line([1,-1],[1,2])]],before=JSON.stringify(loops)
    const r=inspectProfileIntersections(loops)
    expect(r).toMatchObject({complete:true,totalPairs:1,visitedPairs:1,resolvedPairs:1,unvisitedPairs:0})
    expect(r.pairs[0]).toMatchObject({first:{loop:0,curve:0},second:{loop:1,curve:0}})
    const events=r.pairs[0]!.report.components
    expect(events).toHaveLength(1)
    expect(events[0]!.kind).toBe('point')
    expect((events[0]!.point as number[])[1]).toBeCloseTo(4/9,7)
    expect(JSON.stringify(loops)).toBe(before)
  })
  it('reports coincident definitions as overlap, including ordinary end contacts',()=>{
    const a=line([0,0],[1,0])
    const r=inspectProfileIntersections([[a,a]])
    expect(r.pairs[0]!.report.components.some(c=>c.kind==='overlap')).toBe(true)
    const join=inspectProfileIntersections([[a,line([1,0],[1,1])]])
    expect(join.pairs[0]!.report.components.some(c=>c.kind==='point')).toBe(true)
    // The diagnostic does not label a shared endpoint as an invalid profile.
    expect(join).not.toHaveProperty('accepted')
  })
  it('retains unresolved work and unvisited pairs when global limits are reached',()=>{
    const loops=[[line([0,0],[2,2]),line([0,2],[2,0]),line([3,0],[3,2])]]
    const pairLimited=inspectProfileIntersections(loops,{maxPairs:1})
    expect(pairLimited).toMatchObject({complete:false,totalPairs:3,visitedPairs:1,unvisitedPairs:2})
    const boxLimited=inspectProfileIntersections(loops,{maxBoxes:1})
    expect(boxLimited.complete).toBe(false)
    expect(boxLimited.boxesVisited).toBeLessThanOrEqual(1)
    expect(boxLimited.pairs[0]!.report.unresolved.length).toBeGreaterThan(0)
  })
})

it('runs profile pair diagnostics through the worker and validates its source snapshot',async()=>{
  const {createMainSolidWorkerHandler}=await import('../src/services/mainSolidWorkerRuntime')
  const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
  const messages:import('../src/services/mainSolidProtocol').MainSolidResponse[]=[]
  const handle=createMainSolidWorkerHandler(message=>messages.push(message))
  const job={kind:'profileIntersections' as const,loops:[[line([0,0],[2,2])],[line([0,2],[2,0])]],options:{toleranceMm:1e-7,maxPairs:128,maxBoxes:32768}}
  const before=JSON.stringify(job),expected=mainSolidExpectation(job)
  await handle({version:1,id:1,job})
  expect(messages[0]).toMatchObject({id:1,ok:true,kind:'profileIntersections',result:{complete:true,totalPairs:1}})
  const response=messages[0]!
  if(!response.ok)throw Error('Worker did not return diagnostics')
  expect(mainSolidResult(expected,response.result)).toBe(true)
  expect(JSON.stringify(job)).toBe(before)
  const forged=structuredClone(response.result) as import('../src/services/geometry/profileIntersections').ProfileIntersectionReport
  forged.pairs[0]!.first.loop=1
  expect(mainSolidResult(expected,forged)).toBe(false)
})
