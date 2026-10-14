import {expect,it} from 'vitest'
import {bezierNurbsCurve,lineNurbsCurve,extrudeNurbsCurve} from '../src/services/nurbsConstructors'
import {transformNurbsCurve,evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {transformNurbsSurface,transformNurbsSurfacePatches,evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
const m=[[-2,.5,0,3],[0,1,1,-4],[1,0,.25,2],[0,0,0,1]]
it('uses native affine rational curve and surface transforms',()=>{
 const curve=bezierNurbsCurve([[0,0,0],[2,3,1],[4,0,2]],[1,2,1]),transformed=transformNurbsCurve(curve,m)
 expect(transformed.weights).toEqual(curve.weights);expect(transformed.knots).toEqual(curve.knots)
 for(let i=0;i<=100;i++){
  const u=i/100,p=evaluateNurbsCurve(curve,u).point,q=evaluateNurbsCurve(transformed,u).point,expected=[-2*p[0]!+.5*p[1]!+3,p[1]!+p[2]!-4,p[0]!+.25*p[2]!+2]
  expect(Math.hypot(...q.map((x,k)=>x-expected[k]!))).toBeLessThan(1e-10)
 }
 const surface=extrudeNurbsCurve(lineNurbsCurve([0,0,0],[2,0,0]),[0,3,4]),s=transformNurbsSurface(surface,m)
 expect(transformNurbsSurfacePatches([surface,surface],m)).toEqual([s,s])
 const p=evaluateNurbsSurface(s,.3,.7).point
 expect(Math.hypot(...p.map((x,k)=>x-[-4*.3+1.5*.7+3,7*.7-4,2*.3+.7+2][k]!))).toBeLessThan(1e-10)
 expect(()=>transformNurbsCurve(curve,[[1,0,0,0],[0,1,0,0],[0,0,1,0],[.1,0,0,1]])).toThrow(/affine/)
})
it('routes graph control transformations through the Rust adapter',()=>{
 const document={language:'rush/nurbs-1' as const,parameters:[],units:'mm' as const,root:'mesh',nodes:[{id:'c',op:'line_curve' as const,start:[0,0,0],end:[2,0,0]},{id:'s',op:'surface_extrude' as const,input:'c',vector:[0,3,4]},{id:'a',op:'transform' as const,input:'s',matrix:m},{id:'mesh',op:'tessellate' as const,input:'a',segments_u:4,segments_v:4}]}
 const result=buildOwnNurbs(document,{action:'build'})
 expect(result.mesh.positions.every(Number.isFinite)).toBe(true)
})
