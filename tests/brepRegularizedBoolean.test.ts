import {expect,it} from 'vitest'
import {analyzeNurbsBrep,booleanNurbsBrep,createBrepBox,createBrepCylinder,createBrepTube,extrudeBrepCurves,inspectNurbsBrep,nurbsBrepToPolygon,tessellateNurbsBrep,transformNurbsBrep} from '../src/services/geometry/brep'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {GeometryKernelError} from '../src/services/geometry/kernel'

it('rejects malformed legacy B-rep documents without trapping the shared WASM instance',()=>{
 const legacy=createBrepBox([0,0,0],[2,2,2]);delete legacy.topologyIds
 const badIndex=structuredClone(legacy);badIndex.shells[0].faces[0].face=999
 const badWeights=structuredClone(legacy);badWeights.faces[0].surface.weights=[]
 const badKnots=structuredClone(legacy);badKnots.edges[0].curve.knots=[]
 for(const bad of [badIndex,badWeights,badKnots]){
  expect(()=>inspectNurbsBrep(bad)).toThrow(GeometryKernelError)
  expect(inspectNurbsBrep(legacy).topologyValid).toBe(true)
 }
})

it('extrudes a retained multi-span rational circle into independently sampled faces',()=>{
 const curve={degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],controlPoints:[[2,0],[2,2],[0,2],[-2,2],[-2,0],[-2,-2],[0,-2],[2,-2],[2,0]],weights:[1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1]}
 const body=extrudeBrepCurves([[curve]],-2,3)
 expect(body.faces).toHaveLength(6)
 expect(tessellateNurbsBrep(body,1).report.closed).toBe(true)
 expect(analyzeNurbsBrep(body).signedVolumeMm3).toBeCloseTo(20*Math.PI,6)
})

it('keeps empty CSG valid through transport, tessellation and later operations',()=>{
 const box=createBrepBox([0,0,0],[2,2,2])
 const empty=JSON.parse(JSON.stringify(booleanNurbsBrep(box,box,'difference')))
 expect(inspectNurbsBrep(empty).topologyValid).toBe(true)
 expect(empty.bodies).toEqual([]);expect(empty.faces).toEqual([])
 const mesh=tessellateNurbsBrep(empty,4)
 expect(mesh.indices).toEqual([]);expect(mesh.positions).toEqual([])
 expect(mesh.report.closed).toBe(false)
 expect(mesh.faceIds).toEqual([])
 const polygons=nurbsBrepToPolygon(empty,4)
 expect(polygons.bodies).toEqual([]);expect(polygons.shells).toEqual([])
 expect(()=>analyzeNurbsBrep(empty)).toThrow(/closed|empty|body|bodies/i)
 expect(booleanNurbsBrep(empty,box,'union').topologyIds).toEqual(box.topologyIds)
})

it('keeps rational circle Boolean geometry while display detail changes',()=>{
 const a=createBrepCylinder(3,5)
 const b=transformNurbsBrep(createBrepCylinder(3,5),[[1,0,0,2],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 const disk=Math.PI*9,lens=18*Math.acos(1/3)-2*Math.sqrt(8)
 for(const [operation,area] of [['intersection',lens],['difference',disk-lens],['union',2*disk-lens],['xor',2*(disk-lens)]] as const){
  const result=booleanNurbsBrep(a,b,operation)
  const geometry=JSON.stringify(result)
  expect(analyzeNurbsBrep(result).signedVolumeMm3).toBeCloseTo(area*5,6)
  expect(result.edges.some(edge=>edge.curve.degree===2)).toBe(true)
  for(const detail of [2,4,16]){
   const mesh=tessellateNurbsBrep(result,detail)
   expect(mesh.report.closed).toBe(true)
   expect(mesh.report.nonManifoldEdges).toBe(0)
   expect(mesh.report.orientationConflicts).toBe(0)
  }
  expect(JSON.stringify(result)).toBe(geometry)
 }
 const ring=booleanNurbsBrep(a,createBrepCylinder(1,5),'difference')
 expect(ring.faces.filter(f=>f.holes.length)).toHaveLength(2)
 expect(analyzeNurbsBrep(ring).signedVolumeMm3).toBeCloseTo(analyzeNurbsBrep(createBrepTube(3,1,5)).signedVolumeMm3,6)
})

it('exposes curved Boolean and XOR through ModelGraph text and keeps empty bounds null',async()=>{
 const scene=await parseOpenSCAD('// @modelgraph-text/1\na=brep_cylinder(3mm,5mm)\nb=brep_cylinder(1mm,5mm)\nshow a.brep_xor(b).brep_tessellate(8)')
 expect(scene.meshes).toHaveLength(1)
 expect(scene.meshes[0].faceIdsAuthoritative).toBe(true)
 const result=buildOwnNurbs({language:'modelgraph/nurbs-1',units:'mm',nodes:[{id:'a',op:'brep_cylinder',radius:3,height:5},{id:'zero',op:'brep_boolean',inputs:['a','a'],operation:'difference'},{id:'display',op:'brep_tessellate',input:'zero',segments:4}],root:'display'},{action:'build'})!
 expect(result.report.bounds).toBeNull()
 expect(result.mesh?.indices).toEqual([])
})

it('retains separate rational solids after cutting an axial interval at arbitrary orientation',()=>{
 const angle=.47,c=Math.cos(angle),s=Math.sin(angle)
 const pose=[[1,0,0,7],[0,c,-s,-3],[0,s,c,12],[0,0,0,1]]
 const stock=transformNurbsBrep(createBrepCylinder(3,10),pose)
 const cutter=transformNurbsBrep(transformNurbsBrep(createBrepCylinder(3,2),[[1,0,0,0],[0,1,0,0],[0,0,1,4],[0,0,0,1]]),pose)
 for(const operation of ['difference','xor'] as const){
  const result=booleanNurbsBrep(stock,cutter,operation)
  expect(result.bodies).toHaveLength(2)
  expect(result.edges.some(edge=>edge.curve.degree===2)).toBe(true)
  expect(tessellateNurbsBrep(result,8).report.closed).toBe(true)
  expect(analyzeNurbsBrep(result).signedVolumeMm3).toBeCloseTo(72*Math.PI,6)
 }
})
