import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,gordonNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('lowers named curve families and dimensionless stations through Rush',()=>{
 const source=readFileSync('examples/rush/gordon-surface.r','utf8'),g=compileRushFrontend(source)
 expect(g.execution_target).toBe('own-nurbs')
 const n=g.document.nodes.find(n=>n.op==='gordon_surface')!
 expect(n).toMatchObject({parameters_u:[0,.5,1],parameters_v:[0,.5,1]})
 expect(()=>compileRushFrontend(source.replace('parameters_u: [0,0.5,1]','parameters_u: [0mm,0.5mm,1mm]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('u_curves: [u0,u1,u2]','u_curves: [missing,u1,u2]'))).toThrow()
})
it('preserves nonlinear network curves and independent Gordon formula through WASM',()=>{
 const u=[0,.5,1].map(v=>bezierNurbsCurve([[0,v,0],[.5,v,(v+v*v)/2],[1,v,v]]))
 const v=[0,.5,1].map(u=>bezierNurbsCurve([[u,0,0],[u,.5,u/2],[u,1,u+u*(1-u)]]))
 const s=gordonNurbsSurface(u,v,[0,.5,1],[0,.5,1])
 const n=(x:number)=>{
  const [a,b,p0,p1,m0,m1]=x<.5?[0,.5,0,.25,0,3]:[.5,1,.25,1,3,0]
  const h=b!-a!,t=(x-a!)/h,l=1-t
  return l*p0!+t*p1!+h*h*((l**3-l)*m0!+(t**3-t)*m1!)/6
 }
 for(const x of [0,.13,.5,.87,1])for(const y of [0,.17,.5,.83,1]){
  const ng=x-n(x),z=x*y+x*(1-x)*n(y)+y*y*ng-ng*n(y)
  expect(evaluateNurbsSurface(s,x,y).point[2]).toBeCloseTo(z,9)
 }
 for(let j=0;j<3;j++)for(const t of [0,.13,.37,.83,1]){
  const a=evaluateNurbsSurface(s,t,j/2).point,b=evaluateNurbsCurve(u[j]!,t).point
  const c=evaluateNurbsSurface(s,j/2,t).point,d=evaluateNurbsCurve(v[j]!,t).point
  a.forEach((x,k)=>expect(x).toBeCloseTo(b[k]!,9));c.forEach((x,k)=>expect(x).toBeCloseTo(d[k]!,9))
 }
 expect(()=>gordonNurbsSurface(u,v,[0,0,1],[0,.5,1])).toThrow()
})
it('preserves rational arcs between network crossings through WASM',()=>{
 const a=bezierNurbsCurve([[2,0,0],[2,2,0],[0,2,0]],[1,Math.SQRT1_2,1])
 const b=bezierNurbsCurve([[2,0,2],[2,2,2],[0,2,2]],[1,Math.SQRT1_2,1])
 const left=bezierNurbsCurve([[2,0,0],[2,0,2]]),right=bezierNurbsCurve([[0,2,0],[0,2,2]])
 const s=gordonNurbsSurface([a,b],[left,right],[0,1],[0,1])
 for(const u of [0,.13,.37,.83,1])for(const v of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point
  expect(p[0]**2+p[1]**2).toBeCloseTo(4,9)
  expect(p[2]).toBeCloseTo(2*v,9)
 }
 const bad={...right,weights:[2,2]}
 expect(()=>gordonNurbsSurface([a,b],[left,bad],[0,1],[0,1])).toThrow(/homogeneous crossings/)
})
