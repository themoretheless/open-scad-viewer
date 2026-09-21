import {expect,it} from 'vitest'
import {bodySnapGeometry} from '../src/services/solidSnapGeometry'
import {createBrepBox,createBrepCylinder,createBrepSphere,tessellateNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
import {facePlane,solidTopology} from '../src/services/directSolidTools'
import {unprojectDirectPlane,projectDirectPoint,defaultDirectCamera} from '../src/services/directModelingTools'
import {worldPoint} from '../src/services/directSketchGeometry'
const body=(brep:NurbsBrep,segments=4)=>{const mesh=tessellateNurbsBrep(brep,segments);return{id:'b',name:'b',brep,mesh:{positions:mesh.positions,indices:mesh.indices}}}
it('uses topological vertices, edge midpoints, face centers and the body bounding center',()=>{
 const g=bodySnapGeometry(body(createBrepBox([0,0,0],[10,20,30])))
 expect(g.points.filter(p=>p.kind==='vertex')).toHaveLength(8)
 expect(g.points.filter(p=>p.kind==='midpoint')).toHaveLength(12)
 expect(g.points.filter(p=>p.kind==='center')).toHaveLength(6)
 expect(g.points.find(p=>p.kind==='bounds-center')!.point).toEqual([5,10,15])
})
it('finds true circular centers without exposing display-tessellation vertices',()=>{
 const a=bodySnapGeometry(body(createBrepCylinder(5,10),4)),b=bodySnapGeometry(body(createBrepCylinder(5,10),12))
 expect(a.points.filter(p=>p.kind==='vertex')).toEqual(b.points.filter(p=>p.kind==='vertex'))
 for(const z of [0,10])expect(a.points.some(p=>p.kind==='center'&&Math.hypot(p.point[0],p.point[1],p.point[2]-z)<1e-6)).toBe(true)
})
it('refuses sketching on a faceted display triangle of a curved B-rep surface',()=>{
 const sphere=body(createBrepSphere(5))
 expect(()=>facePlane(sphere,solidTopology(sphere.mesh).faces[0])).toThrow()
})
it('maps viewport pointer positions onto rotated and vertical sketch planes',()=>{
 const camera=defaultDirectCamera()
 for(const plane of [{origin:[10,2,3],u:[0,1,0],v:[0,0,1]},{origin:[0,0,10],u:[1,0,0],v:[0,1,0]}] as const){
  const workplane=structuredClone(plane) as unknown as Parameters<typeof unprojectDirectPlane>[1]
  const projected=projectDirectPoint(worldPoint([3,7],workplane),camera)
  const local=unprojectDirectPlane([projected[0],projected[1]],workplane,camera)
  expect(local[0]).toBeCloseTo(3);expect(local[1]).toBeCloseTo(7)
 }
})
