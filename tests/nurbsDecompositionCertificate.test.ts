import {expect,it} from 'vitest'
import {decomposeNurbsCurve,inspectNurbsDecomposition,inspectNurbsDecompositionBatch,type NurbsCurve} from '../src/services/nurbsCurve'
it('transports original rational span bounds, budget refusal and independent translation',()=>{
 const curve:NurbsCurve={degree:2,knots:[0,0,0,.4,1,1,1],controlPoints:[[0,0,0],[.3,.8,0],[.7,-.2,0],[1,0,0]],weights:[1,.75,1.25,2],periodic:false}
 const before=structuredClone(curve),parts=decomposeNurbsCurve(curve)
 for(const [index,part] of parts.entries()){
  const r=inspectNurbsDecomposition(curve,index+2,part.curve)
  expect(r).toMatchObject({reason:null,method:'original-span-bernstein-decomposition',continuousBound:false})
  expect(r.errorUpper).not.toBeNull();expect(r.errorUpper!).toBeLessThan(1e-10)
  expect(inspectNurbsDecomposition(curve,index+2,part.curve,r.products-1)).toMatchObject({errorUpper:null,reason:'work-limit'})
  const shifted=structuredClone(part.curve);for(const point of shifted.controlPoints)point[2]!+=.125
  expect(inspectNurbsDecomposition(curve,index+2,shifted).errorUpper!).toBeGreaterThanOrEqual(.125)
  expect(inspectNurbsDecomposition(curve,0,part.curve).errorUpper).toBeNull()
  expect(()=>inspectNurbsDecomposition(curve,index+2,part.curve,-1)).toThrow()
  expect(()=>inspectNurbsDecomposition(curve,index+2,part.curve,1000001)).toThrow()
 }
 expect(curve).toEqual(before)
 const pairs=parts.map((part,i)=>({curve,span:i+2,retained:part.curve}))
 const all=inspectNurbsDecompositionBatch(pairs,18)
 expect(all).toMatchObject({pairsInspected:2,products:18,reason:null})
 expect(all.errorUpper!).toBeLessThan(1e-10)
 expect(inspectNurbsDecompositionBatch(pairs,17)).toMatchObject({errorUpper:null,pairsInspected:2,reason:'work-limit'})
 expect(inspectNurbsDecompositionBatch([...pairs,{...pairs[0]!,span:0}],100).errorUpper).toBeNull()
 expect(inspectNurbsDecompositionBatch([],100).errorUpper).toBeNull()
})
