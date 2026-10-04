import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,naturalLoftNurbsCurves} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'

it('lowers positional section references with dimensionless stations',()=>{
 const source=readFileSync('examples/rush/natural-loft-surface.r','utf8')
 const g=compileModelGraphText(source)
 expect(g.execution_target).toBe('own-nurbs')
 expect(g.document.nodes.find(n=>n.op==='natural_loft_surface')).toMatchObject({parameters:[0,1,3]})
 expect(()=>compileModelGraphText(source.replace('parameters: [0,1,3]','parameters: [0mm,1mm,3mm]'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('natural_loft_surface(a,b,c,','natural_loft_surface(a,'))).toThrow()
})
it('matches independent rational profile and natural cubic elevation through WASM',()=>{
 const sections=[0,3,0].map(z=>bezierNurbsCurve([[2,0,z],[2,2,z],[0,2,z]],[1,Math.SQRT1_2,1]))
 const s=naturalLoftNurbsCurves(sections,[0,.5,1])
 for(const v of [0,.13,.3,.5,.7,.87,1])for(const u of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point,x=Math.min(v,1-v)
  expect(p[2]).toBeCloseTo(9*x-12*x**3,9)
  expect(p[0]**2+p[1]**2).toBeCloseTo(4,9)
 }
 expect(()=>naturalLoftNurbsCurves(sections,[0,0,1])).toThrow()
 expect(()=>naturalLoftNurbsCurves(sections,[0,1])).toThrow()
})
it('preserves differently weighted sections at nonuniform stations and refuses weight overshoot',()=>{
 const sections=[0,3,7].map((z,j)=>bezierNurbsCurve([[2,0,z],[2,2,z],[0,2,z]],[1,Math.SQRT1_2,1].map(w=>w*(1+.2*j))))
 const s=naturalLoftNurbsCurves(sections,[-2,-1,3])
 for(const [j,v] of [0,.2,1].entries())for(const u of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point,q=evaluateNurbsCurve(sections[j]!,u).point
  p.forEach((x,k)=>expect(x).toBeCloseTo(q[k]!,9))
 }
 const bad=sections.map((c,j)=>({...c,weights:c.weights.map(w=>w/[1,1.2,1.4][j]!*[1,.01,10][j]!)}))
 expect(()=>naturalLoftNurbsCurves(bad,[0,.5,1])).toThrow(/positive control weights/)
})
