import {beforeAll,expect,it} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {assembleClosedNurbsPatches,createG2AnnularTube} from '../src/services/geometricG2'
import {createBrepBox} from '../src/services/geometry/brep'
import type {NurbsBrep} from '../src/services/geometry/brep'
beforeAll(async()=>{await warmGeometryKernel()})
function patches(model:NurbsBrep){const reversed=model.faces.map(()=>false);for(const use of model.shells[0].faces)reversed[use.face]=use.reversed;return {surfaces:model.faces.map(f=>structuredClone(f.surface)),reversed}}
it('sews independent rounded support and end patches into one admitted body',()=>{
 const source=createG2AnnularTube(10,7,20,.1).model,before=structuredClone(source),input=patches(source)
 const result=assembleClosedNurbsPatches(input.surfaces,input.reversed,{sources:[source]})
 expect(result.report).toMatchObject({solidGeometryCertified:true,fullBoundaryG2Certified:true,faces:32,edges:64,directlyOwnedUses:64,totalPairs:496,coveredPairs:496,nextPair:null})
 expect(result.model.bodies).toHaveLength(1);expect(result.model.shells[0].closed).toBe(true)
 expect(source).toEqual(before)
})
it('refuses missing end patches, duplicate ownership, a zero ledger and sharp G2 claims',()=>{
 const source=createBrepBox([0,0,0],[10,10,10]),before=structuredClone(source),p=patches(source)
 for(const request of [()=>assembleClosedNurbsPatches(p.surfaces.slice(0,5),p.reversed.slice(0,5)),()=>assembleClosedNurbsPatches([...p.surfaces,p.surfaces[0]],[...p.reversed,p.reversed[0]]),()=>assembleClosedNurbsPatches(p.surfaces,p.reversed,{maxCorrespondenceWork:0}),()=>assembleClosedNurbsPatches(p.surfaces,p.reversed)])expect(request).toThrow()
 const other=patches(createBrepBox([10,10,10],[20,20,20]))
 expect(()=>assembleClosedNurbsPatches([...p.surfaces,...other.surfaces],[...p.reversed,...other.reversed],{requireFullG2:false})).toThrow(/face fans/)
 const solid=assembleClosedNurbsPatches(p.surfaces,p.reversed,{requireFullG2:false})
 expect(solid.report).toMatchObject({solidGeometryCertified:true,fullBoundaryG2Certified:false})
 expect(source).toEqual(before)
})
it('retains four independent end-transition extents in the actual admitted solid',()=>{
 const extents:[number,number,number,number]=[.1,.25,.5,.75]
 const result=createG2AnnularTube(10,7,20,.5,extents)
 expect(result.report.transitionSizes).toEqual(extents)
 expect(result.report.assembly).toMatchObject({method:'exact-owned-patch-network/1',directlyOwnedUses:64})
 expect(result.report).toMatchObject({fullBoundaryG2Certified:true,solidGeometryCertified:true,totalPairs:496,coveredPairs:496,nextPair:null})
 expect(()=>createG2AnnularTube(10,7,20,.5,[2.5,.25,.5,.75])).toThrow()
})
it('admits a closed network with noncircular rational edges through actual WASM',()=>{
 const source=createG2AnnularTube(8,4,16,.5).model,p=patches(source)
 for(const surface of p.surfaces)for(const row of surface.controlPoints)for(const point of row){point[0]*=2;point[1]*=.5}
 const result=assembleClosedNurbsPatches(p.surfaces,p.reversed)
 expect(result.report).toMatchObject({fullBoundaryG2Certified:true,solidGeometryCertified:true,edges:64,totalPairs:496,coveredPairs:496,nextPair:null})
 const points=result.model.vertices.map(v=>v.point)
 expect([0,1,2].map(i=>[Math.min(...points.map(p=>p[i])),Math.max(...points.map(p=>p[i]))])).toEqual([[-16,16],[-4,4],[0,16]])
})
