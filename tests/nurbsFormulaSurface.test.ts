import {expect,it} from 'vitest'
import {formulaNurbsSurface,type NurbsSurfaceFormulaToken} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {readFileSync} from 'node:fs'

it('preserves two-variable polynomial points and jets after domain normalization',()=>{
 const s=formulaNurbsSurface([-1,1,-2,3],[[10,'u','*'],[7,'v','*'],[4,'u','u','*','u','*',3,'u','*','v','*','v','*','-','*']])
 for(const U of [0,.17,.5,.89,1])for(const V of [0,.13,.5,.91,1]){
  const u=-1+2*U,v=-2+5*V,q=evaluateNurbsSurface(s,U,V)
  expect(q.point[0]).toBeCloseTo(10*u,10);expect(q.point[1]).toBeCloseTo(7*v,10);expect(q.point[2]).toBeCloseTo(4*(u**3-3*u*v*v),10)
  expect(q.du![0]).toBeCloseTo(20,10);expect(q.dv![1]).toBeCloseTo(35,10)
  expect(q.du![2]).toBeCloseTo(24*(u*u-v*v),10);expect(q.dv![2]).toBeCloseTo(-120*u*v,10)
 }
})

it('combines independent rational coordinate denominators and refuses singular charts',()=>{
 const expressions:[NurbsSurfaceFormulaToken[],NurbsSurfaceFormulaToken[],NurbsSurfaceFormulaToken[]]=[['u',1,'u','+','/'],['v',1,'v','+','/'],['u','v','*',1,'u','v','*','+','/']]
 const s=formulaNurbsSurface([0,2,0,3],expressions)
 for(const U of [0,.19,.5,.87,1])for(const V of [0,.21,.5,.93,1]){
  const u=2*U,v=3*V,p=evaluateNurbsSurface(s,U,V).point
  expect(p[0]).toBeCloseTo(u/(1+u),11);expect(p[1]).toBeCloseTo(v/(1+v),11);expect(p[2]).toBeCloseTo(u*v/(1+u*v),11)
 }
 expect(()=>formulaNurbsSurface([-1,1,0,1],[[1,'u','/'],['v'],[0]])).toThrow()
 expect(()=>formulaNurbsSurface([0,1,0,1],[['+'],['v'],[0]])).toThrow()
})

it('lowers two-variable formula surfaces and resolves scalar formula parameters in Rush',()=>{
 for(const name of ['formula-surface','rational-formula-surface']){
  const source=readFileSync(`examples/rush/${name}.r`,'utf8'),graph=compileModelGraphText(source)
  expect(graph.execution_target).toBe('own-nurbs')
  expect(graph.document.nodes.some(n=>n.op==='formula_surface')).toBe(true)
  expect(()=>compileModelGraphText(source.replace('"u"','"t"'))).toThrow()
 }
 const source=readFileSync('examples/rush/formula-surface.r','utf8')
 const graph=compileModelGraphText('param scale = 8 range 1..20\n'+source.replace('[10,"u","*"]','[scale,"u","*"]'))
 const node=graph.document.nodes.find(n=>n.op==='formula_surface')
 if(node?.op!=='formula_surface')throw new Error('Missing formula surface')
 expect(node.expressions[0]).toEqual([8,'u','*'])
 expect(()=>compileModelGraphText(source.replace('domain: [-1,1,-1,1]','domain: [-1mm,1mm,-1mm,1mm]'))).toThrow()
})
