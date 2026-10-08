import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {approximateScrewNurbsSurfaces,bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
it('matches independent rational-profile screw coordinates at retained subdomains',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,1]],[1,2])
 for(const [turns,height] of [[1,4],[-.5,-3]]){
  const a=approximateScrewNurbsSurfaces(profile,[0,0,0],[0,0,1],height!,turns!,23,1e-4)
  for(const patch of a.patches){
   const lo=patch.knotsV[0]!,hi=patch.knotsV.at(-1)!
   for(const u of [0,.13,.5,.87,1])for(const t of [0,.17,.5,.83,1]){
    const v=lo+(hi-lo)*t,p=evaluateNurbsCurve(profile,u).point,theta=23*Math.PI/180+2*Math.PI*turns!*v
    const q=evaluateNurbsSurface(patch,u,v).point
    expect(Math.hypot(q[0]!-p[0]!*Math.cos(theta),q[1]!-p[0]!*Math.sin(theta),q[2]!-p[2]!-height!*v)).toBeLessThanOrEqual(a.report.realArithmeticErrorEstimate+1e-11)
   }
  }
 }
 expect(()=>approximateScrewNurbsSurfaces(profile,[0,0,0],[0,0,0],1,1,0,1e-4)).toThrow()
})
it('checks dimensional screw fields and required budget in packaged Rush',()=>{
 const source=readFileSync('examples/rush/screw-surface.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='screw_surface')).toMatchObject({height:4,turns:1,phase_degrees:23})
 expect(()=>compileRushFrontend(source.replace('height: 4mm','height: 4deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('turns: 1','turns: 1mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('phase_degrees: 23deg','phase_degrees: 23mm'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',max_deviation: 0.0001mm',''))).toThrow()
})
it('preserves screw construction report and patch set through graph build',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/screw-surface.r','utf8'))
 const node=graph.document.nodes.find(n=>n.op==='screw_surface')!
 const built=buildOwnNurbs(graph.document,{action:'build'})
 expect(built.report.error_bound_certified).toBe(false)
 expect(built.report.construction?.[node.id]).toMatchObject({budget:1e-4,continuousBound:false,roundingCertified:false,method:'profile-screw-helix-Hermite'})
})
