import {expect,it} from 'vitest'
import {createBrepBox,inspectNurbsBrep,tessellateNurbsBrep,nurbsBrepToPolygon,inspectPolygonBrep,tessellatePolygonBrep} from '../src/services/brepKernel'
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
