import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {createG2SurfaceBridge,inspectBrepGeometricG2,createG2AnnularTube} from '../src/services/geometricG2'
import {createBrepBox} from '../src/services/geometry/brep'
import type {NurbsSurface} from '../src/services/nurbsSurface'
beforeAll(async()=>{await warmGeometryKernel()})
const support=(offset:number):NurbsSurface=>({degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],
 controlPoints:[[0,0],[1,.25],[2,0]].map(([x,y])=>[[x,offset+y,0],[x,offset+y+1,0]]),weights:[[1,1],[.5,.5],[1,1]]})
it('qualifies both complete rational curved support edges in actual WASM',()=>{
 const a=support(0),b=support(3),before=structuredClone([a,b])
 const result=createG2SurfaceBridge(a,b,{boundaries:['vMax','vMin'],gains:[.5,.5]})
 expect(result.patches).toHaveLength(1)
 expect(result.report).toMatchObject({complete:true,withinTolerance:true,exactIdentityClaim:false,solidGeometryCertified:false})
 expect(result.report.supportSeams).toHaveLength(2)
 for(const seam of result.report.supportSeams)expect(seam).toMatchObject({sourceRange:[0,1],inspection:{decision:'within-tolerance',regularityProven:true}})
 expect([a,b]).toEqual(before)
 const refused=createG2SurfaceBridge(a,b,{boundaries:['vMax','vMin'],gains:[.5,.5],maxCells:0})
 expect(refused.report).toMatchObject({complete:false,withinTolerance:false,cells:0})
})
it('does not certify a complex corner from a partial edge selection or zero work',()=>{
 const model=createBrepBox([0,0,0],[10,10,10]),before=structuredClone(model)
 const partial=inspectBrepGeometricG2(model,[0],{positionTolerance:1e-7,normalTolerance:1e-7,curvatureTolerance:1e-5,maxCells:0})
 expect(partial.corners?.length).toBeGreaterThan(0)
 for(const c of partial.corners!)expect(c).toMatchObject({closedLink:true,allIncidentSeamsWithinTolerance:false,solidGeometryCertified:false})
 expect(partial.corners![0].omittedEdges).toHaveLength(2)
 expect(model).toEqual(before)
})

it('tracks all four incident edges of a rational corner without hiding omissions',()=>{
 const {model}=createG2AnnularTube(10,7,20,.1)
 const before=structuredClone(model),options={positionTolerance:1e-7,normalTolerance:1e-7,curvatureTolerance:1e-5,maxCells:0}
 const partial=inspectBrepGeometricG2(model,[0],options),corner=partial.corners!.find(c=>c.edges.length===4)!
 expect(corner).toBeDefined();expect(corner.closedLink).toBe(true);expect(corner.omittedEdges).toHaveLength(3)
 const complete=inspectBrepGeometricG2(model,corner.edges,options).corners!.find(c=>c.vertex===corner.vertex)!
 expect(complete).toMatchObject({omittedEdges:[],closedLink:true,allIncidentSeamsWithinTolerance:false,solidGeometryCertified:false})
 expect(model).toEqual(before)
})
