import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateSphericalSpiralNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent spherical coordinates and radial equation across poles',()=>{
 for(const [p,q,phase] of [[1,.5,-90],[-.75,.25,23],[.5,-.75,47]]){
  const a=approximateSphericalSpiralNurbsCurve([3,4,5],2,p!,q!,31,phase!,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,theta=31*Math.PI/180+2*Math.PI*p!*t,phi=phase!*Math.PI/180+2*Math.PI*q!*t
   const point=evaluateNurbsCurve(a.curve,t).point
   const exact=[3+2*Math.cos(phi)*Math.cos(theta),4+2*Math.cos(phi)*Math.sin(theta),5+2*Math.sin(phi)]
   expect(Math.hypot(...point.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
   expect(Math.abs(Math.hypot(point[0]!-3,point[1]!-4,point[2]!-5)-2)).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 expect(()=>approximateSphericalSpiralNurbsCurve([0,0,0],2,10,5,0,0,1e-12)).toThrow()
})
it('checks lengths, both angular phases and signed dimensionless turns in Rush',()=>{
 const source=readFileSync('examples/rush/spherical-spiral-extrusion.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='spherical_spiral_curve')).toMatchObject({radius:2,longitude_turns:1,latitude_turns:.5,longitude_phase_degrees:31,latitude_phase_degrees:-90})
 expect(()=>compileRushFrontend(source.replace('radius: 2mm','radius: 2deg'))).toThrow()
 for(const field of ['longitude_phase_degrees: 31deg','latitude_phase_degrees: -90deg'])expect(()=>compileRushFrontend(source.replace(field,field.replace('deg','mm')))).toThrow()
 expect(()=>compileRushFrontend(source.replace('longitude_turns: 1','longitude_turns: 1mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',latitude_turns: 0.5',''))).toThrow()
})
it('retains the tight approximation report through multipatch extrusion',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/spherical-spiral-extrusion.r','utf8'))
 const built=buildOwnNurbs(graph.document,{action:'build'})
 const curve=graph.document.nodes.find(n=>n.op==='spherical_spiral_curve')!
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({budget:1e-4,continuousBound:false,roundingCertified:false})
})
