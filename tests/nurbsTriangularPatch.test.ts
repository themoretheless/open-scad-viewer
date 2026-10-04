import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,triangularNurbsPatch} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'

it('lowers three positional boundaries through Rush',()=>{
 const source=readFileSync('examples/rush/triangular-patch.r','utf8'),g=compileModelGraphText(source)
 expect(g.execution_target).toBe('own-nurbs')
 expect(g.document.nodes.find(n=>n.op==='triangular_patch')).toMatchObject({inputs:expect.any(Array)})
 expect(()=>compileModelGraphText(source.replace('triangular_patch(base,left,right)','triangular_patch(base,left)'))).toThrow()
})
it('matches independent triangle and preserves the collapsed apex through WASM',()=>{
 const a:[number,number,number]=[0,0,0],b:[number,number,number]=[2,0,0],c:[number,number,number]=[.5,2,1]
 const s=triangularNurbsPatch(bezierNurbsCurve([a,b]),bezierNurbsCurve([a,c]),bezierNurbsCurve([b,c]))
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
  const q=evaluateNurbsSurface(s,u,v)
  q.point.forEach((x,k)=>expect(x).toBeCloseTo((1-v)*((1-u)*a[k]!+u*b[k]!)+v*c[k]!,9))
  if(v===1)expect(q.normal).toBeNull()
 }
})
it('preserves differently weighted rational sides and their apex',()=>{
 const base=bezierNurbsCurve([[0,0,0],[2,0,0]])
 const left=bezierNurbsCurve([[0,0,0],[.5,2,1]],[2,2])
 const right=bezierNurbsCurve([[2,0,0],[.5,2,1]],[3,4])
 const s=triangularNurbsPatch(base,left,right)
 for(const t of [0,.13,.37,.83,1]){
  expect(evaluateNurbsSurface(s,t,1).point[2]).toBeCloseTo(1,9)
  for(const [u,c] of [[0,left],[1,right]] as const){
   const p=evaluateNurbsSurface(s,u,t).point,q=evaluateNurbsCurve(c,t).point
   p.forEach((x,k)=>expect(x).toBeCloseTo(q[k]!,9))
  }
 }
 const wrong=bezierNurbsCurve([[2,0,0],[.5,2,1.001]])
 expect(()=>triangularNurbsPatch(base,left,wrong)).toThrow(/endpoints must coincide/)
})
it('matches an independent cone identity for a rational arc boundary',()=>{
 const base=bezierNurbsCurve([[2,0,0],[2,2,0],[0,2,0]],[1,Math.SQRT1_2,1])
 const s=triangularNurbsPatch(base,bezierNurbsCurve([[2,0,0],[0,0,2]]),bezierNurbsCurve([[0,2,0],[0,0,2]]))
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point
  expect(p[0]**2+p[1]**2).toBeCloseTo((2-p[2])**2,9)
 }
})
