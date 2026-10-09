import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateTrochoidNurbsCurve,approximateCycloidNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent curtate cycloid prolate and linear limit formulas',()=>{
 for(const distance of [0,1,2,3]){
  const a=approximateTrochoidNurbsCurve([3,4,5],2,distance,-1,5,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,theta=-1+6*t,p=evaluateNurbsCurve(a.curve,t).point,exact=[3+2*theta-distance*Math.sin(theta),6-distance*Math.cos(theta),5]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 const cusp=evaluateNurbsCurve(approximateCycloidNurbsCurve([0,0,0],2,0,2,1e-4).curve,0)
 expect(cusp.point).toEqual([0,0,0]);expect(cusp.d1).toEqual([0,0,0])
})
for(const variant of ['trochoid','cycloid']){
 it(`checks ${variant} radius dimensions through packaged Rush`,()=>{
  const source=readFileSync(`examples/rush/${variant}-extrusion.r`,'utf8')
  expect(compileRushFrontend(source).document.nodes.find(n=>n.op===`${variant}_curve`)).toMatchObject({end_degrees:60,max_deviation:.0001})
  const radius=variant==='trochoid'?'tracing_radius: 7mm':'radius: 5mm'
  expect(()=>compileRushFrontend(source.replace(radius,radius.replace('mm','deg')))).toThrow()
  expect(()=>compileRushFrontend(source.replace('end_degrees: 60deg','end_degrees: 60mm'))).toThrow()
  expect(()=>compileRushFrontend(source.replace(',max_deviation: 0.0001mm',''))).toThrow()
 })
 it(`preserves ${variant} geometry and uncertified approximation in the graph`,()=>{
  const graph=compileRushFrontend(readFileSync(`examples/rush/${variant}-extrusion.r`,'utf8'))
  const curve=graph.document.nodes.find(n=>n.op===`${variant}_curve`)!
  const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:1}]})
  const theta=Math.PI/3,distance=variant==='trochoid'?7:5,p=built.evaluations[0]!.point
  expect(p[0]).toBeCloseTo(5*theta-distance*Math.sin(theta),10)
  expect(p[1]).toBeCloseTo(5-distance*Math.cos(theta),10)
  expect(built.report.error_bound_certified).toBe(false)
  expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
 })
}
