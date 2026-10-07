import {expect,it} from 'vitest'
import {formulaNurbsCurve,type NurbsFormulaToken} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {readFileSync} from 'node:fs'

it('evaluates polynomial coordinate formulas and derivatives after normalized domain conversion',()=>{
 const curve=formulaNurbsCurve([-2,3],[['t'],['t','t','*',2,'+'],['t',3,'*','neg']])
 for(const u of [0,.13,.5,.87,1]){
  const t=-2+5*u,q=evaluateNurbsCurve(curve,u)
  expect(q.point[0]).toBeCloseTo(t,11);expect(q.point[1]).toBeCloseTo(t*t+2,11);expect(q.point[2]).toBeCloseTo(-3*t,11)
  expect(q.d1![0]).toBeCloseTo(5,11);expect(q.d1![1]).toBeCloseTo(10*t,11);expect(q.d1![2]).toBeCloseTo(-15,11)
  expect(q.d2![1]).toBeCloseTo(50,11)
 }
})

it('retains a rational circle chart and rejects singular denominator intervals',()=>{
 const expressions:[NurbsFormulaToken[],NurbsFormulaToken[],NurbsFormulaToken[]]=[[1,'t','t','*','-',1,'t','t','*','+','/'],[2,'t','*',1,'t','t','*','+','/'],[0]]
 const curve=formulaNurbsCurve([0,1],expressions)
 for(const t of [0,.17,.49,.83,1]){
  const p=evaluateNurbsCurve(curve,t).point
  expect(p[0]).toBeCloseTo((1-t*t)/(1+t*t),11);expect(p[1]).toBeCloseTo(2*t/(1+t*t),11)
  expect(p[0]!**2+p[1]!**2).toBeCloseTo(1,11)
 }
 expect(()=>formulaNurbsCurve([-1,1],[[1,'t','/'],[0],[0]])).toThrow()
 expect(()=>formulaNurbsCurve([0,1],[['+'],[0],[0]])).toThrow()
 expect(()=>formulaNurbsCurve([0,1],[[1,0,'/'],[0],[0]])).toThrow()
})

it('lowers bounded formula tokens without treating operator strings as dimensional values',()=>{
 for(const name of ['formula-curve-extrusion','rational-formula-curve']){
  const source=readFileSync(`examples/rush/${name}.r`,'utf8'),compiled=compileRushFrontend(source)
  expect(compiled.execution_target).toBe('own-nurbs')
  expect(compiled.document.nodes.some(n=>n.op==='formula_curve')).toBe(true)
 }
 const source=readFileSync('examples/rush/formula-curve-extrusion.r','utf8')
 expect(()=>compileRushFrontend(source.replace('domain: [-2,2]','domain: [-2mm,2mm]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('"t","t","*","t","*"','"t","sin"'))).toThrow()
})
