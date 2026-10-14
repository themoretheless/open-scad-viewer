import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateEllipticHelixNurbsCurve,approximateConicalHelixNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'

it('matches independent elliptical helix samples within the ideal interpolation estimate',()=>{
 const a=approximateEllipticHelixNurbsCurve([3,4,5],2,4,-7,-1.5,37,1e-4)
 for(let i=0;i<=200;i++){
  const t=i/200,angle=37*Math.PI/180-3*Math.PI*t,p=evaluateNurbsCurve(a.curve,t).point
  const exact=[3+2*Math.cos(angle),4+4*Math.sin(angle),5-7*t]
  expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
 }
 expect(()=>approximateEllipticHelixNurbsCurve([0,0,0],0,2,1,1,0,1e-3)).toThrow()
})
it('matches independent growing and apex-ending conical helix samples',()=>{
 for(const [start,end] of [[1,5],[5,0]]){
  const a=approximateConicalHelixNurbsCurve([3,4,5],start!,end!,-9,1,23,1e-4)
  expect(a.report.realArithmeticErrorEstimate).toBeLessThanOrEqual(a.report.budget)
  for(let i=0;i<=200;i++){
   const t=i/200,angle=23*Math.PI/180+2*Math.PI*t,radius=start!+(end!-start!)*t,p=evaluateNurbsCurve(a.curve,t).point
   const exact=[3+radius*Math.cos(angle),4+radius*Math.sin(angle),5-9*t]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 expect(()=>approximateConicalHelixNurbsCurve([0,0,0],0,0,1,1,0,1e-3)).toThrow()
})
for(const variant of ['elliptic','conical']){
 it(`checks dimensions of ${variant} helix through packaged Rush`,()=>{
  const source=readFileSync(`examples/rush/${variant}-helix-extrusion.r`,'utf8'),graph=compileRushFrontend(source)
  expect(graph.document.nodes.find(n=>n.op===`${variant}_helix_curve`)).toMatchObject({turns:.25,phase_degrees:0,max_deviation:.0001})
  expect(()=>compileRushFrontend(source.replace('turns: 0.25','turns: 0.25mm'))).toThrow()
  expect(()=>compileRushFrontend(source.replace('phase_degrees: 0deg','phase_degrees: 0mm'))).toThrow()
  const radii=variant==='elliptic'?'radius_y: 3mm':'end_radius: 2mm'
  expect(()=>compileRushFrontend(source.replace(radii,radii.replace('mm','deg')))).toThrow()
 })
 it(`preserves ${variant} approximation status when the root curve has a foundation certificate`,()=>{
  const graph=compileRushFrontend(readFileSync(`examples/rush/${variant}-helix-extrusion.r`,'utf8'))
  const curve=graph.document.nodes.find(n=>n.op===`${variant}_helix_curve`)!
  const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build'})
  expect(built.report.foundation_certificate).not.toBeNull()
  expect(built.report.error_bound_certified).toBe(false)
  expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
 })
}
