import {expect,it} from 'vitest'
import {hermiteNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,trimNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {readFileSync} from 'node:fs'

it('interpolates authored positions and one-sided tangents on nonuniform parameter intervals',()=>{
 const points:[number,number,number][]=[[1,2,3],[4,-2,7],[8,5,0]],tangents:[number,number,number][]=[[2,1,0],[-1,3,2],[0,-2,1]],parameters=[-2,1,5]
 const c=hermiteNurbsCurve(points,tangents,parameters)
 for(let i=0;i<2;i++){
  const h=parameters[i+1]!-parameters[i]!,a=(parameters[i]!+2)/7,b=(parameters[i+1]!+2)/7,segment=trimNurbsCurve(c,a,b)
  for(const t of [0,.13,.5,.87,1]){
   const q=evaluateNurbsCurve(segment,a+(b-a)*t)
   const basis=[2*t**3-3*t*t+1,t**3-2*t*t+t,-2*t**3+3*t*t,t**3-t*t]
   const derivative=[6*t*t-6*t,3*t*t-4*t+1,-6*t*t+6*t,3*t*t-2*t]
   for(let k=0;k<3;k++){
    const values=[points[i]![k]!,h*tangents[i]![k]!,points[i+1]![k]!,h*tangents[i+1]![k]!]
    expect(q.point[k]).toBeCloseTo(basis.reduce((sum,x,j)=>sum+x*values[j]!,0),10)
    expect(q.d1![k]).toBeCloseTo(7/h*derivative.reduce((sum,x,j)=>sum+x*values[j]!,0),9)
   }
  }
 }
 expect(evaluateNurbsCurve(c,3/7).point).toEqual(points[1])
 expect(evaluateNurbsCurve(c,3/7).derivative_status).toBe('insufficient_continuity')
})

it('rejects invalid conditions and rounding collapse while retaining compensating scales',()=>{
 const points:[number,number,number][]=[[0,0,0],[1,0,0]],tangents:[number,number,number][]=[[1,0,0],[1,0,0]]
 for(const parameters of [[0,0],[1,0],[0,Infinity]])expect(()=>hermiteNurbsCurve(points,tangents,parameters)).toThrow()
 expect(()=>hermiteNurbsCurve(points,tangents.slice(0,1),[0,1])).toThrow()
 expect(()=>hermiteNurbsCurve([[1e9,0,0],[1e9,0,0]],[[1e-9,0,0],[1e-9,0,0]],[0,1])).toThrow()
 const c=hermiteNurbsCurve(points,[[1e300,0,0],[1e300,0,0]],[0,1e-300])
 for(const u of [0,.17,.5,.83,1]){
  const q=evaluateNurbsCurve(c,u)
  expect(q.point[0]).toBeCloseTo(u,11);expect(q.d1![0]).toBeCloseTo(1,11)
 }
})

it('keeps Hermite tangents as lengths and authored parameters dimensionless in Rush',()=>{
 const source=readFileSync('examples/rush/hermite-extrusion.r','utf8'),graph=compileModelGraphText(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='hermite_curve')).toMatchObject({parameters:[-2,1,5],tangents:[[4,0,0],[3,0,0],[2,-2,0]]})
 expect(()=>compileModelGraphText(source.replace('parameters: [-2,1,5]','parameters: [-2mm,1mm,5mm]'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('4mm','4deg'))).toThrow()
})
