import {expect,it} from 'vitest'
import {bezierNurbsCurve,inspectProgressiveSweepCapProjection} from '../src/services/nurbsConstructors'
import type {NurbsSurface} from '../src/services/nurbsSurface'

const points=[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]]
const profiles=points.map((p,i)=>bezierNurbsCurve([p,points[(i+1)%4]!]))
const path=bezierNurbsCurve([[0,0,0],[0,0,10]])
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
const options={orientation:'rmf' as const,normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:33,maxDeviation:.01}
const budgets={tolerance:1e-9,maxPairs:10000,maxCells:100000,maxExactWork:1000000}
const plane=(z:number):NurbsSurface=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,z],[0,1,z]],[[1,0,z],[1,1,z]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false})
const inspect=(caps:[NurbsSurface,NurbsSurface],limits=budgets)=>inspectProgressiveSweepCapProjection(profiles,path,scale,twist,options,[4],caps,limits)

it('certifies plane projection and reflection without claiming material ownership for different cap regions',()=>{
 const caps:[NurbsSurface,NurbsSurface]=[plane(0),plane(10)]
 caps[1].controlPoints.reverse()
 expect(inspect(caps)).toMatchObject({capProjectionCertified:true,idealCapDomainsCertified:true,reversesOrientation:[false,true],
  method:'original-progressive-endpoint-plane-projection',continuousBound:false,retainedCapRegionsCertified:false,globalEmbeddingCertified:false,solidCertified:false})
})
it('discards both endpoint projections when the shared cell or exact-work budget is incomplete',()=>{
 const caps:[NurbsSurface,NurbsSurface]=[plane(0),plane(10)]
 const proof=inspect(caps)
 expect(proof.capProjectionCertified).toBe(true)
 for(const limits of [{...budgets,maxCells:proof.cells-1},{...budgets,maxExactWork:proof.exactWork-1}]){
  expect(inspect(caps,limits)).toMatchObject({capProjectionCertified:false,normalDots:null,reversesOrientation:null})
 }
})
it('refuses singular and nonplanar retained caps and rejects malformed endpoint cardinality',()=>{
 const singular=plane(10)
 singular.controlPoints=[[[0,0,0],[0,1,0]],[[0,0,1],[0,1,1]]]
 expect(inspect([plane(0),singular]).capProjectionCertified).toBe(false)
 const warped=plane(10)
 warped.controlPoints[1]![1]![2]!+=.01
 expect(inspect([plane(0),warped]).capProjectionCertified).toBe(false)
 expect(()=>inspect([plane(0)] as unknown as [NurbsSurface,NurbsSurface])).toThrow()
})
