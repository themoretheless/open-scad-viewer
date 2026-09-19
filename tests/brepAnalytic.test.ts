import {expect,it} from 'vitest'
import {createBrepCylinder,createBrepFrustum,createBrepTube,createBrepTorus,inspectNurbsBrep,analyzeNurbsBrep,tessellateNurbsBrep,booleanNurbsBrep,createBrepBox,extrudeBrepPolygon} from '../src/services/geometry/brep'
import {transformSelection} from '../src/services/directSolidTools'

it('tessellates exact curved solids without open seams across display resolutions',()=>{
 for(const [model,volume] of [
  [createBrepCylinder(3,5),Math.PI*9*5],
  [createBrepFrustum(3,1,5),Math.PI*5*(9+3+1)/3],
  [createBrepTube(3,1,5),Math.PI*8*5],
 ] as const){
  expect(inspectNurbsBrep(model).topologyValid).toBe(true)
  let previous=0
  for(const detail of [1,2,4,8,16,32]){
   const mesh=tessellateNurbsBrep(model,detail)
   expect(mesh.report.closed).toBe(true)
   expect(mesh.report.nonManifoldEdges).toBe(0)
   expect(mesh.report.signedVolumeMm3).toBeGreaterThan(previous)
   expect(mesh.report.signedVolumeMm3).toBeLessThan(volume)
   expect(new Set(mesh.topologyFaceIds)).toEqual(new Set(model.topologyIds!.faces))
   previous=mesh.report.signedVolumeMm3
  }
  expect(previous/volume).toBeGreaterThan(.999)
 }
})
it('preserves rational solids and identities through transforms and serialization',()=>{
 const brep=createBrepTube(3,1,5),built=tessellateNurbsBrep(brep,8)
 const document=transformSelection({version:1,sketches:[],bodies:[{id:'b',name:'Tube',brep,mesh:{positions:built.positions,indices:built.indices}}]},['b'],[5,-2,7],[1,1,1],43,2)
 const result:typeof brep=JSON.parse(JSON.stringify(document.bodies[0].brep))
 expect(inspectNurbsBrep(result).topologyValid).toBe(true)
 expect(result.topologyIds!.faces).toEqual(brep.topologyIds!.faces)
 expect(tessellateNurbsBrep(result,8).report.signedVolumeMm3).toBeCloseTo(built.report.signedVolumeMm3*8,6)
 const separated=booleanNurbsBrep(result,createBrepBox([0,0,0],[1,1,1]),'union')
 expect(inspectNurbsBrep(separated).topologyValid).toBe(true)
 expect(separated.bodies).toHaveLength(2)
 expect(separated.faces.filter(face=>face.surface.degreeU>1||face.surface.degreeV>1)).toHaveLength(result.faces.filter(face=>face.surface.degreeU>1||face.surface.degreeV>1).length)
})
it('preserves separated torus and box carriers despite overlapping bounding boxes',()=>{
 const torus=createBrepTorus(4,1),box=createBrepBox([-1,-1,-1],[1,1,1])
 const before=JSON.stringify([torus,box])
 const result=booleanNurbsBrep(torus,box,'union')
 expect(inspectNurbsBrep(result)).toMatchObject({topologyValid:true,bodyCount:2,boundaryEdgeCount:0,solidGeometryStatus:'not_certified'})
 expect(result.faces.map(face=>JSON.stringify(face.surface)).sort()).toEqual([...torus.faces,...box.faces].map(face=>JSON.stringify(face.surface)).sort())
 expect(analyzeNurbsBrep(result).signedVolumeMm3).toBeCloseTo(8*Math.PI**2+8,7)
 expect(tessellateNurbsBrep(result,4).report).toMatchObject({closed:true,nonManifoldEdges:0,orientationConflicts:0})
 expect(JSON.stringify([torus,box])).toBe(before)
 const restored=JSON.parse(JSON.stringify(result))
 expect(inspectNurbsBrep(restored).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(restored).signedVolumeMm3).toBeCloseTo(8*Math.PI**2+8,7)
})
it('refuses torus contact coincident with existing box trim edges without a faceted fallback',()=>{
 expect(()=>booleanNurbsBrep(createBrepTorus(4,1),createBrepBox([0,0,0],[10,10,10]),'union')).toThrow(/coincident boundaries/)
})
it('extrudes concave outer boundaries with several holes without filling notches',()=>{
 const profile:[number,number][]=[[0,0],[8,0],[8,3],[4,3],[4,7],[0,7]]
 const holes:[number,number][][]=[[[1,1],[1,2],[2,2],[2,1]],[[1,4],[1,6],[3,6],[3,4]]]
 const model=extrudeBrepPolygon(profile,0,2,holes),mesh=tessellateNurbsBrep(model,2)
 expect(mesh.report.closed).toBe(true)
 expect(mesh.report.signedVolumeMm3).toBeCloseTo((40-1-4)*2,7)
})

it('revolves a concave turning section into exact rational patches',async()=>{
 const {revolveBrepProfile}=await import('../src/services/geometry/brep')
 const profile:[number,number][]=[[1,0],[3,0],[3,2],[2,2],[2,5],[1,5]]
 const model=revolveBrepProfile(profile)
 expect(model.faces).toHaveLength(24)
 const mesh=tessellateNurbsBrep(model,16)
 expect(mesh.report.closed).toBe(true)
 expect(mesh.report.signedVolumeMm3/(25*Math.PI)).toBeGreaterThan(.998)
 expect(mesh.report.signedVolumeMm3/(25*Math.PI)).toBeLessThan(1)
 const withPoles=revolveBrepProfile([[0,0],[2,0],[2,2],[0,2]])
 expect(tessellateNurbsBrep(withPoles,8).report.closed).toBe(true)
})

it('authors circle and concave sketch extrusions on arbitrary workplanes',async()=>{
 const {extrudeSketchBrep,parseDirectDocument}=await import('../src/services/directModeling')
 const {sampleCurve}=await import('../src/services/directSketchGeometry')
 const analytic={kind:'circle' as const,center:[4,6] as [number,number],radius:2,start:0,sweep:360}
 const model=extrudeSketchBrep({id:'s',name:'Circle',closed:true,points:sampleCurve(analytic),analytic,plane:{origin:[10,20,30],u:[0,1,0],v:[0,0,1]}},-3,1)
 const mesh=tessellateNurbsBrep(model,8)
 expect(model.faces).toHaveLength(6)
 expect(mesh.report.closed).toBe(true)
 expect(Math.min(...model.vertices.map(v=>v.point[0]))).toBe(8)
 expect(Math.max(...model.vertices.map(v=>v.point[0]))).toBe(11)
 expect(Math.min(...model.vertices.map(v=>v.point[1]))).toBe(22)
 const doc=parseDirectDocument(JSON.stringify({version:1,sketches:[],bodies:[{id:'b',name:'Circle',brep:model,mesh:{positions:mesh.positions,indices:mesh.indices}}]}))
 expect(doc.bodies[0].brep?.topologyIds?.faces).toEqual(model.topologyIds!.faces)
})

it('keeps volumes and manifold incidence in rotated, holed, concave Boolean chains',()=>{
 const stock=extrudeBrepPolygon([[3,2],[3,5],[0,5],[0,0],[5,0],[5,2]],0,2,[[[1,1],[1,2],[2,2],[2,1]]])
 const cutter=createBrepBox([0,0,-1],[2.5,6,3])
 for(const model of [stock,cutter]){
  const rotate=(p:number[])=>[p[0]*Math.cos(.37)-p[1]*Math.sin(.37),p[0]*Math.sin(.37)+p[1]*Math.cos(.37),p[2]]
  for(const v of model.vertices)v.point=rotate(v.point) as [number,number,number]
  for(const e of model.edges)e.curve.controlPoints=e.curve.controlPoints.map(rotate)
  for(const f of model.faces)f.surface.controlPoints=f.surface.controlPoints.map(row=>row.map(rotate))
 }
 for(const [operation,volume,nextVolume] of [['intersection',23,60],['difference',13,73],['union',73,73]] as const){
  const result=booleanNurbsBrep(stock,cutter,operation)
  const mesh=tessellateNurbsBrep(result,2)
  expect(mesh.report.closed).toBe(true)
  expect(mesh.report.signedVolumeMm3).toBeCloseTo(volume,6)
  const again=booleanNurbsBrep(result,cutter,'union')
  expect(inspectNurbsBrep(again).topologyValid).toBe(true)
  expect(tessellateNurbsBrep(again,2).report.signedVolumeMm3).toBeCloseTo(nextVolume,6)
 }
})

it('preserves curved trim seams and pole topology for exact spheres, tori and apex cones',async()=>{
 const {createBrepSphere,createBrepTorus}=await import('../src/services/geometry/brep')
 for(const [body,volume] of [[createBrepSphere(3),36*Math.PI],[createBrepTorus(8,2),64*Math.PI*Math.PI],[createBrepFrustum(3,0,4),12*Math.PI],[createBrepFrustum(0,3,4),12*Math.PI]] as const){
  let previous=0
  for(const segments of [1,2,4,8,16]){
   const mesh=tessellateNurbsBrep(body,segments)
   expect(mesh.report.closed).toBe(true);expect(mesh.report.orientationConflicts).toBe(0);expect(mesh.report.degenerateTriangles).toBe(0)
   expect(new Set(mesh.topologyFaceIds)).toEqual(new Set(body.topologyIds!.faces))
   expect(mesh.report.signedVolumeMm3).toBeGreaterThan(previous);expect(mesh.report.signedVolumeMm3).toBeLessThan(volume)
   previous=mesh.report.signedVolumeMm3
  }
  expect(previous/volume).toBeGreaterThan(.99)
 }
})

it('makes watertight exact partial revolutions with planar end caps',async()=>{
 const {revolveBrepProfile}=await import('../src/services/geometry/brep')
 for(const profile of [[[0,0],[3,0],[3,4],[0,4]],[[1,0],[3,0],[3,4],[1,4]]] as [number,number][][]){
  for(const angle of [30,-90,137,270]){
   const body=revolveBrepProfile(profile,angle),volume=Math.PI*(9-profile[0][0]**2)*4*Math.abs(angle)/360
   for(const segments of [1,4,16]){const mesh=tessellateNurbsBrep(body,segments);expect(mesh.report.closed).toBe(true);expect(mesh.report.signedVolumeMm3).toBeGreaterThan(0);expect(mesh.report.signedVolumeMm3).toBeLessThanOrEqual(volume*1.000001)}
  }
 }
})

it('authors clockwise sketch extrusion and validates inputs in the Rust entry point',async()=>{
 const {extrudeSketchBrep}=await import('../src/services/directModeling')
 const sketch={id:'cw',name:'Clockwise',closed:true,points:[[0,0],[0,2],[3,2],[3,0]] as [number,number][]}
 const before=JSON.stringify(sketch)
 const model=extrudeSketchBrep(sketch,-4,1)
 expect(model.vertices.every(v=>v.point[2]===-3||v.point[2]===1)).toBe(true)
 expect(JSON.stringify(sketch)).toBe(before)
 expect(()=>extrudeSketchBrep({...sketch,closed:false},1)).toThrow(/closed sketch/)
 expect(()=>extrudeSketchBrep(sketch,0)).toThrow(/nonzero finite height/)
})
