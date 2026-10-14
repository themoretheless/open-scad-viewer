import {expect,it} from 'vitest'
import {bezierNurbsCurve,scaledSweepNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'

it('preserves independent rational radius and trajectory laws through WASM',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[1,1,0],[0,1,0]],[1,Math.SQRT1_2,1])
 const path=bezierNurbsCurve([[0,0,0],[0,0,2]],[1,2])
 const scale={degree:1,knots:[0,0,1,1],values:[1,3],weights:[2,1]}
 const surface=scaledSweepNurbsCurve(profile,path,scale,[0,0,0])
 for(const u of [0,.17,.5,.83,1])for(const v of [0,.13,.5,.87,1]){
  const p=evaluateNurbsSurface(surface,u,v).point
  const radius=(2+v)/(2-v)
  expect(p[0]!**2+p[1]!**2).toBeCloseTo(radius**2,9)
  expect(p[2]).toBeCloseTo(4*v/(1+v),9)
 }
 expect(()=>scaledSweepNurbsCurve(profile,path,{...scale,values:[1,0]},[0,0,0])).toThrow()
})

it('scales relative to the explicit center and anchors a distant path start',()=>{
 const profile=bezierNurbsCurve([[11,20,30],[12,20,30]])
 const path=bezierNurbsCurve([[1e9,0,10],[1e9,0,12]])
 const surface=scaledSweepNurbsCurve(profile,path,{degree:1,knots:[0,0,1,1],values:[1,3],weights:[1,1]},[10,20,30])
 for(const u of [0,.5,1])for(const v of [0,.25,1]){
  const p=evaluateNurbsSurface(surface,u,v).point
  expect(p[0]).toBeCloseTo(10+(1+u)*(1+2*v),9)
  expect(p[1]).toBeCloseTo(20,9)
  expect(p[2]).toBeCloseTo(30+2*v,9)
 }
})

it('normalizes a nonunit multispan scale domain independently of the path',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path=bezierNurbsCurve([[0,0,0],[0,0,4]])
 const surface=scaledSweepNurbsCurve(profile,path,{degree:1,knots:[2,2,4,6,6],values:[1,3,2],weights:[1,1,1]},[0,0,0])
 for(const u of [0,.3,1])for(const v of [0,.13,.49,.5,.51,.87,1]){
  const p=evaluateNurbsSurface(surface,u,v).point
  const radius=v<=.5?1+4*v:4-2*v
  expect(p[0]).toBeCloseTo((1+u)*radius,9)
  expect(p[1]).toBeCloseTo(0,9)
  expect(p[2]).toBeCloseTo(4*v,9)
 }
})

it('returns analytic first and mixed derivatives of the polynomial product',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path=bezierNurbsCurve([[0,0,0],[0,0,1],[0,0,4]])
 const surface=scaledSweepNurbsCurve(profile,path,{degree:1,knots:[0,0,1,1],values:[1,3],weights:[1,1]},[0,0,0])
 for(const u of [.13,.5,.87])for(const v of [.17,.5,.83]){
  const q=evaluateNurbsSurface(surface,u,v)
  expect(q.du![0]).toBeCloseTo(1+2*v,9)
  expect(q.dv![0]).toBeCloseTo(2*(1+u),9)
  expect(q.dv![2]).toBeCloseTo(2+4*v,9)
  expect(q.duv![0]).toBeCloseTo(2,9)
  expect(q.dvv![2]).toBeCloseTo(4,9)
 }
})
