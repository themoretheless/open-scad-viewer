import {it,expect} from 'vitest'
import {createRuledBrepLoft,inspectNurbsBrep,analyzeNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
type P=[number,number,number]
const base:P[]=[[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]]
const c=Math.SQRT1_2
const top=base.map(([x,y]):P=>[c*(x-y),c*(x+y),3])
it('retains bilinear ruled side surfaces and integrates their volume independently of display facets',()=>{
 const sections=[base,top],before=JSON.stringify(sections),model=createRuledBrepLoft(sections)
 expect(model.faces).toHaveLength(6);expect(model.edges).toHaveLength(12)
 expect(model.loops.every(l=>l.coedges.length===4)).toBe(true)
 expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 // A(t)=4*((1-t)^2+2*cos(45°)*t*(1-t)+t^2), height=3.
 const expected=4*(2+c)
 expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(expected,6)
 const coarse=tessellateNurbsBrep(model,4),fine=tessellateNurbsBrep(model,16)
 expect(coarse.report.closed).toBe(true);expect(fine.report.closed).toBe(true)
 expect(Math.abs(fine.report.signedVolumeMm3-expected)).toBeLessThan(Math.abs(coarse.report.signedVolumeMm3-expected))
 expect(JSON.stringify(sections)).toBe(before)
})
it('keeps ruled volume under rigid placement and refuses unsupported correspondence atomically',()=>{
 const place=([x,y,z]:P):P=>[c*x+c*z+5,y-3,-c*x+c*z+7]
 const model=createRuledBrepLoft([base.map(place),top.map(place)])
 expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(4*(2+c),6)
 const twist=base.map(([x,y]):P=>[-x,-y,3])
 expect(()=>createRuledBrepLoft([base,twist])).toThrow('intermediate')
 expect(()=>createRuledBrepLoft([base,top.slice(1)])).toThrow()
 expect(createRuledBrepLoft([base,top]).faces).toHaveLength(6)
})
it('sews multiple ruled intervals through one shared station ring',()=>{
 const last=base.map(([x,y]):P=>[x,y,6])
 const model=createRuledBrepLoft([base,top,last])
 expect(model.faces).toHaveLength(10);expect(model.edges).toHaveLength(20)
 expect(model.bodies).toHaveLength(1)
 expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(8*(2+c),6)
 expect(tessellateNurbsBrep(model,4).report.closed).toBe(true)
})
it('admits large valid rotations after support refinement and preserves the exact section-area volume',()=>{
 for(const angle of [90,120,150]){
  const radians=angle*Math.PI/180,s=Math.sin(radians),cos=Math.cos(radians)
  const rotated=base.map(([x,y]):P=>[cos*x-s*y,s*x+cos*y,3])
  const model=createRuledBrepLoft([base,rotated])
  expect(model.faces).toHaveLength(6)
  expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo(4*(2+cos),6)
  expect(tessellateNurbsBrep(model,8).report.closed).toBe(true)
 }
})
