import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {extrudeNurbsCurvePatches,approximateToroidalSpiralNurbsCurve,approximateTorusKnotNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent toroidal coordinates for signed and fractional turns',()=>{
 for(const [p,q] of [[.5,1.25],[-.75,.5],[2,3]]){
  const a=approximateToroidalSpiralNurbsCurve([3,4,5],3,1,p!,q!,23,47,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,theta=23*Math.PI/180+2*Math.PI*p!*t,phi=47*Math.PI/180+2*Math.PI*q!*t
   const point=evaluateNurbsCurve(a.curve,t).point
   const exact=[3+(3+Math.cos(phi))*Math.cos(theta),4+(3+Math.cos(phi))*Math.sin(theta),5+Math.sin(phi)]
   expect(Math.hypot(...point.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
 }
 expect(()=>approximateToroidalSpiralNurbsCurve([0,0,0],3,1,2,3,0,0,1e-12)).toThrow()
})
it('closes a single torus knot and refuses links and fractional knot indices',()=>{
 const a=approximateTorusKnotNurbsCurve([0,0,0],3,1,2,3,0,0,1e-4)
 expect(a.curve.controlPoints.at(-1)).toEqual(a.curve.controlPoints[0])
 expect(a.curve.periodic).toBe(false)
 for(const [p,q] of [[2,4],[2.5,3],[1,3]])expect(()=>approximateTorusKnotNurbsCurve([0,0,0],3,1,p!,q!,0,0,1e-4)).toThrow()
})
it('checks toroidal lengths, angular phases and dimensionless turns in Rush',()=>{
 const source=readFileSync('examples/rush/toroidal-spiral-extrusion.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='toroidal_spiral_curve')).toMatchObject({major_turns:.125,minor_turns:.375,major_phase_degrees:23,minor_phase_degrees:47})
 for(const field of ['major_radius: 3mm','minor_radius: 1mm'])expect(()=>compileRushFrontend(source.replace(field,field.replace('mm','deg')))).toThrow()
 for(const field of ['major_phase_degrees: 23deg','minor_phase_degrees: 47deg'])expect(()=>compileRushFrontend(source.replace(field,field.replace('deg','mm')))).toThrow()
 expect(()=>compileRushFrontend(source.replace('major_turns: 0.125','major_turns: 0.125mm'))).toThrow()
 const knot=readFileSync('examples/rush/torus-knot-extrusion.r','utf8')
 expect(compileRushFrontend(knot).document.nodes.find(n=>n.op==='torus_knot_curve')).toMatchObject({p:2,q:3})
 expect(()=>compileRushFrontend(knot.replace('p: 2','p: 2mm'))).toThrow()
})
it('keeps explicit approximation reports for both graph constructors',()=>{
 for(const name of ['toroidal-spiral-extrusion','torus-knot-extrusion']){
  const graph=compileRushFrontend(readFileSync(`examples/rush/${name}.r`,'utf8'))
  const built=buildOwnNurbs(graph.document,{action:'build'})
  expect(built.report.error_bound_certified).toBe(false)
  const curve=graph.document.nodes.find(n=>['toroidal_spiral_curve','torus_knot_curve'].includes(n.op))!
  expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false})
 }
})
it('extrudes the tight full knot without refitting and retains every patch domain',()=>{
 const fit=approximateTorusKnotNurbsCurve([0,0,0],3,1,2,3,0,0,1e-4)
 const patches=extrudeNurbsCurvePatches(fit.curve,[0,0,1])
 expect(patches).toHaveLength(fit.report.spans)
 for(let i=0;i<patches.length;i++){
  const patch=patches[i]!,lo=i/patches.length,hi=(i+1)/patches.length
  expect(patch.knotsU[0]).toBe(lo)
  expect(patch.knotsU.at(-1)).toBe(hi)
  for(const t of [0,.17,.5,.83,1]){
   const u=lo+(hi-lo)*t,point=evaluateNurbsCurve(fit.curve,u).point
   for(const v of [0,.3,1]){
    const p=evaluateNurbsSurface(patch,u,v).point
    expect(Math.hypot(...p.map((x,d)=>x-point[d]!-(d===2?v:0)))).toBeLessThan(1e-11)
   }
  }
 }
 const graph=compileRushFrontend(readFileSync('examples/rush/torus-knot-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='torus_knot_curve')!
 expect(curve).toMatchObject({max_deviation:1e-4})
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({budget:1e-4,continuousBound:false,roundingCertified:false})
})
