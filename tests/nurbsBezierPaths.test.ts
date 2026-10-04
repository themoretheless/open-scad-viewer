import {expect,it} from 'vitest'
import {bezierNurbsCurve,composeNurbsCurves} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {readFileSync} from 'node:fs'

it('evaluates rational Bezier controls through the packaged geometry host',()=>{
  const points:[number,number,number][]=[[1,2,3],[-2,5,1],[4,-1,2],[8,3,-2]]
  const weights=[2,0.5,3,1]
  const curve=bezierNurbsCurve(points,weights)
  for(const t of [0,0.13,0.4,0.91,1]){
    const h=points.map((p,i)=>[...p.map(x=>x*weights[i]!),weights[i]!])
    for(let remaining=h.length-1;remaining>0;remaining--)for(let i=0;i<remaining;i++)for(let a=0;a<4;a++)h[i]![a]=(1-t)*h[i]![a]!+t*h[i+1]![a]!
    const p=evaluateNurbsCurve(curve,t).point
    for(let a=0;a<3;a++)expect(p[a]).toBeCloseTo(h[0]![a]!/h[0]![3]!,11)
  }
  expect(bezierNurbsCurve(points).weights).toEqual([1,1,1,1])
  expect(()=>bezierNurbsCurve(points,[1,1])).toThrow()
})

it('preserves rational pieces across composite seams and refuses disconnected paths',()=>{
  const a=bezierNurbsCurve([[0,0,0],[1,3,0],[2,1,0]],[2,1,4])
  const b=bezierNurbsCurve([[2,1,0],[3,-4,0],[5,2,0]],[0.5,3,1])
  b.knots=b.knots.map(k=>7+5*k)
  const c=composeNurbsCurves([a,b])
  for(const t of [0,0.1,0.43,0.9,1]){
    for(const [source,u,min,max] of [[a,t/2,0,1],[b,(1+t)/2,7,12]] as const){
      const p=evaluateNurbsCurve(source,min+t*(max-min)).point
      const q=evaluateNurbsCurve(c,u).point
      for(let i=0;i<3;i++)expect(q[i]).toBeCloseTo(p[i]!,11)
    }
  }
  b.controlPoints[0]![0]=2.001
  expect(()=>composeNurbsCurves([a,b])).toThrow()
  const mixed=composeNurbsCurves([a,bezierNurbsCurve([[2,1,0],[3,1,0]])])
  expect(mixed.degree).toBe(2)
  for(const t of [0,0.3,0.9,1]){
    const p=evaluateNurbsCurve(mixed,(1+t)/2).point
    expect(p[0]).toBeCloseTo(2+t,11)
    expect(p[1]).toBeCloseTo(1,11)
    expect(p[2]).toBeCloseTo(0,11)
  }
})

it('lowers Bezier coordinates and positional composite references through Rush',()=>{
  const source=readFileSync('examples/rush/composite-curve-extrusion.r','utf8')
  const c=compileModelGraphText(source)
  expect(c.execution_target).toBe('own-nurbs')
  expect(c.document.nodes.filter(n=>n.op==='bezier_curve')).toHaveLength(2)
  expect(c.document.nodes.find(n=>n.op==='curve_compose')?.inputs).toHaveLength(2)
  expect(()=>compileModelGraphText(source.replace('10mm','10deg'))).toThrow()
})
