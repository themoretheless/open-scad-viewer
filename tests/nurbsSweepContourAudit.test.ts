import {expect,it} from 'vitest'
import {inspectSweepContours} from '../src/services/nurbsSweepAudit'
import {bezierNurbsCurve} from '../src/services/nurbsConstructors'
const square=(x:number,y:number,size:number)=>{
 const points=[[x,y,0],[x+size,y,0],[x+size,y+size,0],[x,y+size,0]]
 return points.map((p,i)=>({...bezierNurbsCurve([p,points[(i+1)%4]!]),weights:[1,2]}))
}
const options={tolerance:1e-6,maxPairs:10000,maxCells:10000}
it('certifies exact unclamped hollow contours through WASM and refuses unproved altered weights',()=>{
 const outer={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:Array(6).fill(1),periodic:false}
 const hole={...structuredClone(outer),controlPoints:outer.controlPoints.map(p=>p.map(x=>x*.25))}
 const loops=[[outer],[hole]],before=structuredClone(loops)
 expect(inspectSweepContours(loops,options)).toMatchObject({capDomainCertified:true,capGeometryCertified:false,globalEmbeddingCertified:false,reason:null})
 const rounded=structuredClone(outer);rounded.weights[1]=2
 expect(inspectSweepContours([[rounded]],options)).toMatchObject({capDomainCertified:false,reason:'contour-simplicity-unproved'})
 expect(loops).toEqual(before)
})
it('proves the continuous planar domain without claiming cap geometry or shell containment',()=>{
 const loops=[square(0,0,10),square(1,1,2),square(6,6,2)],before=structuredClone(loops)
 expect(inspectSweepContours(loops,options)).toMatchObject({capDomainCertified:true,capGeometryCertified:false,globalEmbeddingCertified:false,planeAxis:2,reason:null})
 expect(loops).toEqual(before)
})
it('refuses outside, nested and touching holes and preserves independent zero budgets',()=>{
 const outer=square(0,0,10)
 for(const loops of [[outer,square(11,1,2)],[outer,square(1,1,4),square(2,2,1)],[outer,square(0,1,2)]]){
  expect(inspectSweepContours(loops,options).capDomainCertified).toBe(false)
 }
 for(const budget of ['maxPairs','maxCells'] as const){
  expect(inspectSweepContours([outer,square(1,1,2)],{...options,[budget]:0})).toMatchObject({capDomainCertified:false,reason:'contour-budget-exhausted'})
 }
 expect(()=>inspectSweepContours([outer],{...options,tolerance:-1})).toThrow()
 const nonplanar=structuredClone(outer);nonplanar[0]!.controlPoints[0]![2]=1
 expect(inspectSweepContours([nonplanar],options)).toMatchObject({capDomainCertified:false,planeAxis:null})
})
it('qualifies actual segmented circle primitives without rounded decomposition',async()=>{
 const {circleNurbsCurve}=await import('../src/services/nurbsConstructors')
 const loops=[10,2].map(radius=>[circleNurbsCurve([0,0,0],[0,0,1],radius)])
 expect(inspectSweepContours(loops,options).capDomainCertified).toBe(true)
})
