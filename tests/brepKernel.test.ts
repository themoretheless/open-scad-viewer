import {expect,it} from 'vitest'
import {booleanNurbsBrep,chamferNurbsBrep,chamferNurbsBrepEdges,createBrepBox,extrudeBrepPolygon,filletNurbsBrep,filletNurbsBrepEdges,inspectNurbsBrep,tessellateNurbsBrep,nurbsBrepToPolygon,inspectPolygonBrep,tessellatePolygonBrep} from '../src/services/geometry/brep'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {withSelectionSurfaces} from '../src/services/meshSurfaceGroups'
import {transformSelection} from '../src/services/directSolidTools'
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
 const cavity=booleanNurbsBrep(createBrepBox([0,0,0],[4,4,4]),createBrepBox([1,1,1],[3,3,3]),'difference')
 expect(cavity.bodies[0].innerShells).toHaveLength(1)
 expect(tessellateNurbsBrep(cavity,2).report.signedVolumeMm3).toBeCloseTo(56,8)
 expect(()=>booleanNurbsBrep(cavity,a,'union')).toThrow(/cavit/i)
 expect(()=>filletNurbsBrep(a,0,2,8)).toThrow(/smaller|consumes/i)
})
it('constructs exact planar-profile B-reps and blends connected edge chains',()=>{
 const wedge=extrudeBrepPolygon([[0,0],[4,0],[0,3]],-1,2)
 expect([wedge.vertices.length,wedge.edges.length,wedge.faces.length,wedge.bodies.length]).toEqual([6,9,5,1])
 expect(tessellateNurbsBrep(wedge,2).report.signedVolumeMm3).toBeCloseTo(18,8)
 const box=createBrepBox([0,0,0],[10,10,10]),first=box.edges[0],connected=box.edges.findIndex((edge,index)=>index>0&&edge.vertices.some(vertex=>first.vertices.includes(vertex)))
 const chamfer=chamferNurbsBrepEdges(box,[0,connected],1),fillet=filletNurbsBrepEdges(box,[0,connected],1,4)
 expect(chamfer.faces.length).toBeGreaterThan(7);expect(fillet.faces.length).toBeGreaterThan(chamfer.faces.length)
 expect(tessellateNurbsBrep(fillet,2).report.closed).toBe(true)
 expect(()=>extrudeBrepPolygon([[0,0],[2,0],[1,1],[2,2],[0,2]],0,1)).toThrow(/convex/i)
})
it('preserves authored B-rep geometry through Solid transforms for rotated intersections',()=>{
 const brep=createBrepBox([-2,-1,-1],[2,1,1]),built=tessellateNurbsBrep(brep,1)
 const document=transformSelection({version:1,sketches:[],bodies:[{id:'b',name:'Box',brep,mesh:{positions:built.positions,indices:built.indices}}]},['b'],[0,0,0],[0,0,1],45,1)
 const rotated=document.bodies[0].brep!;expect(inspectNurbsBrep(rotated).topologyValid).toBe(true)
 const result=booleanNurbsBrep(createBrepBox([-2,-2,-1],[2,2,1]),rotated,'intersection')
 expect(result.faces.length).toBeGreaterThanOrEqual(8);expect(tessellateNurbsBrep(result,1).report.closed).toBe(true)
 expect(()=>booleanNurbsBrep(createBrepBox([-2,-2,-1],[2,2,1]),rotated,'union')).toThrow(/axis-aligned/i)
})
