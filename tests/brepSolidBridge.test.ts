import { stringifyMeshJson } from '../src/services/meshJson'
import {expect,it} from 'vitest'
import {EXAMPLES} from '../src/data/examples'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {sceneMeshesToSolidDocument} from '../src/services/solidBridge'
import {parseDirectDocument} from '../src/services/directModeling'
import {analyzeNurbsBrep, booleanNurbsBrep, tessellateNurbsBrep, type NurbsBrep} from '../src/services/geometry/brep'
import {identity,rotateY,scale,translate} from '../src/services/math3d'

async function cylinder() {
  return (await parseOpenSCAD('// @modelgraph-text/1\nshow brep_cylinder(3mm,4mm).brep_tessellate(6)')).meshes[0]
}

it('retains the complete authored enclosure through the actual Code-to-Solid conversion and persistence',async()=>{
  const scene=await parseOpenSCAD(EXAMPLES['brep-enclosure'])
  const source=scene.meshes[0].nativeGeometry!
  const expected=JSON.parse(source.geometryJson).geometry as NurbsBrep
  const before=JSON.stringify(scene.meshes[0])
  const solid=sceneMeshesToSolidDocument(scene.meshes)
  expect(solid.bodies).toHaveLength(1)
  const body=solid.bodies[0]
  expect(body.brep).toEqual(expected)
  expect(body.brep).not.toBe(expected)
  expect(body.brep!.faces).toHaveLength(48)
  expect(body.brep!.edges.some(edge=>edge.curve.degree===2)).toBe(true)
  expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(7619.809938003907,6)
  expect(parseDirectDocument(stringifyMeshJson(solid)).bodies[0].brep).toEqual(expected)
  expect(tessellateNurbsBrep(body.brep!,4).report).toMatchObject({closed:true,boundaryEdges:0,nonManifoldEdges:0})
  expect(JSON.stringify(scene.meshes[0])).toBe(before)
},15000)

it('applies reflected scene placement to both retained f64 surfaces and outward display triangles',async()=>{
  const source=await cylinder()
  const mesh=structuredClone(source)
  mesh.transform=translate(scale(identity(),[-2,3,.5]),[10,20,-4])
  const body=sceneMeshesToSolidDocument([mesh]).bodies[0]
  const mass=analyzeNurbsBrep(body.brep!)
  expect(mass.signedVolumeMm3).toBeCloseTo(108*Math.PI,6)
  mass.centroid.forEach((v,i)=>expect(v).toBeCloseTo([10,20,-3][i],8))
  const axes=[0,1,2].map(axis=>body.mesh.positions.filter((_,i)=>i%3===axis))
  expect(axes.map(axis=>Math.min(...axis))).toEqual([4,11,-4])
  expect(axes.map(axis=>Math.max(...axis))).toEqual([16,29,-2])
  let signedMeshVolume=0
  const points=(i:number)=>body.mesh.positions.slice(i*3,i*3+3)
  for(let i=0;i<body.mesh.indices.length;i+=3) {
    const [a,b,c]=Array.from(body.mesh.indices.slice(i,i+3),points)
    signedMeshVolume+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6
  }
  expect(signedMeshVolume).toBeGreaterThan(0)
  expect(booleanNurbsBrep(body.brep!,structuredClone(body.brep!),'difference').bodies).toEqual([])
  expect(source.nativeGeometry).toEqual(mesh.nativeGeometry)
  const plain=structuredClone(mesh); delete plain.nativeGeometry
  const polygon=sceneMeshesToSolidDocument([plain]).bodies[0]
  expect(polygon).not.toHaveProperty('brep')
  expect(polygon.mesh).toEqual(body.mesh)
})

it('uses the renderer row-major contract for non-symmetric rotation and refuses projective placement',async()=>{
  const mesh=await cylinder()
  mesh.transform=rotateY(identity(),Math.PI/2)
  const body=sceneMeshesToSolidDocument([mesh]).bodies[0]
  const mass=analyzeNurbsBrep(body.brep!)
  mass.centroid.forEach((v,i)=>expect(v).toBeCloseTo([2,0,0][i],8))
  expect(mass.signedVolumeMm3).toBeCloseTo(36*Math.PI,6)
  const xs=body.mesh.positions.filter((_,i)=>i%3===0)
  expect(Math.min(...xs)).toBeCloseTo(0,8)
  expect(Math.max(...xs)).toBeCloseTo(4,8)
  mesh.transform[12]=1
  expect(()=>sceneMeshesToSolidDocument([mesh])).toThrow(/finite affine scene transform/)
  mesh.transform=scale(identity(),[1,1,0])
  expect(()=>sceneMeshesToSolidDocument([mesh])).toThrow(/singular scene transform/)
  delete mesh.nativeGeometry
  expect(()=>sceneMeshesToSolidDocument([mesh])).toThrow(/singular scene transform/)
})

it('refuses corrupt or empty native authority atomically instead of silently downgrading it to a mesh',async()=>{
  const mesh=await cylinder()
  const corrupt=structuredClone(mesh)
  corrupt.nativeGeometry={...corrupt.nativeGeometry!,revision:'0'.repeat(64)}
  expect(()=>sceneMeshesToSolidDocument([mesh,corrupt])).toThrow(/Invalid native B-rep snapshot/)
  const original=JSON.parse(mesh.nativeGeometry!.geometryJson).geometry as NurbsBrep
  const empty=booleanNurbsBrep(original,original,'difference')
  const stale=structuredClone(mesh)
  stale.nativeGeometry=createNativeGeometryArtifact('empty','brep',{geometry:empty},{})
  expect(()=>sceneMeshesToSolidDocument([stale])).toThrow(/empty B-rep cannot have a displayed/)
  stale.vertices=new Float32Array(); stale.indices=new Uint32Array()
  expect(sceneMeshesToSolidDocument([stale]).bodies).toEqual([])
  const missing=structuredClone(mesh); missing.vertices=new Float32Array(); missing.indices=new Uint32Array()
  expect(()=>sceneMeshesToSolidDocument([missing])).toThrow(/nonempty B-rep requires a display mesh/)
})

it('preserves coordinates and rational weights beyond display Float32 precision',async()=>{
  const mesh=(await parseOpenSCAD('// @modelgraph-text/1\nshow brep_cylinder(3.141592653589793mm,2.718281828459045mm).brep_tessellate(3)')).meshes[0]
  const native=JSON.parse(mesh.nativeGeometry!.geometryJson).geometry as NurbsBrep
  const body=sceneMeshesToSolidDocument([mesh]).bodies[0]
  expect(body.brep).toEqual(native)
  expect(body.brep!.vertices.some(vertex=>vertex.point.some(v=>v!==Math.fround(v)))).toBe(true)
  expect(body.brep!.edges.some(edge=>edge.curve.weights.some(w=>w!==1))).toBe(true)
  expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(Math.PI**3*Math.E,6)
})
