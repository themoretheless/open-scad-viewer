import {expect,it} from 'vitest'
import {scaledSweepNurbsCurve,checkedProfileSweepNurbsSurface,bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
const path=bezierNurbsCurve([[0,0,0],[0,0,5]])
const scale={degree:1,knots:[10,10,14,14],values:[1,2],weights:[3,1]}
it('delivers independently weighted scale/path geometry and jets through WASM',()=>{
 const weighted:NurbsCurve={...path,weights:[1,2],knots:[2,2,7,7]}
 const s=scaledSweepNurbsCurve(profile,weighted,scale,[.5,0,0])
 for(const v of [0,.2,.5,.8,1]){
  const c=evaluateNurbsCurve(weighted,2+5*v),r=(3*(1-v)+2*v)/(3*(1-v)+v)
  const p=evaluateNurbsSurface(s,.3,v)
  expect(p.point[0]).toBeCloseTo(.5+r*.8,10);expect(p.point[2]).toBeCloseTo(c.point[2]!,10)
  expect(p.du![0]).toBeCloseTo(r,10);expect(p.dv![2]).toBeCloseTo(5*c.d1![2]!,10)
 }
})
it('promotes accepted RMF surfaces and retains sampled refusal without geometry',()=>{
 const law={...scale,weights:[1,1]}
 const good=checkedProfileSweepNurbsSurface(profile,path,law,[1,0,0],5,.001)
 expect(good.report).toMatchObject({accepted:true,continuousBound:false,stations:17,sections:5})
 expect(evaluateNurbsSurface(good.surface!,.5,1).point).toEqual([3,0,5])
 const arc:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1]}
 const bad=checkedProfileSweepNurbsSurface(profile,arc,law,[1,0,0],3,1e-6)
 expect(bad.surface).toBeNull();expect(bad.report).toMatchObject({accepted:false,continuousBound:false})
 expect(()=>checkedProfileSweepNurbsSurface(profile,path,law,[0,0,1],5,.1)).toThrow(/normal/)
 expect(()=>scaledSweepNurbsCurve(profile,path,{...law,values:[0,1]},[0,0,0])).toThrow(/positive/)
})
it('exposes both constructors through the existing Rush path',()=>{
 for(const op of ['scaled_sweep','profile_sweep']){
  const source=`// @rush/1\np = circle_curve(center:[0,0,0],normal:[0,0,1],radius:1mm)\nc = bezier_curve(points:[[0,0,0],[0,0,5mm]])\nshow ${op}(p,c,scale:{degree:1,knots:[0,0,1,1],values:[1,2],weights:[1,1]},${op==='scaled_sweep'?'origin:[0,0,0]':'normal:[1,0,0],sections:5,max_deviation:0.01mm'}).tessellate(segments_u:4,segments_v:4)`
  const result=buildOwnNurbs(compileModelGraphText(source).document,{action:'build'})
  expect(result.mesh!.indices.length).toBeGreaterThan(0)
 }
})
it('returns explicit C0 closed RMF seams and refuses incompatible endpoint scales',()=>{
 const circle=circleNurbsCurve([0,0,0],[0,0,1],1),law={...scale,values:[1,1],weights:[1,1]}
 const closed=checkedProfileSweepNurbsSurface(profile,circle,law,[1,0,0],17,1)
 expect(closed.report).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',continuousBound:false})
 expect(closed.surface!.periodicV).toBe(true)
 expect(()=>checkedProfileSweepNurbsSurface(profile,circle,{...law,values:[1,2]},[1,0,0],17,1)).toThrow(/scale/)
})
