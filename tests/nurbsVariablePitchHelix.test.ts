import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateVariablePitchHelixNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent variable-pitch axial law for both revolution signs',()=>{
 for(const turns of [.75,-.75]){
  const a=approximateVariablePitchHelixNurbsCurve([3,4,5],2,7,turns,2,11,23,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,angle=23*Math.PI/180+2*Math.PI*turns*t
   const z=(t*t*t-2*t*t+t)*turns*2+(-2*t*t*t+3*t*t)*7+(t*t*t-t*t)*turns*11
   const p=evaluateNurbsCurve(a.curve,t).point,exact=[3+2*Math.cos(angle),4+2*Math.sin(angle),5+z]
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
  expect(evaluateNurbsCurve(a.curve,0).d1![2]).toBeCloseTo(turns*2,10)
  expect(evaluateNurbsCurve(a.curve,1).d1![2]).toBeCloseTo(turns*11,10)
  expect(a.report).toMatchObject({continuousBound:false,roundingCertified:false})
 }
})
it('checks endpoint pitch length dimensions through packaged Rush',()=>{
 const source=readFileSync('examples/rush/variable-pitch-helix-extrusion.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='variable_pitch_helix_curve')).toMatchObject({start_pitch:24,end_pitch:72,turns:.25})
 for(const key of ['start_pitch: 24mm','end_pitch: 72mm'])expect(()=>compileRushFrontend(source.replace(key,key.replace('mm','deg')))).toThrow()
 expect(()=>compileRushFrontend(source.replace('turns: 0.25','turns: 0.25mm'))).toThrow()
})
it('keeps the approximation uncertified despite a root curve foundation certificate',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/variable-pitch-helix-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='variable_pitch_helix_curve')!
 const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build'})
 expect(built.report.foundation_certificate).not.toBeNull()
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
})
