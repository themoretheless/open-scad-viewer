import {expect,it} from 'vitest'
import {booleanNurbsBrep,chamferNurbsBrep,createBrepBox,filletNurbsBrep,inspectNurbsBrep,tessellateNurbsBrep,nurbsBrepToPolygon,inspectPolygonBrep,tessellatePolygonBrep} from '../src/services/geometry/brep'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {withSelectionSurfaces} from '../src/services/meshSurfaceGroups'
it('shares topology across own NURBS and polygon kernels with stable face groups',()=>{
 const model=createBrepBox([0,0,0],[2,3,4]);expect([model.vertices.length,model.edges.length,model.faces.length,model.bodies.length]).toEqual([8,12,6,1])
 expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 const mesh=tessellateNurbsBrep(model,3);expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeCloseTo(24,8)
 expect(new Set(mesh.faceIds).size).toBe(6)
 const polygon=nurbsBrepToPolygon(model,3);expect(inspectPolygonBrep(polygon).topologyValid).toBe(true);expect(polygon.faces).toHaveLength(6)
 expect(tessellatePolygonBrep(polygon).report.signedVolumeMm3).toBeCloseTo(24,8)
 model.edges[0].vertices[0]=999;expect(()=>inspectNurbsBrep(model)).toThrow()
})
it('publishes authored B-rep identities through the compact viewer path',async()=>{
 const scene=await parseOpenSCAD('// @modelgraph-text/1\nbody=brep_box([0,0,0],[2mm,3mm,4mm])\nshow body.brep_tessellate(3)')
 const mesh=scene.meshes[0];expect(mesh.faceIdsAuthoritative).toBe(true);expect(new Set(mesh.faceIds).size).toBe(6)
 expect(withSelectionSurfaces(mesh)).toBe(mesh)
})
it('runs fail-closed booleans and edge treatments on manifold NURBS B-reps',()=>{
 const a=createBrepBox([0,0,0],[2,2,2]),b=createBrepBox([1,1,0],[3,3,2])
 for(const operation of ['union','difference','intersection'] as const){
  const result=booleanNurbsBrep(a,b,operation),mesh=tessellateNurbsBrep(result,2)
  expect(inspectNurbsBrep(result).topologyValid).toBe(true);expect(mesh.report.closed).toBe(true)
 }
 const chamfer=chamferNurbsBrep(a,0,.25),fillet=filletNurbsBrep(a,0,.25,8)
 expect(chamfer.faces).toHaveLength(7);expect(fillet.faces).toHaveLength(13)
 expect(tessellateNurbsBrep(fillet,2).report.closed).toBe(true)
 const disconnected=booleanNurbsBrep(a,createBrepBox([4,0,0],[5,1,1]),'union')
 expect(disconnected.bodies).toHaveLength(2)
 expect(tessellateNurbsBrep(disconnected,2).report.closed).toBe(true)
 expect(booleanNurbsBrep(disconnected,createBrepBox([0,0,0],[1,1,1]),'intersection').bodies).toHaveLength(1)
 expect(()=>filletNurbsBrep(a,0,2,8)).toThrow(/smaller|consumes/i)
})
