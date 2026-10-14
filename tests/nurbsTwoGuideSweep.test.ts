import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {circleNurbsCurve,bezierNurbsCurve,twoGuideSweepNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('preserves rational profile and independently parameterized guides through WASM',()=>{
 const profile=bezierNurbsCurve([[0,0,0],[2,1,1]],[1,3])
 const a=bezierNurbsCurve([[0,0,0],[0,0,4]])
 const b={...bezierNurbsCurve([[2,0,0],[4,0,6]],[1,2]),knots:[2,2,6,6]}
 const s=twoGuideSweepNurbsCurve(profile,a,b,2,[0,2,0],[0,0,3])
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point,alpha=3*u/(1+2*u)
  expect(p[0]).toBeCloseTo(alpha*(2+6*v)/(1+v),9)
  expect(p[1]).toBeCloseTo(2*alpha,9)
  expect(p[2]).toBeCloseTo((1-alpha)*4*v+alpha*12*v/(1+v)+3*alpha,9)
 }
 expect(()=>twoGuideSweepNurbsCurve(profile,a,b,0,[0,2,0],[0,0,3])).toThrow()
 expect(()=>twoGuideSweepNurbsCurve(profile,a,b,2,[0,2,0],[0,3,0])).toThrow()
})
it('uses length for width and dimensionless transverse axes in packaged Rush',()=>{
 const source=readFileSync('examples/rush/two-guide-sweep.r','utf8')
 const graph=compileRushFrontend(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='two_guide_sweep')).toMatchObject({width:10,axis_y:[0,1,0],axis_z:[0,0,1]})
 expect(compileRushFrontend(source.replace('width: 10mm','width: 1cm')).document).toEqual(graph.document)
 expect(()=>compileRushFrontend(source.replace('width: 10mm','width: 10deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('axis_y: [0,1,0]','axis_y: [0,1mm,0]'))).toThrow()
})
it('returns independent first and mixed derivatives of the affine guide product',()=>{
 const profile=bezierNurbsCurve([[0,0,0],[2,1,0]])
 const a=bezierNurbsCurve([[0,0,0],[0,0,4]])
 const b=bezierNurbsCurve([[2,0,0],[4,0,6]])
 const s=twoGuideSweepNurbsCurve(profile,a,b,2,[0,1,0],[0,0,1])
 for(const u of [.13,.5,.87])for(const v of [.17,.5,.83]){
  const q=evaluateNurbsSurface(s,u,v)
  const du=[2+2*v,1,2*v],dv=[2*u,0,4+2*u],duv=[2,0,2]
  for(let d=0;d<3;d++){
   expect(q.du![d]).toBeCloseTo(du[d]!,9)
   expect(q.dv![d]).toBeCloseTo(dv[d]!,9)
   expect(q.duv![d]).toBeCloseTo(duv[d]!,9)
  }
 }
})
it('preserves both closed seams and the independent torus equation through WASM',()=>{
 const profile=circleNurbsCurve([1,0,0],[0,0,1],1)
 const a=circleNurbsCurve([0,0,0],[0,0,1],10),b=circleNurbsCurve([0,0,0],[0,0,1],12)
 const s=twoGuideSweepNurbsCurve(profile,a,b,2,[0,0,1],[0,1,0])
 for(const u of [0,.13,.37,.63,.87,1])for(const v of [0,.17,.43,.57,.83,1]){
  const q=evaluateNurbsSurface(s,u,v).point
  expect((Math.hypot(q[0],q[1])-11)**2+q[2]**2).toBeCloseTo(1,9)
 }
 for(const t of [0,.13,.37,.63,.87,1]){
  const a=evaluateNurbsSurface(s,t,0).point,b=evaluateNurbsSurface(s,t,1).point
  const c=evaluateNurbsSurface(s,0,t).point,d=evaluateNurbsSurface(s,1,t).point
  for(let axis=0;axis<3;axis++){
   expect(a[axis]).toBeCloseTo(b[axis]!,9)
   expect(c[axis]).toBeCloseTo(d[axis]!,9)
  }
 }
 expect(s.periodicV).toBe(false)
})
