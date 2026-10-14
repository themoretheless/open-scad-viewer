import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateLogarithmicSpiralNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent exponential radial law for growth contraction and zero',()=>{
 for(const growth of [-.5,0,.5]){
  const a=approximateLogarithmicSpiralNurbsCurve([3,4,5],2,growth,-1,2,1e-4)
  for(let i=0;i<=200;i++){
   const theta=-1+3*i/200,r=2*Math.exp(growth*(theta+1)),p=evaluateNurbsCurve(a.curve,i/200).point
   const exact=[3+r*Math.cos(theta),4+r*Math.sin(theta),5]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 expect(()=>approximateLogarithmicSpiralNurbsCurve([0,0,0],2,1000,0,2,1e-4)).toThrow()
})
it('checks dimensions and mandatory growth through packaged Rush',()=>{
 const source=readFileSync('examples/rush/logarithmic-spiral-extrusion.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='logarithmic_spiral_curve')).toMatchObject({growth:.3,end_degrees:60})
 expect(()=>compileRushFrontend(source.replace('growth: 0.3','growth: 0.3mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('end_degrees: 60deg','end_degrees: 60mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',growth: 0.3',''))).toThrow()
})
it('preserves growth per radian and uncertified approximation in the graph',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/logarithmic-spiral-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='logarithmic_spiral_curve')!
 const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:1}]})
 const theta=Math.PI/3,r=5*Math.exp(.3*theta),p=built.evaluations[0]!.point
 expect(p[0]).toBeCloseTo(r*Math.cos(theta),10)
 expect(p[1]).toBeCloseTo(r*Math.sin(theta),10)
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
})
