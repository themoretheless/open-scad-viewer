import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateClothoidNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
function fresnel(s:number):[number,number]{
 const sum:[number,number]=[0,0];let factorial=1
 for(let n=0;n<32;n++){
  if(n)factorial*=n
  const term=s**(2*n+1)/(2**n*factorial*(2*n+1))
  if(n%4===0)sum[0]+=term;else if(n%4===1)sum[1]+=term;else if(n%4===2)sum[0]-=term;else sum[1]-=term
 }
 return sum
}
it('matches independent Fresnel power series and keeps both error contributions',()=>{
 for(const sign of [-1,1]){
  const a=approximateClothoidNurbsCurve([3,4,5],2,0,sign*2,0,1e-5)
  expect(a.report.realArithmeticErrorEstimate).toBe(a.report.quadratureErrorEstimate+a.report.hermiteErrorEstimate)
  expect(a.report.realArithmeticErrorEstimate).toBeLessThanOrEqual(a.report.budget)
  expect(a.report.quadratureIntervals).toBeGreaterThanOrEqual(2)
  for(let i=0;i<=200;i++){
   const t=i/200,exact=fresnel(2*t),p=evaluateNurbsCurve(a.curve,t).point
   expect(Math.hypot(p[0]!-3-exact[0],p[1]!-4-sign*exact[1])).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-11)
  }
 }
 expect(()=>approximateClothoidNurbsCurve([0,0,0],2,0,100,0,1e-14)).toThrow()
})
it('checks inverse-length curvature and angular/length dimensions through Rush',()=>{
 const source=readFileSync('examples/rush/clothoid-extrusion.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='clothoid_curve')).toMatchObject({length:2,start_curvature:0,end_curvature:2,phase_degrees:0})
 for(const field of ['start_curvature: 0 / 1mm','end_curvature: 2 / 1mm'])expect(()=>compileModelGraphText(source.replace(field,field.replace('/ 1mm','* 1mm')))).toThrow()
 expect(()=>compileModelGraphText(source.replace('length: 2mm','length: 2deg'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('phase_degrees: 0deg','phase_degrees: 1mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace(',end_curvature: 2 / 1mm',''))).toThrow()
})
it('preserves quadrature and Hermite reports through multipatch extrusion',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/clothoid-extrusion.r','utf8'))
 const built=buildOwnNurbs(graph.document,{action:'build'})
 const curve=graph.document.nodes.find(n=>n.op==='clothoid_curve')!
 const report=built.report.construction?.[curve.id] as {quadratureErrorEstimate:number;hermiteErrorEstimate:number;realArithmeticErrorEstimate:number}
 expect(built.report.error_bound_certified).toBe(false)
 expect(report).toMatchObject({budget:1e-4,continuousBound:false,roundingCertified:false,method:'clothoid-Hermite-composite-Simpson'})
 expect(report.realArithmeticErrorEstimate).toBe(report.quadratureErrorEstimate+report.hermiteErrorEstimate)
})
