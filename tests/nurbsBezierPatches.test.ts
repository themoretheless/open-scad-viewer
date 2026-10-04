import {expect,it} from 'vitest'
import {planeNurbsPatch,bilinearNurbsPatch,bezierNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {readFileSync} from 'node:fs'

it('preserves affine planes and nonplanar bilinear corners through WASM',()=>{
  const origin:[number,number,number]=[2,-3,5],a:[number,number,number]=[4,1,2],b:[number,number,number]=[-1,6,3]
  const plane=planeNurbsPatch(origin,a,b)
  const corners:Parameters<typeof bilinearNurbsPatch>[0]=[[[1,2,3],[4,1,8]],[[8,-1,2],[7,6,-4]]]
  const patch=bilinearNurbsPatch(corners)
  for(const u of [0,0.23,0.8,1])for(const v of [0,0.37,1]){
    const p=evaluateNurbsSurface(plane,u,v).point,q=evaluateNurbsSurface(patch,u,v).point
    for(let k=0;k<3;k++){
      expect(p[k]).toBeCloseTo(origin[k]!+u*a[k]!+v*b[k]!,11)
      expect(q[k]).toBeCloseTo((1-u)*(1-v)*corners[0][0][k]!+(1-u)*v*corners[0][1][k]!+u*(1-v)*corners[1][0][k]!+u*v*corners[1][1][k]!,11)
    }
  }
  expect(()=>planeNurbsPatch(origin,a,a)).toThrow()
  expect(()=>planeNurbsPatch([1e9,1e9,1e9],[1e-30,0,0],[0,1e-30,0])).toThrow()
})

it('evaluates rational tensor Bezier patches with an independent homogeneous oracle',()=>{
  const points:[number,number,number][][]=[[[0,0,0],[0,2,3],[0,4,0]],[[3,0,2],[3,2,6],[3,4,1]],[[6,0,0],[6,2,3],[6,4,0]]]
  const weights=[[1,2,1],[2,3,2],[1,2,1]]
  const s=bezierNurbsSurface(points,weights)
  function casteljau(p:number[][],t:number){
    const q=p.map(x=>x.slice())
    for(let n=q.length-1;n>0;n--)for(let i=0;i<n;i++)for(let k=0;k<4;k++)q[i]![k]=(1-t)*q[i]![k]!+t*q[i+1]![k]!
    return q[0]!
  }
  for(const u of [0,0.17,0.63,1])for(const v of [0,0.31,0.9,1]){
    const rows=points.map((r,i)=>casteljau(r.map((p,j)=>[...p.map(x=>x*weights[i]![j]!),weights[i]![j]!]),v))
    const h=casteljau(rows,u),q=evaluateNurbsSurface(s,u,v).point
    for(let k=0;k<3;k++)expect(q[k]).toBeCloseTo(h[k]!/h[3]!,11)
  }
  expect(bezierNurbsSurface(points).weights).toEqual([[1,1,1],[1,1,1],[1,1,1]])
  expect(()=>bezierNurbsSurface(points,[[1,1,1]])).toThrow()
})

it('lowers patch geometry as lengths and leaves rational weights dimensionless in Rush',()=>{
  for(const [name,op] of [['plane-patch','plane_patch'],['bilinear-patch','bilinear_patch'],['bezier-surface','bezier_surface']]){
    const source=readFileSync(`examples/rush/${name}.r`,'utf8')
    expect(compileModelGraphText(source).document.nodes.some(n=>n.op===op)).toBe(true)
    expect(()=>compileModelGraphText(source.replace('-10mm','-10deg'))).toThrow()
  }
})
