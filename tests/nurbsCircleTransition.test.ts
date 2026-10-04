import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {circleTransitionNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
function angular(u:number):[number,number]{
 const t=u*4,q=Math.min(3,Math.floor(t)),s=t-q,a=(1-s)**2,b=2*Math.SQRT1_2*s*(1-s),c=s*s,x=(a+b)/(a+b+c),y=(b+c)/(a+b+c)
 return ([[x,y],[-y,x],[-x,-y],[y,-x]] as [number,number][])[q]!
}
it('preserves two circles in perpendicular planes using compact ruled controls',()=>{
 const s=circleTransitionNurbsSurface({center:[0,0,0],normal:[0,0,1],seam:[1,0,0],radius:2},{center:[3,4,5],normal:[1,0,0],seam:[0,1,0],radius:3})
 expect(s.degreeU).toBe(2);expect(s.degreeV).toBe(1);expect(s.controlPoints).toHaveLength(9)
 expect(s.controlPoints.every(row=>row.length===2)).toBe(true)
 for(let i=0;i<=1000;i++)for(const v of [0,.13,.5,.87,1]){
  const u=i/1000,[x,y]=angular(u),p=evaluateNurbsSurface(s,u,v).point
  const expected=[(1-v)*2*x+3*v,(1-v)*2*y+v*(4+3*x),v*(5+3*y)]
  expect(Math.hypot(...p.map((a,k)=>a-expected[k]!))).toBeLessThan(1e-10)
 }
 expect(s.controlPoints[0]).toEqual(s.controlPoints.at(-1))
})
it('rejects coincident circles, invalid radii and parallel seam directions',()=>{
 const a={center:[0,0,0] as [number,number,number],normal:[0,0,1] as [number,number,number],seam:[1,0,0] as [number,number,number],radius:1}
 expect(()=>circleTransitionNurbsSurface(a,a)).toThrow()
 expect(()=>circleTransitionNurbsSurface(a,{...a,center:[0,0,1],radius:0})).toThrow()
 expect(()=>circleTransitionNurbsSurface(a,{...a,center:[0,0,1],seam:[0,0,2]})).toThrow()
})
it('checks centers and radii as lengths and directions as scalars in packaged Rush',()=>{
 const source=readFileSync('examples/rush/circle-transition-surface.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='circle_transition_surface')).toMatchObject({start_radius:2,end_radius:3,end_center:[3,4,5]})
 for(const [a,b] of [['end_center: [3mm,4mm,5mm]','end_center: [3deg,4mm,5mm]'],['end_radius: 3mm','end_radius: 3deg'],['start_normal: [0,0,1]','start_normal: [0,0,1mm]'],['start_seam: [1,0,0]','start_seam: [1mm,0,0]'],[',end_radius: 3mm','']])expect(()=>compileModelGraphText(source.replace(a!,b!))).toThrow()
})
