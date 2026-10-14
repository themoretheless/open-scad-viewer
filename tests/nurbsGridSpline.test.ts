import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {gridSplineNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('reproduces independent bilinear geometry on nonuniform authored parameters',()=>{
 const u=[-2,-1.5,1,3],v=[0,.3,2],points=u.map(x=>v.map(y=>[x,y,x*y] as [number,number,number]))
 const s=gridSplineNurbsSurface(points,u,v)
 for(const x of [0,.13,.5,.87,1])for(const y of [0,.17,.5,.83,1]){
  const q=evaluateNurbsSurface(s,x,y),a=-2+5*x,b=2*y
  for(let k=0;k<3;k++){
   expect(q.point[k]).toBeCloseTo([a,b,a*b][k]!,9)
   expect(q.du![k]).toBeCloseTo([5,0,5*b][k]!,8)
   expect(q.dv![k]).toBeCloseTo([0,2,2*a][k]!,8)
  }
 }
 for(let i=0;i<u.length;i++)for(let j=0;j<v.length;j++){
  const q=evaluateNurbsSurface(s,(u[i]!+2)/5,v[j]!/2)
  q.point.forEach((x,k)=>expect(x).toBeCloseTo(points[i]![j]![k]!,10))
 }
})
it('refuses malformed grids and parameter counts before interpolation',()=>{
 const p:[number,number,number][][]=[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]]
 expect(()=>gridSplineNurbsSurface(p,[0,0],[0,1])).toThrow()
 expect(()=>gridSplineNurbsSurface(p,[0,1],[0])).toThrow()
 expect(()=>gridSplineNurbsSurface([p[0]!,p[1]!.slice(0,1)],[0,1],[0,1])).toThrow()
 expect(()=>gridSplineNurbsSurface([],[],[])).toThrow()
})
it('matches an independent non-bilinear product of natural cubics through WASM',()=>{
 const u=[0,1/3,1],v=[0,.5,1],f=[0,2,0],g=[0,3,0]
 const p=u.map((x,i)=>v.map((y,j)=>[x,y,f[i]!*g[j]!] as [number,number,number]))
 const surface=gridSplineNurbsSurface(p,u,v)
 const scalarU=(x:number)=>{
  const [a,b,p0,p1,m0,m1]=x<1/3?[0,1/3,0,2,0,-27]:[1/3,1,2,0,-27,0]
  const h=b!-a!,t=(x-a!)/h,l=1-t
  return l*p0!+t*p1!+h*h*((l**3-l)*m0!+(t**3-t)*m1!)/6
 }
 const scalarV=(x:number)=>{const a=Math.min(x,1-x);return 9*a-12*a**3}
 for(const x of [0,.13,.5,.87,1])for(const y of [0,.17,.5,.83,1]){
  expect(evaluateNurbsSurface(surface,x,y).point[2]).toBeCloseTo(scalarU(x)*scalarV(y),9)
 }
})
it('lowers grid sites as lengths and both parameter arrays dimensionlessly through Rush',()=>{
 const source=readFileSync('examples/rush/grid-spline-surface.r','utf8'),graph=compileRushFrontend(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='grid_spline_surface')).toMatchObject({parameters_u:[0,1,3],parameters_v:[0,1,2]})
 expect(()=>compileRushFrontend(source.replace('8mm','8deg'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('parameters_u: [0,1,3]','parameters_u: [0mm,1mm,3mm]'))).toThrow()
})
