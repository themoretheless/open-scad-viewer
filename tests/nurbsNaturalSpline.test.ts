import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {naturalSplineNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,trimNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'

it('matches the independent natural cubic solution and one-sided first/second derivatives',()=>{
 const points:[number,number,number][]=[[0,0,0],[1,2,-1],[3,0,2]]
 const c=naturalSplineNurbsCurve(points,[-2,0,4]),u=[0,1/3,1],moments=[[0,0,0],[0,-27,22.5],[0,0,0]]
 for(let i=0;i<2;i++){
  const a=u[i]!,b=u[i+1]!,h=b-a,segment=trimNurbsCurve(c,a,b)
  for(const s of [0,.17,.5,.83,1]){
   const q=evaluateNurbsCurve(segment,a+s*h),l=1-s,r=s
   for(let k=0;k<3;k++)expect(q.point[k]).toBeCloseTo(l*points[i]![k]!+r*points[i+1]![k]!+h*h*((l**3-l)*moments[i]![k]!+(r**3-r)*moments[i+1]![k]!)/6,10)
  }
 }
 const left=trimNurbsCurve(c,0,1/3),right=trimNurbsCurve(c,1/3,1)
 const a=evaluateNurbsCurve(left,1/3),b=evaluateNurbsCurve(right,1/3)
 for(let k=0;k<3;k++){
  expect(a.d1![k]).toBeCloseTo(b.d1![k]!,9)
  expect(a.d2![k]).toBeCloseTo(b.d2![k]!,9)
  expect(evaluateNurbsCurve(left,0).d2![k]).toBeCloseTo(0,9)
  expect(evaluateNurbsCurve(right,1).d2![k]).toBeCloseTo(0,9)
 }
})

it('refuses invalid site contracts and preserves affine parameter changes',()=>{
 const p:[number,number,number][]=[[0,0,0],[1,2,0],[3,0,1]]
 expect(naturalSplineNurbsCurve(p,[0,1,3]).control_points).toEqual(naturalSplineNurbsCurve(p,[-2,0,4]).control_points)
 for(const t of [[0,0,1],[1,0,2],[0,1,Infinity],[0,1]])expect(()=>naturalSplineNurbsCurve(p,t)).toThrow()
 expect(()=>naturalSplineNurbsCurve([[0,0,0],[NaN,0,0]],[0,1])).toThrow()
})

it('lowers natural spline sites with length units and dimensionless parameters through Rush',()=>{
 const source=readFileSync('examples/rush/natural-spline-extrusion.r','utf8')
 const graph=compileModelGraphText(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='natural_spline_curve')).toMatchObject({points:[[-12,0,0],[0,8,0],[15,0,0]],parameters:[-2,0,4]})
 expect(()=>compileModelGraphText(source.replace('parameters: [-2,0,4]','parameters: [-2mm,0mm,4mm]'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('8mm','8deg'))).toThrow()
})
