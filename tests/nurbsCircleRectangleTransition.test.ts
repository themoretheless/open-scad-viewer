import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {circleRectangleTransitionNurbsPatches} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
const circle={center:[0,0,0] as [number,number,number],normal:[0,0,1] as [number,number,number],seam:[1,0,0] as [number,number,number],radius:2}
const rectangle={center:[1,2,5] as [number,number,number],axisU:[3,0,0] as [number,number,number],axisV:[0,1,0] as [number,number,number]}
it('retains four compact patches, independent boundaries and linear interior',()=>{
 const patches=circleRectangleTransitionNurbsPatches(circle,rectangle),corners=[[4,3,5],[-2,3,5],[-2,1,5],[4,1,5]]
 expect(patches).toHaveLength(4)
 for(let i=0;i<4;i++){
  const s=patches[i]!
  expect(s.degreeU).toBe(3);expect(s.degreeV).toBe(1);expect(s.controlPoints).toHaveLength(4)
  expect(s.knotsU[0]).toBe(i/4);expect(s.knotsU.at(-1)).toBe((i+1)/4)
  for(let j=0;j<=200;j++)for(const v of [0,.13,.5,.87,1]){
   const t=j/200,u=(i+t)/4,a=(1-t)**2,b=2*Math.SQRT1_2*t*(1-t),c=t*t,x=(a+b)/(a+b+c),y=(b+c)/(a+b+c)
   const xy=[[x,y],[-y,x],[-x,-y],[y,-x]][i]!,ca=[2*xy[0]!,2*xy[1]!,0],rb=corners[i]!.map((x,k)=>(1-t)*x+t*corners[(i+1)%4]![k]!)
   const p=evaluateNurbsSurface(s,u,v).point
   expect(Math.hypot(...p.map((x,k)=>x-((1-v)*ca[k]!+v*rb[k]!)))).toBeLessThan(1e-10)
  }
  for(const v of [0,.13,.5,.87,1]){
   const a=evaluateNurbsSurface(s,(i+1)/4,v).point,b=evaluateNurbsSurface(patches[(i+1)%4]!,((i+1)%4)/4,v).point
   expect(Math.hypot(...a.map((x,k)=>x-b[k]!))).toBeLessThan(1e-10)
  }
 }
 expect(()=>circleRectangleTransitionNurbsPatches(circle,{...rectangle,axisV:[1,1,0]})).toThrow()
})
it('checks circle directions as scalars and rectangle half-edges as lengths in Rush',()=>{
 const source=readFileSync('examples/rush/circle-rectangle-transition.r','utf8')
 expect(compileModelGraphText(source).document.nodes.find(n=>n.op==='circle_rectangle_transition')).toMatchObject({circle_radius:2,rectangle_axis_u:[3,0,0]})
 for(const [a,b] of [['circle_normal: [0,0,1]','circle_normal: [0,0,1mm]'],['rectangle_axis_u: [3mm,0mm,0mm]','rectangle_axis_u: [3deg,0mm,0mm]'],['circle_radius: 2mm','circle_radius: 2deg'],[',rectangle_axis_v: [0mm,1mm,0mm]','']])expect(()=>compileModelGraphText(source.replace(a!,b!))).toThrow()
})
