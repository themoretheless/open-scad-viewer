import {expect,it} from 'vitest'
import {bezierNurbsCurve,sweepNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {readFileSync} from 'node:fs'

it('retains both rational parameterizations in a fixed-orientation sweep',()=>{
 const profile=bezierNurbsCurve([[-4,0,0],[0,0,5],[4,0,0]],[1,2,1])
 const path=bezierNurbsCurve([[2,-3,7],[2,10,9],[12,20,13]],[1,3,2])
 const s=sweepNurbsCurve(profile,path),start=evaluateNurbsCurve(path,0).point
 expect(s.knotsU).toEqual(profile.knots);expect(s.knotsV).toEqual(path.knots)
 for(const u of [0,.13,.5,.81,1])for(const v of [0,.19,.5,.87,1]){
  const p=evaluateNurbsCurve(profile,u).point,q=evaluateNurbsCurve(path,v).point,r=evaluateNurbsSurface(s,u,v).point
  for(let k=0;k<3;k++)expect(r[k]).toBeCloseTo(p[k]!+(q[k]!-start[k]!),11)
 }
})

it('retains a tiny profile when the path starts far from the origin',()=>{
 const profile=bezierNurbsCurve([[1e-9,0,0],[2e-9,0,0]])
 const path=bezierNurbsCurve([[1e9,0,0],[1e9,0,5]])
 const s=sweepNurbsCurve(profile,path)
 expect(s.controlPoints.map(row=>row[0])).toEqual(profile.controlPoints)
 for(const u of [0,.17,.5,.83,1])for(const v of [0,.23,.7,1]){
  const p=evaluateNurbsSurface(s,u,v).point
  expect(Math.abs(p[0]-(1+u)*1e-9)).toBeLessThan(1e-23)
  expect(p[2]).toBeCloseTo(5*v,11)
 }
})

it('lowers profile/path references for a translation sweep in Rush',()=>{
 const graph=compileRushFrontend(readFileSync('examples/rush/translation-sweep.r','utf8'))
 expect(graph.document.nodes.find(n=>n.op==='surface_sweep')).toMatchObject({inputs:graph.document.nodes.slice(0,2).map(n=>n.id)})
})

it('removes common weight scales when raw tensor products exceed the admitted range',()=>{
 for(const scale of [1e-12,1e12]){
  const profile=bezierNurbsCurve([[1,0,0],[2,0,0]],[scale,scale]),path=bezierNurbsCurve([[0,0,0],[0,0,5]],[scale,scale])
  const s=sweepNurbsCurve(profile,path)
  expect(s.weights.flat()).toEqual([1,1,1,1])
  const p=evaluateNurbsSurface(s,.37,.61).point
  expect(p[0]).toBeCloseTo(1.37,11);expect(p[2]).toBeCloseTo(3.05,11)
 }
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]],[1e-12,1]),path=bezierNurbsCurve([[0,0,0],[0,0,5]],[1e-12,1])
 expect(()=>sweepNurbsCurve(profile,path)).toThrow()
})
