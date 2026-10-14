import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,clampedLoftNurbsCurves} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('lowers section references with dimensionless parameters and length tangents',()=>{
 const source=readFileSync('examples/rush/clamped-loft-surface.r','utf8')
 const g=compileRushFrontend(source)
 expect(g.execution_target).toBe('own-nurbs')
 expect(g.document.nodes.find(n=>n.op==='clamped_loft_surface')).toMatchObject({parameters:[0,1,3],start_tangent:[0,0,8],end_tangent:[0,0,12]})
 expect(()=>compileRushFrontend(source.replace('parameters: [0,1,3]','parameters: [0mm,1mm,3mm]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('8mm]', '8deg]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',end_tangent: [0mm,0mm,12mm]',''))).toThrow()
})
const section=(z:number,scale=1)=>bezierNurbsCurve([[2,0,z],[2,2,z],[0,2,z]],[1,Math.SQRT1_2,1].map(w=>w*scale))
it('matches independent two-section Hermite elevation through WASM',()=>{
 const s=clampedLoftNurbsCurves([section(0),section(7)],[2,7],[0,0,3],[0,0,-1])
 for(const v of [0,.13,.3,.5,.7,.87,1])for(const u of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,v).point
  const z=(-2*v**3+3*v*v)*7+(v**3-2*v*v+v)*15-(v**3-v*v)*5
  expect(p[2]).toBeCloseTo(z,9)
  expect(p[0]**2+p[1]**2).toBeCloseTo(4,9)
 }
})
it('preserves Cartesian endpoint conditions for nonuniform stations and rational weights',()=>{
 const s=clampedLoftNurbsCurves([section(0),section(3,1.2),section(7,1.4)],[2,3,7],[1,2,3],[-2,1,4])
 for(const u of [0,.17,.5,.83,1]){
  for(const [v,t] of [[0,[1,2,3]],[1,[-2,1,4]]] as const){
   const q=evaluateNurbsSurface(s,u,v)
   q.dv!.forEach((x,k)=>expect(x).toBeCloseTo(5*t[k]!,8))
  }
  expect(evaluateNurbsSurface(s,u,.2).point[2]).toBeCloseTo(3,9)
 }
 expect(()=>clampedLoftNurbsCurves([section(0),section(1)],[0,0],[0,0,1],[0,0,1])).toThrow()
 expect(()=>clampedLoftNurbsCurves([section(0),section(1,.01),section(2,10)],[0,.5,1],[0,0,1],[0,0,1])).toThrow(/positive control weights/)
})
