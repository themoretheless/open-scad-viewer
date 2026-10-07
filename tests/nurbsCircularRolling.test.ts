import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateEpicycloidNurbsCurve,approximateHypocycloidNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent inside and outside rolling-circle formulas and cusps',()=>{
 for(const inside of [false,true]){
  const constructor=inside?approximateHypocycloidNurbsCurve:approximateEpicycloidNurbsCurve
  const a=constructor([3,4,5],3,1,0,2,1e-4),rho=inside?2:4,sign=inside?1:-1
  for(let i=0;i<=200;i++){
   const theta=2*i/200,p=evaluateNurbsCurve(a.curve,i/200).point
   const exact=[3+rho*Math.cos(theta)+sign*Math.cos(rho*theta),4+rho*Math.sin(theta)-Math.sin(rho*theta),5]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
  expect(evaluateNurbsCurve(a.curve,0).d1).toEqual([0,0,0])
 }
 expect(()=>approximateHypocycloidNurbsCurve([0,0,0],1,2,0,2,1e-4)).toThrow()
})
for(const variant of ['epicycloid','hypocycloid']){
 it(`checks ${variant} radius and angular dimensions through packaged Rush`,()=>{
  const source=readFileSync(`examples/rush/${variant}-extrusion.r`,'utf8')
  expect(compileRushFrontend(source).document.nodes.find(n=>n.op===`${variant}_curve`)).toMatchObject({fixed_radius:3,rolling_radius:1,end_degrees:30})
  expect(()=>compileRushFrontend(source.replace('fixed_radius: 3mm','fixed_radius: 3deg'))).toThrow()
  expect(()=>compileRushFrontend(source.replace('end_degrees: 30deg','end_degrees: 30mm'))).toThrow()
  expect(()=>compileRushFrontend(source.replace(',rolling_radius: 1mm',''))).toThrow()
 })
 it(`preserves ${variant} formula and uncertified construction report in the graph`,()=>{
  const graph=compileRushFrontend(readFileSync(`examples/rush/${variant}-extrusion.r`,'utf8'))
  const curve=graph.document.nodes.find(n=>n.op===`${variant}_curve`)!
  const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:1}]})
  const theta=Math.PI/6,inside=variant==='hypocycloid',rho=inside?2:4,sign=inside?1:-1,p=built.evaluations[0]!.point
  expect(p[0]).toBeCloseTo(rho*Math.cos(theta)+sign*Math.cos(rho*theta),10)
  expect(p[1]).toBeCloseTo(rho*Math.sin(theta)-Math.sin(rho*theta),10)
  expect(built.report.error_bound_certified).toBe(false)
  expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
 })
}
