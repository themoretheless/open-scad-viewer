import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {ellipseTransitionNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
function angular(u:number):[number,number]{
 const t=u*4,q=Math.min(3,Math.floor(t)),s=t-q,a=(1-s)**2,b=2*Math.SQRT1_2*s*(1-s),c=s*s,x=(a+b)/(a+b+c),y=(b+c)/(a+b+c)
 return ([[x,y],[-y,x],[-x,-y],[y,-x]] as [number,number][])[q]!
}
it('preserves rotated skew ellipses and their independent ruled equation',()=>{
 const a={center:[3,4,5] as [number,number,number],axisU:[2,0,0] as [number,number,number],axisV:[0,1,0] as [number,number,number]},b={center:[6,7,8] as [number,number,number],axisU:[0,3,0] as [number,number,number],axisV:[1,1,2] as [number,number,number]}
 const surface=ellipseTransitionNurbsSurface(a,b)
 expect(surface.degreeU).toBe(2);expect(surface.degreeV).toBe(1);expect(surface.controlPoints).toHaveLength(9)
 expect(surface.controlPoints.every(row=>row.length===2)).toBe(true)
 for(let i=0;i<=1000;i++)for(const v of [0,.13,.5,.87,1]){
  const u=i/1000,[x,y]=angular(u),p=evaluateNurbsSurface(surface,u,v).point
  const expected=a.center.map((c,k)=>(1-v)*(c+a.axisU[k]!*x+a.axisV[k]!*y)+v*(b.center[k]!+b.axisU[k]!*x+b.axisV[k]!*y))
  expect(Math.hypot(...p.map((c,k)=>c-expected[k]!))).toBeLessThan(1e-10)
 }
 expect(surface.controlPoints[0]).toEqual(surface.controlPoints.at(-1))
 expect(()=>ellipseTransitionNurbsSurface(a,a)).toThrow()
 expect(()=>ellipseTransitionNurbsSurface(a,{...b,axisV:[0,6,0]})).toThrow()
})
it('requires length units for all ellipse centers and axis vectors in Rush',()=>{
 const source=readFileSync('examples/rush/ellipse-transition-surface.r','utf8')
 expect(compileRushFrontend(source).document.nodes.find(n=>n.op==='ellipse_transition_surface')).toMatchObject({end_axis_v:[1,1,2]})
 for(const [a,b] of [['start_center: [3mm,4mm,5mm]','start_center: [3deg,4mm,5mm]'],['end_axis_u: [0mm,3mm,0mm]','end_axis_u: [0mm,3deg,0mm]'],[',end_axis_v: [1mm,1mm,2mm]','']])expect(()=>compileRushFrontend(source.replace(a!,b!))).toThrow()
})
