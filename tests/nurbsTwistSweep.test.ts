import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {bezierNurbsCurve,twistSweepNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'

it('preserves the independent rational quarter-turn law and path translation through WASM',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]])
 const path=bezierNurbsCurve([[0,0,0],[0,0,4]])
 const s=twistSweepNurbsCurve(profile,path,[0,0,0],[0,0,1],0,90)
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
  const w=(1-v)**2+Math.SQRT2*v*(1-v)+v*v
  const c=((1-v)**2+Math.SQRT2*v*(1-v))/w,t=(Math.SQRT2*v*(1-v)+v*v)/w
  const q=evaluateNurbsSurface(s,u,v).point
  expect(q[0]).toBeCloseTo((1+u)*c,9)
  expect(q[1]).toBeCloseTo((1+u)*t,9)
  expect(q[2]).toBeCloseTo(4*v,9)
 }
})
it('uses an explicit center and arbitrary fixed axis with a distant path anchor',()=>{
 const profile=bezierNurbsCurve([[10,21,30],[10,22,30]])
 const path=bezierNurbsCurve([[1e9,0,10],[1e9,0,14]])
 const s=twistSweepNurbsCurve(profile,path,[10,20,30],[1e300,0,0],0,90)
 const q=evaluateNurbsSurface(s,.5,1).point
 expect(q[0]).toBeCloseTo(10,9);expect(q[1]).toBeCloseTo(20,9);expect(q[2]).toBeCloseTo(35.5,9)
 expect(()=>twistSweepNurbsCurve(profile,path,[10,20,30],[0,0,0],0,90)).toThrow()
})
it('enforces length, dimensionless axis and angular dimensions through packaged Rush',()=>{
 const source=readFileSync('examples/rush/twist-sweep.r','utf8')
 const graph=compileModelGraphText(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='twist_sweep')).toMatchObject({origin:[0,0,0],axis:[0,0,1],start_degrees:0,sweep_degrees:180})
 expect(()=>compileModelGraphText(source.replace('180deg','180mm'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('axis: [0,0,1]','axis: [0,0,1mm]'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('origin: [0mm,0mm,0mm]','origin: [0mm,0mm,1deg]'))).toThrow()
})
it('preserves rational profile radius with a full turn and a constant nonzero orientation',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]],[1,2]),path=bezierNurbsCurve([[0,0,0],[0,0,4]])
 for(const angle of [0,360]){
  const s=twistSweepNurbsCurve(profile,path,[0,0,0],[0,0,1],45,angle)
  for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
   const q=evaluateNurbsSurface(s,u,v).point,radius=(1+3*u)/(1+u)
   expect(q[0]**2+q[1]**2).toBeCloseTo(radius**2,9)
   expect(q[2]).toBeCloseTo(4*v,9)
   if(angle===0){expect(q[0]).toBeCloseTo(radius*Math.SQRT1_2,9);expect(q[1]).toBeCloseTo(radius*Math.SQRT1_2,9)}
  }
 }
})
it('returns independent derivative jets of the quarter-turn rational functions',()=>{
 const profile=bezierNurbsCurve([[1,0,0],[2,0,0]]),path=bezierNurbsCurve([[0,0,0],[0,0,4]])
 const s=twistSweepNurbsCurve(profile,path,[0,0,0],[0,0,1],0,90)
 const q=Math.SQRT2
 for(const u of [.13,.5,.87])for(const v of [.17,.5,.83]){
  const w=1+(q-2)*v+(2-q)*v*v,wd=q-2+2*(2-q)*v,wdd=2*(2-q)
  const numerators=[[1,q-2,1-q],[0,q,1-q]]
  const e=evaluateNurbsSurface(s,u,v)
  for(let d=0;d<2;d++){
   const [a,b,c]=numerators[d]!,n=a!+b!*v+c!*v*v,nd=b!+2*c!*v,ndd=2*c!
   const first=nd/w-n*wd/(w*w)
   const second=ndd/w-n*wdd/(w*w)-2*nd*wd/(w*w)+2*n*wd*wd/(w*w*w)
   expect(e.du![d]).toBeCloseTo(n/w,9)
   expect(e.dv![d]).toBeCloseTo((1+u)*first,9)
   expect(e.duv![d]).toBeCloseTo(first,9)
   expect(e.dvv![d]).toBeCloseTo((1+u)*second,9)
  }
  expect(e.dv![2]).toBeCloseTo(4,9)
 }
})
