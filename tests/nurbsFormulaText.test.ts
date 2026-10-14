import {expect,it} from 'vitest'
import {formulaNurbsCurve,formulaNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {readFileSync} from 'node:fs'

it('preserves infix precedence and ordinary polynomial coordinate formulas',()=>{
 const curve=formulaNurbsCurve([-2,3],['t','-t^2+2','3*(t+1)'])
 for(const u of [0,.17,.5,.89,1]){
  const t=-2+5*u,p=evaluateNurbsCurve(curve,u).point
  expect(p[0]).toBeCloseTo(t,11);expect(p[1]).toBeCloseTo(-t*t+2,11);expect(p[2]).toBeCloseTo(3*(t+1),11)
 }
 for(const source of ['sin(t)','u','t^0','t^13','t^1.5','t^2^3','t**2','2t','t;0','1e999'])expect(()=>formulaNurbsCurve([0,1],[source,'0','0'])).toThrow()
 expect(()=>formulaNurbsCurve([0,1],['('.repeat(30)+'t'+')'.repeat(30),'0','0'])).toThrow()
})

it('converts rational two-variable text and mixed coordinate token lists without sampled fitting',()=>{
 const s=formulaNurbsSurface([0,1,0,1],['u/(1+u*v)','v*(1+u*v)^-1',['u','v','*']])
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.19,.5,.91,1]){
  const p=evaluateNurbsSurface(s,u,v).point
  expect(p[0]).toBeCloseTo(u/(1+u*v),11);expect(p[1]).toBeCloseTo(v/(1+u*v),11);expect(p[2]).toBeCloseTo(u*v,11)
 }
 expect(()=>formulaNurbsSurface([-1,1,0,1],['1/u','v','0'])).toThrow()
})

it('preserves authored infix strings in canonical Rush graph definitions',()=>{
 for(const name of ['formula-curve-text','formula-surface-text']){
  const graph=compileRushFrontend(readFileSync(`examples/rush/${name}.r`,'utf8'))
  const node=graph.document.nodes.find(n=>n.op==='formula_curve'||n.op==='formula_surface')
  if(node?.op!=='formula_curve'&&node?.op!=='formula_surface')throw new Error('Missing formula node')
  expect(node.expressions.every(e=>typeof e==='string')).toBe(true)
 }
})
