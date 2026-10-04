import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateLissajousNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
it('matches independent three-axis harmonic trajectories and derivatives',()=>{
 for(const frequencies of [[1,2,3],[-1,.5,0],[0,-2,1.5]]){
  const amplitudes:[number,number,number]=[2,3,4],phases:[number,number,number]=[23,47,91]
  const a=approximateLissajousNurbsCurve([3,4,5],amplitudes,frequencies as [number,number,number],phases,1e-4)
  for(let i=0;i<=200;i++){
   const t=i/200,p=evaluateNurbsCurve(a.curve,t).point
   const exact=[3,4,5].map((c,d)=>c+amplitudes[d]!*Math.sin(phases[d]!*Math.PI/180+2*Math.PI*frequencies[d]!*t))
   expect(Math.hypot(...p.map((x,d)=>x-exact[d]!))).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-12)
  }
  for(const t of [0,1]){
   const d1=evaluateNurbsCurve(a.curve,t).d1!
   frequencies.forEach((f,d)=>expect(d1[d]).toBeCloseTo(amplitudes[d]!*2*Math.PI*f*Math.cos(phases[d]!*Math.PI/180+2*Math.PI*f*t),9))
  }
 }
})
it('checks vector dimensions and required phases through packaged Rush',()=>{
 const source=readFileSync('examples/rush/lissajous-extrusion.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='lissajous_curve')).toMatchObject({frequencies:[.125,.25,.375],phases_degrees:[0,30,60]})
 expect(()=>compileModelGraphText(source.replace('frequencies: [0.125','frequencies: [0.125mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('30deg','30mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace(',phases_degrees: [0deg,30deg,60deg]',''))).toThrow()
})
it('keeps graph phases angular and its approximation uncertified',()=>{
 const graph=compileModelGraphText(readFileSync('examples/rush/lissajous-extrusion.r','utf8'))
 const curve=graph.document.nodes.find(n=>n.op==='lissajous_curve')!
 const built=buildOwnNurbs({...graph.document,nodes:[curve],root:curve.id},{action:'build',evaluations:[{node:curve.id,u:0}]})
 const p=built.evaluations[0]!.point
 expect(p[0]).toBeCloseTo(0,12);expect(p[1]).toBeCloseTo(1.5,12);expect(p[2]).toBeCloseTo(Math.sqrt(3),12)
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[curve.id]).toMatchObject({continuousBound:false,roundingCertified:false,budget:.0001})
})
