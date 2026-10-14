import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,closedLoftNurbsCurves} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'

const sections=()=>[[1,0],[0,1],[-1,0],[0,-1],[1,0]].map(([x,y])=>bezierNurbsCurve([[x!,y!,-2],[x!,y!,2]]))
it('lowers cyclic section references and dimensionless stations through Rush',()=>{
 const source=readFileSync('examples/rush/closed-loft-surface.r','utf8'),g=compileRushFrontend(source)
 expect(g.execution_target).toBe('own-nurbs')
 expect(g.document.nodes.find(n=>n.op==='closed_loft_surface')).toMatchObject({parameters:[0,1,2,3,4]})
 expect(()=>compileRushFrontend(source.replace('parameters: [0,1,2,3,4]','parameters: [0mm,1mm,2mm,3mm,4mm]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('closed_loft_surface(a,b,c,d,a,','closed_loft_surface(a,b,c,'))).toThrow()
})
it('matches independent cyclic cubic span through WASM',()=>{
 const s=closedLoftNurbsCurves(sections(),[0,1,2,3,4])
 expect(s.periodicV).toBe(false)
 for(const u of [0,.17,.5,.83,1]){
  const p=evaluateNurbsSurface(s,u,.125).point
  p.forEach((x,k)=>expect(x).toBeCloseTo([.6875,.6875,-2+4*u][k]!,9))
 }
})
it('preserves nonuniform cyclic seam jets and rejects unmatched homogeneous profiles',()=>{
 const c=sections(),s=closedLoftNurbsCurves(c,[-2,-1,.5,2,4])
 for(const u of [0,.17,.5,.83,1]){
  const a=evaluateNurbsSurface(s,u,0),b=evaluateNurbsSurface(s,u,1)
  for(const key of ['point','du','dv','duu','duv','dvv'] as const){
   a[key]!.forEach((x,k)=>expect(x).toBeCloseTo(b[key]![k]!,8))
  }
 }
 const bad=c.map(x=>({...x,weights:[...x.weights]}));bad[4]!.weights[0]!*=1.001
 expect(()=>closedLoftNurbsCurves(bad,[0,1,2,3,4])).toThrow(/endpoints must match/)
 expect(()=>closedLoftNurbsCurves(c,[0,1,1,3,4])).toThrow()
})
