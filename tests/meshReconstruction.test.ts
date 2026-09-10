import {expect,it} from 'vitest'
import {inspectNurbsBrep,createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {meshToNurbsBrep,meshToNurbs,meshToSdf,meshToSubdivision,tessellateNurbsPatches} from '../src/services/geometry/reconstruction'
import {evaluateSdf,tessellateSdf} from '../src/services/geometry/sdf'
import {tessellateSubdivision} from '../src/services/geometry/subdivision'
import {parseOpenSCAD} from '../src/services/openscadParser'
const cube=()=>tessellateNurbsBrep(createBrepBox([-1,-1,-1],[1,1,1]),1)
it('reconstructs exact NURBS patches and bounded smoothed patches through WASM',()=>{
 const mesh=cube(),exact=meshToNurbs(mesh)
 expect(exact.patches).toHaveLength(mesh.indices.length/3);expect(exact.sampledMaxDeviationMm).toBeLessThan(1e-9)
 const roundtrip=tessellateNurbsPatches(exact,2);expect(roundtrip.report.closed).toBe(true);expect(roundtrip.report.signedVolumeMm3).toBeCloseTo(8,8)
 expect(()=>meshToNurbs(mesh,'point_normal',0)).toThrow(/deviation/)
 const smooth=meshToNurbs(mesh,'point_normal',1);expect(smooth.patches[0].degreeU).toBe(3);expect(smooth.errorBoundCertified).toBe(false);expect(tessellateNurbsPatches(smooth,2).report.closed).toBe(true)
})
it('constructs signed mesh distance, composes it with SDF, and resamples a closed result',()=>{
 const field=meshToSdf(cube());expect(evaluateSdf(field,[0,0,0])).toBeCloseTo(-1,10);expect(evaluateSdf(field,[2,0,0])).toBeCloseTo(1,10)
 const result=tessellateSdf({kind:'offset',input:field,distance:0.1},{min:[-1.5,-1.5,-1.5],max:[1.5,1.5,1.5],cells:[12,12,12]})
 expect(result.report.closed).toBe(true);expect(result.report.signedVolumeMm3).toBeGreaterThan(8)
 const sheet={positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]};expect(()=>meshToSdf(sheet)).toThrow();expect(evaluateSdf(meshToSdf(sheet,false),[0,0,2])).toBe(2)
 expect(()=>tessellateSdf(field,{min:[-2,-2,-2],max:[2,2,2],cells:[65,8,8]})).toThrow()
})
it('fits subdivision controls and reports actual sampled deviation',()=>{
 const fitted=meshToSubdivision(cube(),16)
 expect(fitted.vertexResidualAfterMm).toBeLessThan(fitted.vertexResidualBeforeMm*0.05)
 expect(fitted.deviation.sampleCount).toBeGreaterThan(0);expect(fitted.deviation.errorBoundCertified).toBe(false)
 expect(tessellateSubdivision(fitted.cage,1).report.closed).toBe(true)
})
it('supports explicit reverse-conversion pipelines in compact text',async()=>{
 const base='// @modelgraph-text/1\nmesh=brep_box([-1,-1,-1],[1,1,1]).brep_tessellate(1)\n'
 for(const pipe of ['mesh_to_nurbs_brep().brep_tessellate(2)','mesh_to_nurbs().nurbs_patches_tessellate(2)','mesh_fit_nurbs(1mm).nurbs_patches_tessellate(2)','mesh_to_subdivision(8).subdivision_tessellate(1)','mesh_to_sdf().sdf_tessellate([-1.5,-1.5,-1.5],[1.5,1.5,1.5],[8,8,8])']){
  const scene=await parseOpenSCAD(base+'show mesh.'+pipe);expect(scene.meshes).toHaveLength(1);expect(scene.meshes[0].indices.length).toBeGreaterThan(0)
 }
 const direct=await parseOpenSCAD('// @modelgraph-text/1\nshow triangle_mesh([[0,0,0],[1,0,0],[0,1,0]],[[0,1,2]]).mesh_to_nurbs().nurbs_patches_tessellate(2)');expect(direct.meshes).toHaveLength(1)
})
it('welds duplicated triangle positions before reverse conversion and rejects degenerate sources',()=>{
 const mesh=cube();const positions=mesh.indices.flatMap(i=>mesh.positions.slice(i*3,i*3+3));const soup={positions,indices:mesh.indices.map((_,i)=>i)}
 expect(evaluateSdf(meshToSdf(soup),[0,0,0])).toBeCloseTo(-1,10)
 expect(()=>meshToNurbs({positions:[0,0,0,1,0,0,2,0,0],indices:[0,1,2]})).toThrow(/degenerate/)
})

it('rebuilds shared NURBS B-rep incidence from mesh triangles',()=>{
 const mesh=cube();const model=meshToNurbsBrep(mesh);expect(model.faces).toHaveLength(mesh.indices.length/3);expect(model.bodies).toHaveLength(1);expect(inspectNurbsBrep(model).topologyValid).toBe(true)
 const result=tessellateNurbsBrep(model,2);expect(result.report.closed).toBe(true);expect(result.report.signedVolumeMm3).toBeCloseTo(8,8)
})

it('keeps planar point-normal reconstruction planar and tolerates translated coordinates',()=>{
 const sheet={positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]};expect(meshToNurbs(sheet,'point_normal',0).sampledMaxDeviationMm).toBeLessThan(1e-12)
 const m=cube();const moved={positions:m.positions.map(v=>100000+v*0.01),indices:m.indices};const patches=meshToNurbs(moved);const result=tessellateNurbsPatches(patches,2);expect(result.report.closed).toBe(true);expect(result.report.signedVolumeMm3).toBeCloseTo(0.000008,10)
})
