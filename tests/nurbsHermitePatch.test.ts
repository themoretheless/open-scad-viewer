import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {hermiteNurbsPatch,type NurbsCornerJets} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'

const jet=(u:number,v:number)=>[
 [u+2*v+u*v,u*u+v**3+u**3*v*v,u**3*v**3],
 [1+v,2*u+3*u*u*v*v,3*u*u*v**3],
 [2+u,3*v*v+2*u**3*v,3*u**3*v*v],
 [1,6*u*u*v,9*u*u*v*v],
]
it('matches independent bicubic positions, tangents and mixed derivatives',()=>{
 const data=Array.from({length:4},(_,k)=>Array.from({length:2},(_,i)=>Array.from({length:2},(_,j)=>jet(i,j)[k]))) as NurbsCornerJets[]
 const surface=hermiteNurbsPatch(data[0]!,data[1]!,data[2]!,data[3]!)
 for(const u of [0,.13,.5,.87,1])for(const v of [0,.17,.5,.83,1]){
  const q=evaluateNurbsSurface(surface,u,v),expected=jet(u,v)
  for(let k=0;k<3;k++){
   expect(q.point[k]).toBeCloseTo(expected[0]![k]!,10)
   expect(q.du![k]).toBeCloseTo(expected[1]![k]!,9)
   expect(q.dv![k]).toBeCloseTo(expected[2]![k]!,9)
   expect(q.duv![k]).toBeCloseTo(expected[3]![k]!,8)
  }
 }
})
it('refuses nonfinite or rounded-away authored conditions while admitting degenerate patches',()=>{
 const grid=(x:number)=>[[[x,x,x],[x,x,x]],[[x,x,x],[x,x,x]]] as NurbsCornerJets
 const zero=grid(0),large=grid(1e9),tiny=grid(1e-9)
 expect(()=>hermiteNurbsPatch(zero,zero,zero,zero)).not.toThrow()
 expect(()=>hermiteNurbsPatch(grid(NaN),zero,zero,zero)).toThrow()
 expect(()=>hermiteNurbsPatch(large,tiny,zero,zero)).toThrow()
 expect(()=>hermiteNurbsPatch(large,zero,tiny,zero)).toThrow()
 expect(()=>hermiteNurbsPatch(large,zero,zero,tiny)).toThrow()
})
it('lowers corner jets using length units through Rush',()=>{
 const source=readFileSync('examples/rush/hermite-patch.r','utf8'),graph=compileRushFrontend(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='hermite_patch')).toMatchObject({twist:[[[0,0,0],[0,0,0]],[[0,0,0],[0,0,18]]]})
 expect(()=>compileRushFrontend(source.replace('18mm','18deg'))).toThrow()
})
