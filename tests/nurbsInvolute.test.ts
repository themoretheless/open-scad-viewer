import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateInvoluteNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
it('matches the independent involute formula and endpoint tangent across zero',()=>{
 for(const [start,end] of [[0,2],[-2,1],[1,4]]){
  const a=approximateInvoluteNurbsCurve([3,4,5],2,start!,end!,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,theta=start!+(end!-start!)*t,p=evaluateNurbsCurve(a.curve,t).point
   const exact=[3+2*(Math.cos(theta)+theta*Math.sin(theta)),4+2*(Math.sin(theta)-theta*Math.cos(theta)),5]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
  for(const [t,theta] of [[0,start!],[1,end!]]){
   const d=evaluateNurbsCurve(a.curve,t!).d1!
   expect(d[0]).toBeCloseTo(2*theta!*Math.cos(theta!)*(end!-start!),10)
   expect(d[1]).toBeCloseTo(2*theta!*Math.sin(theta!)*(end!-start!),10)
  }
 }
 expect(()=>approximateInvoluteNurbsCurve([0,0,0],2,2,0,1e-4)).toThrow()
})
it('checks angular dimensions and mandatory budget through packaged Rush',()=>{
 const source=readFileSync('examples/rush/involute-extrusion.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='involute_curve')).toMatchObject({start_degrees:0,end_degrees:60,max_deviation:.0001})
 expect(()=>compileModelGraphText(source.replace('end_degrees: 60deg','end_degrees: 60mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace(',max_deviation: 0.0001mm',''))).toThrow()
})
it('converts graph angles to radians and keeps the approximation uncertified',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/involute-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='involute_curve')!
 const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:1}]})
 const theta=Math.PI/3,point=built.evaluations[0]!.point
 expect(point[0]).toBeCloseTo(5*(Math.cos(theta)+theta*Math.sin(theta)),10)
 expect(point[1]).toBeCloseTo(5*(Math.sin(theta)-theta*Math.cos(theta)),10)
 expect(built.report.foundation_certificate).not.toBeNull()
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
})
