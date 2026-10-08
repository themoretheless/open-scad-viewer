import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateHelixNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'

it('keeps sampled helix errors under the reported ideal interpolation estimate through WASM',()=>{
 for(const turns of [1,-2.5]){
  const a=approximateHelixNurbsCurve([3,4,5],2,-7,turns,37,1e-4)
  expect(a.report.realArithmeticErrorEstimate).toBeLessThanOrEqual(a.report.budget)
  expect(a.report.continuousBound).toBe(false);expect(a.report.roundingCertified).toBe(false)
  for(let i=0;i<=200;i++){
   const t=i/200,angle=37*Math.PI/180+2*Math.PI*turns*t,p=evaluateNurbsCurve(a.curve,t).point
   const exact=[3+2*Math.cos(angle),4+2*Math.sin(angle),5-7*t]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 const coarse=approximateHelixNurbsCurve([0,0,0],2,10,1,0,1e-2),fine=approximateHelixNurbsCurve([0,0,0],2,10,1,0,1e-5)
 expect(fine.report.spans).toBeGreaterThan(coarse.report.spans)
 expect(()=>approximateHelixNurbsCurve([0,0,0],2,10,100,0,1e-8)).toThrow()
})
it('checks dimensions through packaged Rush',()=>{
 const source=readFileSync('examples/rush/helix-extrusion.r','utf8'),graph=compileRushFrontend(source)
 const helix=graph.document.nodes.find(n=>n.op==='helix_curve')!
 expect(helix).toMatchObject({radius:5,height:12,turns:.25,phase_degrees:0,max_deviation:.0001})
 expect(()=>compileRushFrontend(source.replace('turns: 0.25','turns: 0.25mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('phase_degrees: 0deg','phase_degrees: 0mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('max_deviation: 0.0001mm','max_deviation: 0.0001deg'))).toThrow()
})
it('retains the approximation report throughout graph construction',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/helix-extrusion.r','utf8'))
 const helix=graph.document.nodes.find(n=>n.op==='helix_curve')!
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[helix.id]).toMatchObject({budget:.0001,continuousBound:false,roundingCertified:false})
 const curveOnly={...graph.document,nodes:[helix],root:helix.id}
 const curveResult=buildOwnNurbs(curveOnly,{action:'build'})
 expect(curveResult.report.foundation_certificate).not.toBeNull()
 expect(curveResult.report.error_bound_certified).toBe(false)
 expect(curveResult.report.construction?.[helix.id]).toMatchObject({continuousBound:false,roundingCertified:false})
})
