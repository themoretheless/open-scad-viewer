import {expect,it} from 'vitest'
import {bezierNurbsCurve,coonsNurbsPatch} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,reverseNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {readFileSync} from 'node:fs'

const points:[number,number,number][][]=[
 [[0,0,0],[10,0,5],[20,0,0]],
 [[0,16,0],[10,16,-3],[20,16,0]],
 [[0,0,0],[0,8,4],[0,16,0]],
 [[20,0,0],[20,8,-2],[20,16,0]]
]
function boundaries(){return points.map(p=>bezierNurbsCurve(p,[1,2,1]))}

it('retains all four rational Coons boundaries and an independently evaluated homogeneous interior',()=>{
 const curves=boundaries(),s=coonsNurbsPatch(curves)
 for(const t of [0,.07,.23,.5,.79,.93,1]){
  for(const [edge,u,v] of [[0,t,0],[1,t,1],[2,0,t],[3,1,t]]){
   const p=evaluateNurbsCurve(curves[edge!]!,t).point,q=evaluateNurbsSurface(s,u!,v!).point
   for(let k=0;k<3;k++)expect(q[k]).toBeCloseTo(p[k]!,11)
  }
 }
 const h=(index:number,t:number)=>{
  const bernstein=[(1-t)**2,2*t*(1-t),t*t],weights=[1,2,1]
  return [0,1,2,3].map(k=>bernstein.reduce((sum,b,i)=>sum+b*weights[i]!*(k===3?1:points[index]![i]![k]!),0))
 }
 for(const u of [.13,.5,.83])for(const v of [.19,.5,.91]){
  const bottom=h(0,u),top=h(1,u),left=h(2,v),right=h(3,v)
  const corners=[[0,0,0,1],[20,0,0,1],[0,16,0,1],[20,16,0,1]]
  const result=[0,1,2,3].map(k=>(1-v)*bottom[k]!+v*top[k]!+(1-u)*left[k]!+u*right[k]!-((1-u)*(1-v)*corners[0]![k]!+u*(1-v)*corners[1]![k]!+(1-u)*v*corners[2]![k]!+u*v*corners[3]![k]!))
  const q=evaluateNurbsSurface(s,u,v).point
  for(let k=0;k<3;k++)expect(q[k]).toBeCloseTo(result[k]!/result[3]!,11)
 }
})

it('refuses gaps, reversed boundaries, incompatible corner weights and wrong boundary count',()=>{
 const c=boundaries()
 expect(()=>coonsNurbsPatch(c.slice(0,3))).toThrow()
 expect(()=>coonsNurbsPatch([c[0]!,c[1]!,reverseNurbsCurve(c[2]!),c[3]!])).toThrow()
 const misplaced=structuredClone(c);misplaced[3]!.controlPoints[0]![2]=1
 expect(()=>coonsNurbsPatch(misplaced)).toThrow()
 const mismatched=structuredClone(c);mismatched[3]!.weights[2]=2
 expect(()=>coonsNurbsPatch(mismatched)).toThrow()
})

it('lowers four ordered boundary references in Rush and checks the graph arity',()=>{
 const source=readFileSync('examples/rush/coons-patch.r','utf8')
 const compiled=compileRushFrontend(source)
 expect(compiled.execution_target).toBe('own-nurbs')
 const patch=compiled.document.nodes.find(n=>n.op==='coons_patch')
 const boundaries=compiled.document.nodes.filter(n=>n.op==='bezier_curve')
 expect(boundaries).toHaveLength(4)
 expect(patch).toMatchObject({inputs:boundaries.map(n=>n.id)})
 expect(()=>compileRushFrontend(source.replace('coons_patch(bottom,top,left,right)','coons_patch(bottom,top,left)'))).toThrow()
})
