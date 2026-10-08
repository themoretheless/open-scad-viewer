import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {clampedSplineNurbsCurve,hermiteNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,trimNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('reproduces a cubic and its first two derivatives with nonuniform authored parameters',()=>{
 const t=[-2,-.7,.3,2],points=t.map(x=>[x,x*x,x**3] as [number,number,number])
 const c=clampedSplineNurbsCurve(points,t,[1,-4,12],[1,4,12])
 for(let i=0;i<3;i++){
  const a=(t[i]!+2)/4,b=(t[i+1]!+2)/4,segment=trimNurbsCurve(c,a,b)
  for(const s of [0,.13,.5,.87,1]){
   const u=a+s*(b-a),x=-2+4*u,q=evaluateNurbsCurve(segment,u)
   for(let k=0;k<3;k++){
    expect(q.point[k]).toBeCloseTo([x,x*x,x**3][k]!,9)
    expect(q.d1![k]).toBeCloseTo([4,8*x,12*x*x][k]!,8)
    expect(q.d2![k]).toBeCloseTo([0,32,96*x][k]!,7)
   }
  }
 }
})

it('agrees with two-site Hermite and refuses unrepresentable endpoint conditions',()=>{
 const p:[number,number,number][]=[[0,1,2],[3,-1,0]],t=[-2,3],start:[number,number,number]=[2,0,1],end:[number,number,number]=[-1,3,0]
 expect(clampedSplineNurbsCurve(p,t,start,end).control_points).toEqual(hermiteNurbsCurve(p,[start,end],t).control_points)
 expect(()=>clampedSplineNurbsCurve(p,t,[NaN,0,0],end)).toThrow()
 expect(()=>clampedSplineNurbsCurve(p,[0,1e9],[1e300,0,0],end)).toThrow()
 expect(()=>clampedSplineNurbsCurve([[0,0,0],[0,0,0]],[0,1e-300],[1e-300,0,0],[0,0,0])).toThrow()
 expect(()=>clampedSplineNurbsCurve([[1e9,0,0],[1e9+1,0,0],[1e9+2,0,0]],[0,1e-6,1],[1e-5,0,0],[0,0,0])).toThrow(/Hermite tangent collapses/)
})

it('lowers endpoint tangents as lengths and preserves dimensionless spline parameters in Rush',()=>{
 const source=readFileSync('examples/rush/clamped-spline-extrusion.r','utf8')
 const graph=compileRushFrontend(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='clamped_spline_curve')).toMatchObject({parameters:[-2,0,4],start_tangent:[6,0,0],end_tangent:[3,-2,0]})
 expect(()=>compileRushFrontend(source.replace('6mm','6deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('parameters: [-2,0,4]','parameters: [-2mm,0mm,4mm]'))).toThrow()
})
