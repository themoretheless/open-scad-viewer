import {expect,it} from 'vitest'
import {bezierNurbsCurve,loftNurbsCurves,loftAlignedNurbsCurves,extrudeNurbsCurve,revolveNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {readFileSync} from 'node:fs'

it('retains ruled rational boundaries and straight generators when section weights agree',()=>{
  const a=bezierNurbsCurve([[-10,0,0],[0,8,0],[10,0,0]],[1,2,1])
  const b=bezierNurbsCurve([[-8,0,12],[0,-6,18],[8,0,12]],[1,2,1])
  const s=loftNurbsCurves([a,b])
  for(const u of [0,.13,.5,.87,1])for(const v of [0,.19,.71,1]){
    const p=evaluateNurbsCurve(a,u).point,q=evaluateNurbsCurve(b,u).point,r=evaluateNurbsSurface(s,u,v).point
    for(let k=0;k<3;k++)expect(r[k]).toBeCloseTo((1-v)*p[k]!+v*q[k]!,11)
  }
})

it('aligns section degree and interpolates every loft station without changing sections',()=>{
  const sections=[bezierNurbsCurve([[-10,0,0],[10,0,0]]),bezierNurbsCurve([[-8,0,10],[0,8,12],[8,0,10]]),bezierNurbsCurve([[-6,0,20],[0,-4,20],[6,0,20]])]
  const s=loftAlignedNurbsCurves(sections)
  sections.forEach((c,j)=>{for(const u of [0,.17,.5,.81,1]){
    const p=evaluateNurbsCurve(c,u).point,q=evaluateNurbsSurface(s,u,j).point
    for(let k=0;k<3;k++)expect(q[k]).toBeCloseTo(p[k]!,11)
  }})
})

it('preserves extrusion offsets and radial distances under a partial revolution',()=>{
  const c=bezierNurbsCurve([[8,0,0],[14,0,6],[10,0,15]])
  const e=extrudeNurbsCurve(c,[3,-2,12]),r=revolveNurbsCurve(c,[0,0,0],[0,0,1],270)
  const vEnd=r.knotsV[r.controlPoints[0]!.length]!
  for(const u of [0,.2,.57,1])for(const v of [0,.13,.7,1]){
    const p=evaluateNurbsCurve(c,u).point,q=evaluateNurbsSurface(e,u,v).point,s=evaluateNurbsSurface(r,u,v*vEnd).point
    for(let k=0;k<3;k++)expect(q[k]).toBeCloseTo(p[k]!+v*[3,-2,12][k]!,11)
    expect(Math.hypot(s[0]!,s[1]!)).toBeCloseTo(p[0]!,11)
    expect(s[2]).toBeCloseTo(p[2]!,11)
  }
  const last=evaluateNurbsSurface(r,.57,vEnd).point,p=evaluateNurbsCurve(c,.57).point
  expect(last[0]).toBeCloseTo(0,11);expect(last[1]).toBeCloseTo(-p[0]!,11)
  expect(()=>extrudeNurbsCurve(c,[0,0,0])).toThrow()
  expect(()=>revolveNurbsCurve(c,[0,0,0],[0,0,0],270)).toThrow()
})

it('lowers section and revolution construction examples through Rush',()=>{
  for(const [name,op] of [['profile-revolution','surface_revolve'],['ruled-surface','ruled_surface'],['section-loft','surface_loft']]){
    const source=readFileSync(`examples/rush/${name}.r`,'utf8')
    const graph=compileRushFrontend(source)
    expect(graph.document.nodes.some(n=>n.op===op)).toBe(true)
  }
  const revolution=readFileSync('examples/rush/profile-revolution.r','utf8')
  expect(()=>compileRushFrontend(revolution.replace('270deg','270mm'))).toThrow()
})
