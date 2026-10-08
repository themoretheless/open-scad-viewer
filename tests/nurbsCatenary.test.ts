import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateCatenaryNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent cosh geometry and preserves near-vertex height',()=>{
 const a=approximateCatenaryNurbsCurve([3,4,5],2,-3,4,1e-4)
 for(let i=0;i<=200;i++){
  const t=i/200,x=-3+7*t,p=evaluateNurbsCurve(a.curve,t).point,exact=[3+x,4+2*(Math.cosh(x/2)-1),5]
  expect(Math.hypot(...p.map((v,d)=>v-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
 }
 const tiny=approximateCatenaryNurbsCurve([0,0,0],1,1e-9,2e-9,1e-30)
 expect(evaluateNurbsCurve(tiny.curve,0).point[1]).toBeGreaterThan(0)
 expect(a.report.method).toBe('uniform-abscissa-cubic-Hermite-fourth-derivative-estimate')
})
it('checks catenary scale and x-bound length dimensions through packaged Rush',()=>{
 const source=readFileSync('examples/rush/catenary-extrusion.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='catenary_curve')).toMatchObject({scale:5,start_x:-2,end_x:3})
 for(const field of ['scale: 5mm','end_x: 3mm'])expect(()=>compileRushFrontend(source.replace(field,field.replace('mm','deg')))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',scale: 5mm',''))).toThrow()
})
it('preserves vertex-relative endpoint and abscissa report in graph',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/catenary-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='catenary_curve')!
 const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:1}]})
 const p=built.evaluations[0]!.point
 expect(p[0]).toBeCloseTo(3,10);expect(p[1]).toBeCloseTo(5*(Math.cosh(.6)-1),10)
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,method:'uniform-abscissa-cubic-Hermite-fourth-derivative-estimate'})
})
