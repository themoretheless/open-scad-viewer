import {expect,it} from 'vitest'
import {booleanNurbsBrep,chamferNurbsBrep,chamferNurbsBrepEdges,createBrepBox,createFacetedBrepCylinder,createFacetedBrepLoft,createFacetedBrepRevolve,createFacetedBrepSphere,createFacetedBrepSweep,extrudeBrepPolygon,filletNurbsBrep,filletNurbsBrepEdges,inspectNurbsBrep,tessellateNurbsBrep,nurbsBrepToPolygon,inspectPolygonBrep,tessellatePolygonBrep} from '../src/services/geometry/brep'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {withSelectionSurfaces} from '../src/services/meshSurfaceGroups'
import {transformSelection} from '../src/services/directSolidTools'
it('shares topology across own NURBS and polygon kernels with stable face groups',()=>{
 const model=createBrepBox([0,0,0],[2,3,4]);expect([model.vertices.length,model.edges.length,model.faces.length,model.bodies.length]).toEqual([8,12,6,1])
 expect(model.topologyIds?.vertices).toHaveLength(8);expect(new Set(model.topologyIds?.faces).size).toBe(6)
 expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 const mesh=tessellateNurbsBrep(model,3);expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeCloseTo(24,8)
 expect(new Set(mesh.faceIds).size).toBe(6)
 expect(new Set(mesh.topologyFaceIds).size).toBe(6)
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
 const filled=booleanNurbsBrep(cavity,createBrepBox([1,1,1],[3,3,3]),'union')
 expect(filled.bodies[0].innerShells).toHaveLength(0)
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
 const concave=extrudeBrepPolygon([[0,0],[5,0],[5,2],[3,2],[3,5],[0,5]],0,2)
 expect(tessellateNurbsBrep(concave,2).report.signedVolumeMm3).toBeCloseTo(38,7)
 const trimmed=extrudeBrepPolygon([[0,0],[5,0],[5,5],[0,5]],0,2,[[[1,1],[1,2],[2,2],[2,1]]])
 expect(trimmed.faces.filter(face=>face.holes.length)).toHaveLength(2)
 expect(tessellateNurbsBrep(trimmed,2).report.signedVolumeMm3).toBeCloseTo(48,7)
})
it('preserves authored B-rep geometry through Solid transforms for rotated booleans',()=>{
 const brep=createBrepBox([-2,-1,-1],[2,1,1]),built=tessellateNurbsBrep(brep,1)
 const document=transformSelection({version:1,sketches:[],bodies:[{id:'b',name:'Box',brep,mesh:{positions:built.positions,indices:built.indices}}]},['b'],[0,0,0],[0,0,1],45,1)
 const rotated=document.bodies[0].brep!;expect(inspectNurbsBrep(rotated).topologyValid).toBe(true)
 const stock=createBrepBox([-2,-2,-1],[2,2,1])
 for(const operation of ['union','difference','intersection'] as const){
  const result=booleanNurbsBrep(stock,rotated,operation)
  expect(result.faces.length).toBeGreaterThanOrEqual(8);expect(tessellateNurbsBrep(result,1).report.closed).toBe(true)
 }
})
it('keeps persistent topology IDs for unchanged Boolean entities',()=>{
 const stock=createBrepBox([0,0,0],[3,2,2]),cutter=createBrepBox([2,0,0],[4,2,2])
 const result=booleanNurbsBrep(stock,cutter,'difference')
 expect(result.topologyIds?.vertices.filter(id=>stock.topologyIds?.vertices.includes(id)).length).toBeGreaterThanOrEqual(4)
 expect(result.topologyIds?.edges.filter(id=>stock.topologyIds?.edges.includes(id)).length).toBeGreaterThanOrEqual(4)
 expect(result.topologyIds?.faces.some(id=>stock.topologyIds?.faces.includes(id))).toBe(true)
 const split=booleanNurbsBrep(stock,createBrepBox([1,-1,-1],[2,3,3]),'difference')
 expect(split.topologyIds?.lineage?.some(record=>record.operation==='split'&&record.children.length>1)).toBe(true)
 const mesh=tessellateNurbsBrep(result,1)
 expect(mesh.topologyFaceIds).toEqual(mesh.faceIds.map(face=>result.topologyIds!.faces[face]))
})
it('constructs explicitly faceted round primitives as manifold B-reps',()=>{
 const cylinder=createFacetedBrepCylinder(2,5,16),sphere=createFacetedBrepSphere(2,16,8)
 expect(cylinder.faces).toHaveLength(18);expect(sphere.faces).toHaveLength(224)
 expect(tessellateNurbsBrep(cylinder,1).report.closed).toBe(true)
 expect(tessellateNurbsBrep(sphere,1).report.closed).toBe(true)
})
it('constructs loft, polyline sweep and full revolve as honest faceted B-reps',()=>{
 const loft=createFacetedBrepLoft([[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]],[[-1,-1,3],[1,-1,3],[1,1,3],[-1,1,3]]])
 const sweep=createFacetedBrepSweep([[-1,-1],[1,-1],[1,1],[-1,1]],[[0,0,0],[0,0,3],[2,0,5]],[0,1,0])
 const revolve=createFacetedBrepRevolve([[0,-2],[2,-2],[2,2],[0,2]],16)
 for(const model of [loft,sweep,revolve]){
  expect(inspectNurbsBrep(model).topologyValid).toBe(true)
  expect(tessellateNurbsBrep(model,1).report.closed).toBe(true)
 }
 expect(loft.faces).toHaveLength(10);expect(sweep.faces).toHaveLength(18)
 expect(()=>createFacetedBrepLoft([[[-1,-1,0],[1,-1,0],[0,0,0],[-1,1,0]],[[-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1]]])).toThrow(/convex/i)
})
it('lofts retained parallel sections in an oblique translated frame',()=>{
 const c=Math.SQRT1_2,place=([x,y,z]:number[]):[number,number,number]=>[c*x+c*z+5,y-3,-c*x+c*z+7]
 const sections=[[[-2,-2,0],[2,-2,0],[2,2,0],[-2,2,0]],[[-1,-1,3],[1,-1,3],[1,1,3],[-1,1,3]]].map(s=>s.map(place)),before=JSON.stringify(sections)
 const model=createFacetedBrepLoft(sections),mesh=tessellateNurbsBrep(model,1)
 expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeCloseTo(28,8)
 expect(model.faces).toHaveLength(10)
 const bad=structuredClone(sections);bad[1][0][2]+=.1
 expect(()=>createFacetedBrepLoft(bad)).toThrow('parallel')
 expect(JSON.stringify(sections)).toBe(before)
})
it('refuses star and repeated-loop profiles before loft or sweep publication',()=>{
 const ring=Array.from({length:5},(_,i):[number,number]=>[Math.cos(i*2*Math.PI/5),Math.sin(i*2*Math.PI/5)])
 for(const profile of [[0,2,4,1,3].map(i=>ring[i]),[...ring,...ring]]){
  const before=JSON.stringify(profile)
  const sections=[0,3].map(z=>profile.map(([x,y]):[number,number,number]=>[x,y,z]))
  expect(()=>createFacetedBrepLoft(sections)).toThrow('convex')
  expect(()=>createFacetedBrepSweep(profile,[[0,0,0],[0,0,3]],[0,1,0])).toThrow('convex')
  expect(JSON.stringify(profile)).toBe(before)
 }
 const valid=createFacetedBrepSweep(ring,[[0,0,0],[0,0,3]],[0,1,0])
 expect(tessellateNurbsBrep(valid,1).report.closed).toBe(true)
})
