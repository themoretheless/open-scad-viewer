import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateArchimedeanSpiralNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
it('matches independent linear-radius spiral for growth contraction and origin start',()=>{
 for(const [start,end] of [[1,5],[5,0],[0,5]]){
  const a=approximateArchimedeanSpiralNurbsCurve([3,4,5],start!,end!,23,743,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,theta=(23+720*t)*Math.PI/180,r=start!+(end!-start!)*t,p=evaluateNurbsCurve(a.curve,t).point
   const exact=[3+r*Math.cos(theta),4+r*Math.sin(theta),5]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 expect(()=>approximateArchimedeanSpiralNurbsCurve([0,0,0],1,2,30,0,1e-4)).toThrow()
})
it('checks radius and angle dimensions plus mandatory endpoints through packaged Rush',()=>{
 const source=readFileSync('examples/rush/archimedean-spiral-extrusion.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='archimedean_spiral_curve')).toMatchObject({start_radius:2,end_radius:5,end_degrees:60})
 expect(()=>compileModelGraphText(source.replace('end_radius: 5mm','end_radius: 5deg'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('end_degrees: 60deg','end_degrees: 60mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace(',end_radius: 5mm',''))).toThrow()
})
it('preserves graph angular endpoint and uncertified approximation',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/archimedean-spiral-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='archimedean_spiral_curve')!
 const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:1}]})
 const p=built.evaluations[0]!.point
 expect(p[0]).toBeCloseTo(2.5,10);expect(p[1]).toBeCloseTo(5*Math.sqrt(3)/2,10)
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
})
