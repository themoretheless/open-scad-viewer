import {describe,it,expect} from 'vitest'
import {validateProfileIntersectionReport} from '../src/services/geometry/profileIntersections'
import type {NurbsCurve} from '../src/services/nurbsCurve'
const curve:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0],[1,1]],weights:[1,1]}
const loops=[[curve],[curve]]
const options={toleranceMm:1e-7,maxPairs:128,maxBoxes:32768}
function report() {
  return {scope:'distinct-profile-segment-pairs',complete:true,totalPairs:1,visitedPairs:1,resolvedPairs:1,unvisitedPairs:0,
    boxesVisited:12,...options,pairs:[{first:{loop:0,curve:0},second:{loop:1,curve:0},report:{
      version:'nurbs-foundation/5',kind:'curve_curve',rounding:'binary64-nextafter-outward',evidence:{},
      coverage:{method:'Bernstein',complete:true,boxesVisited:12,resourceLimit:8192,bernsteinExcluded:10,krawczykIsolated:1},
      components:[{kind:'point',first:.5,second:.5,firstInterval:[.49,.51],secondInterval:[.49,.51],point:[.5,.5,0],residual:0,geometryEnclosure:[[.49,.51],[.49,.51],[0,0]]}],unresolved:[],
    }}]}
}
describe('profile intersection ownership and work contract',()=>{
  it('retains the report and source definitions without mutation',()=>{
    const input=report(),before=JSON.stringify(loops)
    expect(validateProfileIntersectionReport(input,loops,options)).toBe(input)
    expect(JSON.stringify(loops)).toBe(before)
  })
  it.each([
    ['source ownership',(r:ReturnType<typeof report>)=>{r.pairs[0]!.second.loop=0}],
    ['aggregate boxes',(r:ReturnType<typeof report>)=>{r.boxesVisited=11}],
    ['pair budget',(r:ReturnType<typeof report>)=>{r.maxPairs=129}],
    ['resolved count',(r:ReturnType<typeof report>)=>{r.resolvedPairs=0}],
    ['parameter domain',(r:ReturnType<typeof report>)=>{r.pairs[0]!.report.components[0]!.firstInterval=[-1,.51]}],
    ['parameter ownership',(r:ReturnType<typeof report>)=>{r.pairs[0]!.report.components[0]!.first=.8}],
    ['finite geometry',(r:ReturnType<typeof report>)=>{r.pairs[0]!.report.components[0]!.point[0]=NaN}],
    ['geometry enclosure',(r:ReturnType<typeof report>)=>{r.pairs[0]!.report.components[0]!.point[0]=100}],
    ['box budget',(r:ReturnType<typeof report>)=>{r.pairs[0]!.report.coverage.resourceLimit=9000}],
  ])('refuses inconsistent %s',(_,mutate)=>{
    const input=report();mutate(input)
    expect(()=>validateProfileIntersectionReport(input,loops,options)).toThrow(/Invalid profile intersection report/)
  })
  it('preserves incomplete coverage instead of claiming absence',()=>{
    const input=report()
    input.complete=false;input.resolvedPairs=0;input.pairs[0]!.report.coverage.complete=false
    const unresolved={parameterBox:[0,1,0,1],reason:'resource_boundary'}
    const raw={...input,pairs:[{...input.pairs[0],report:{...input.pairs[0]!.report,unresolved:[unresolved]}}]}
    expect(validateProfileIntersectionReport(raw,loops,options).complete).toBe(false)
    raw.complete=true
    expect(()=>validateProfileIntersectionReport(raw,loops,options)).toThrow()
  })
  it('accepts an unvisited tail only when the aggregate remains incomplete',()=>{
    const input={...report(),complete:false,totalPairs:3,unvisitedPairs:2}
    expect(validateProfileIntersectionReport(input,[[curve],[curve],[curve]],options).unvisitedPairs).toBe(2)
  })
})

it('validates diagnostics attached to refused profile preparation without allowing an accepted contour',async()=>{
  const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
  const {emptyDirectDocument}=await import('../src/services/directModeling')
  const document=emptyDirectDocument()
  document.curves=[{id:'a',name:'A',curve},{id:'b',name:'B',curve}]
  const expected=mainSolidExpectation({kind:'profilePrepare',document,ids:['a','b'],tolerance:0})
  const intersections=report();intersections.maxBoxes=8192
  const input={document,id:'a',plane:{origin:[0,0,0],u:[1,0,0],v:[0,1,0]},report:{
    accepted:false,reason:'invalid-contour',points:[],defects:[],connectors:[],
    curveSources:[{chain:0,segment:0,reversed:false,connector:false},{chain:1,segment:0,reversed:false,connector:false}],
    diagnosticLoops:[[curve,curve]],intersections:{...intersections,pairs:[{...intersections.pairs[0],second:{loop:0,curve:1}}]},
    diagnosticDisplay:[{curve:0,kind:'intersection',points:[[0,0],[1,1]]}],
  }}
  expect(mainSolidResult(expected,input)).toBe(true)
  expect(mainSolidResult(expected,{...input,report:{...input.report,diagnosticDisplay:[null]}})).toBe(false)
  expect(mainSolidResult(expected,{...input,report:{...input.report,accepted:true,reason:'accepted'}})).toBe(false)
  expect(mainSolidResult(expected,{...input,report:{...input.report,diagnosticDisplay:[{curve:8,kind:'intersection',points:[[0,0],[1,1]]}]}})).toBe(false)
  expect(mainSolidResult(expected,{...input,report:{...input.report,diagnosticLoops:undefined}})).toBe(false)
  expect(mainSolidResult(expected,{...input,report:{...input.report,diagnosticDisplay:[...input.report.diagnosticDisplay,...input.report.diagnosticDisplay]}})).toBe(false)
})
