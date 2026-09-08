import {booleanPolygonMeshes} from '../src/services/polygonKernel'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/brepKernel'
import {expect,it} from 'vitest'
import {tessellateSubdivision} from '../src/services/subdivisionKernel'
import {evaluateSdf,tessellateSdf,type SdfField} from '../src/services/sdfKernel'
import {parseOpenSCAD} from '../src/services/openscadParser'
const cage={vertices:[[-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],[-1,-1,1],[1,-1,1],[1,1,1],[-1,1,1]],faces:[[0,3,2,1],[4,5,6,7],[0,1,5,4],[1,2,6,5],[2,3,7,6],[3,0,4,7]]}
it('refines a closed cube and retains six authored patches',()=>{
 const mesh=tessellateSubdivision(cage,2);expect(mesh.report.closed).toBe(true);expect(mesh.report.triangleCount).toBe(192);expect(new Set(mesh.faceIds).size).toBe(6);expect(mesh.report.signedVolumeMm3).toBeGreaterThan(0);expect(mesh.report.signedVolumeMm3).toBeLessThan(8)
})
it('extracts an oriented hollow solid and rejects unbounded extraction',()=>{
 const field:SdfField={kind:'difference',a:{kind:'sphere',center:[0,0,0],radius:1},b:{kind:'sphere',center:[0,0,0],radius:0.5}}
 expect(evaluateSdf(field,[0,0,0])).toBe(0.5)
 const mesh=tessellateSdf(field,{min:[-1.5,-1.5,-1.5],max:[1.5,1.5,1.5],cells:[20,20,20]});expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeCloseTo(4*Math.PI/3*(1-0.125),0)
 expect(()=>tessellateSdf(field,{min:[-0.8,-0.8,-0.8],max:[0.8,0.8,0.8],cells:[8,8,8]})).toThrow(/boundary/)
})
it('renders compact subdivision and smooth implicit geometry with physical units',async()=>{
 const sub=await parseOpenSCAD(`// @modelgraph-text/1\nshow subdivision(${JSON.stringify(cage.vertices)},${JSON.stringify(cage.faces)},2)`)
 expect(sub.meshes).toHaveLength(1);expect(sub.meshes[0].faceIdsAuthoritative).toBe(true)
 const sdf=await parseOpenSCAD('// @modelgraph-text/1\na=sdf_sphere([-0.5mm,0,0],1mm)\nb=sdf_sphere([0.5mm,0,0],1mm)\nshow sdf_smooth_union(a,b,radius:0.3mm) |> sdf_tessellate([-2mm,-2mm,-2mm],[2mm,2mm,2mm],[16,16,16])')
 expect(sdf.meshes).toHaveLength(1);expect(sdf.meshes[0].indices.length).toBeGreaterThan(0)
})
it('rejects exponential shared SDF expressions before serialization',()=>{
 let f:SdfField={kind:'sphere',center:[0,0,0],radius:1};for(let i=0;i<12;i++)f={kind:'union',a:f,b:f};expect(()=>evaluateSdf(f,[0,0,0])).toThrow(/256/)
})

it('combines subdivision, SDF and NURBS B-rep through the own polygon kernel',()=>{
 const sub=tessellateSubdivision(cage,1)
 const sdf=tessellateSdf({kind:'sphere',center:[3,0,0],radius:0.5},{min:[2,-1,-1],max:[4,1,1],cells:[8,8,8]})
 const brep=tessellateNurbsBrep(createBrepBox([5,0,0],[6,1,1]),1)
 const joined=booleanPolygonMeshes(booleanPolygonMeshes(sub,sdf,'union'),brep,'union')
 expect(joined.report.closed).toBe(true)
 expect(joined.report.signedVolumeMm3).toBeCloseTo(sub.report.signedVolumeMm3+sdf.report.signedVolumeMm3+1,8)
})
