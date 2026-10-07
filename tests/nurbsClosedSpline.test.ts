import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {closedSplineNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,trimNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('matches the independent symmetric cubic loop and both seam derivatives',()=>{
 const p:[number,number,number][]=[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0]]
 const c=closedSplineNurbsCurve(p,[0,1,2,3,4]),first=trimNurbsCurve(c,0,.25),last=trimNurbsCurve(c,.75,1)
 for(const s of [0,.13,.5,.87,1]){
  const q=evaluateNurbsCurve(first,s/4),a=1-s
  expect(q.point[0]).toBeCloseTo(a**3+3*a*a*s+1.5*a*s*s,11)
  expect(q.point[1]).toBeCloseTo(1.5*a*a*s+3*a*s*s+s**3,11)
 }
 const a=evaluateNurbsCurve(first,0),b=evaluateNurbsCurve(last,1)
 expect(a.point).toEqual(b.point)
 for(let k=0;k<3;k++){
  expect(a.d1![k]).toBeCloseTo(b.d1![k]!,10)
  expect(a.d2![k]).toBeCloseTo(b.d2![k]!,9)
 }
 expect(c.periodic).toBe(false)
})

it('requires exact endpoint closure and valid increasing parameter contracts',()=>{
 const p:[number,number,number][]=[[0,0,0],[1,2,0],[3,0,1],[0,0,0]]
 expect(()=>closedSplineNurbsCurve([...p.slice(0,3),[Number.MIN_VALUE,0,0]],[0,1,2,3])).toThrow()
 expect(()=>closedSplineNurbsCurve(p,[0,1,1,3])).toThrow()
 expect(()=>closedSplineNurbsCurve(p,[0,1,2])).toThrow()
 expect(()=>closedSplineNurbsCurve(p.slice(0,3),[0,1,2])).toThrow()
})

it('lowers closed spline length sites and dimensionless parameters through Rush',()=>{
 const source=readFileSync('examples/rush/closed-spline-extrusion.r','utf8'),graph=compileRushFrontend(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='closed_spline_curve')).toMatchObject({parameters:[0,1,2,3,4]})
 expect(()=>compileRushFrontend(source.replace('10mm','10deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('parameters: [0,1,2,3,4]','parameters: [0mm,1mm,2mm,3mm,4mm]'))).toThrow()
})
