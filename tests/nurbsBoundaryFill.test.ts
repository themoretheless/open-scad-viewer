import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,boundaryFillNurbsSurfaces} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {tessellateNurbsPatches} from '../src/services/geometry/reconstruction'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'

it('lowers positional boundary cycle and length center through Rush',()=>{
 const source=readFileSync('examples/rush/boundary-fill.r','utf8'),g=compileRushFrontend(source)
 expect(g.execution_target).toBe('own-nurbs')
 expect(g.document.nodes.find(n=>n.op==='boundary_fill')).toMatchObject({center:[10,10,5]})
 expect(()=>compileRushFrontend(source.replace('center: [10mm,10mm,5mm]','center: [10mm,10mm,5deg]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace(',center: [10mm,10mm,5mm]',''))).toThrow()
})
it('retains every boundary and common spoke across a five-patch rational fan',()=>{
 const p:[number,number,number][]=[[0,0,0],[2,0,0],[3,1,0],[1,3,0],[-1,1,0]]
 const edges=p.map((a,i)=>bezierNurbsCurve([a,p[(i+1)%p.length]!],i===1?[2,3]:undefined))
 const center:[number,number,number]=[1,1,2],patches=boundaryFillNurbsSurfaces(edges,center)
 expect(patches).toHaveLength(5)
 for(let i=0;i<5;i++)for(const t of [0,.13,.37,.83,1]){
  const a=evaluateNurbsSurface(patches[i]!,t,0).point,b=evaluateNurbsCurve(edges[i]!,t).point
  const c=evaluateNurbsSurface(patches[i]!,1,t).point,d=evaluateNurbsSurface(patches[(i+1)%5]!,0,t).point
  a.forEach((x,k)=>expect(x).toBeCloseTo(b[k]!,9));c.forEach((x,k)=>expect(x).toBeCloseTo(d[k]!,9))
 }
 const mesh=tessellateNurbsPatches({patches,faceIds:[0,1,2,3,4]},8)
 expect(new Set(mesh.faceIds)).toEqual(new Set([0,1,2,3,4]))
 expect(mesh.faceIds).toHaveLength(mesh.indices.length/3)
 expect(()=>tessellateNurbsPatches({patches,faceIds:[0]},8)).toThrow(/budget/)
 expect(()=>boundaryFillNurbsSurfaces(edges.slice(0,4),center)).toThrow(/oriented cycle/)
})
it('transforms every rational patch control and preserves face IDs before tessellation',()=>{
 const source=readFileSync('examples/rush/boundary-fill.r','utf8')
 const g=compileRushFrontend(source).document
 const fill=g.nodes.find(n=>n.op==='boundary_fill')!
 const matrix=[[2,0,0,7],[0,3,0,-2],[0,0,4,3],[0,0,0,1]]
 const transformed={...g,nodes:[...g.nodes.filter(n=>n.id!==g.root),{id:'moved',op:'transform',input:fill.id,matrix},{id:'display',op:'nurbs_patches_tessellate',input:'moved',segments:8}],root:'display'}
 const built=buildOwnNurbs(transformed,{action:'build'})
 const definitions=(built as any).report.definitions
 const a=definitions[fill.id],b=definitions.moved
 expect(b.patches).toHaveLength(5);expect(b.faceIds).toEqual(a.faceIds)
 for(let i=0;i<5;i++)for(let u=0;u<a.patches[i].controlPoints.length;u++)for(let v=0;v<a.patches[i].controlPoints[u].length;v++){
  const p=a.patches[i].controlPoints[u][v],q=b.patches[i].controlPoints[u][v]
  expect(q).toEqual([2*p[0]+7,3*p[1]-2,4*p[2]+3])
 }
 expect(b.patches.map((p:any)=>p.weights)).toEqual(a.patches.map((p:any)=>p.weights))
 expect((built as any).mesh.faceIds).toContain(4)
 expect(b).not.toHaveProperty('sampledMaxDeviationMm')
})
it('reports the hull of all patches including distant edges and reflected coordinates',()=>{
 const points=[[0,0,0],[2,0,0],[3,1,0],[1,3,0],[-1,1,0]]
 const inputs=points.map((_,i)=>`edge${i}`)
 const edges=points.map((start,i)=>({id:inputs[i],op:'line_curve',start,end:points[(i+1)%5]}))
 const doc={language:'rush/nurbs-1',units:'mm',parameters:[],nodes:[...edges,{id:'fill',op:'boundary_fill',inputs,center:[1,1,2]}],root:'fill'}
 const a=buildOwnNurbs(doc,{action:'build'})
 expect(a.report.bounds_scope).toBe('conservative control hull')
 expect(a.report.bounds).toEqual({min:[-1,0,0],max:[3,3,2]})
 const moved={...doc,nodes:[...doc.nodes,{id:'reflected',op:'transform',input:'fill',matrix:[[2,0,0,7],[0,3,0,-2],[0,0,-4,3],[0,0,0,1]]}],root:'reflected'}
 const b=buildOwnNurbs(moved,{action:'build'})
 expect(b.report.bounds).toEqual({min:[5,-2,-5],max:[13,7,3]})
})
