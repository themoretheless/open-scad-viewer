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

it('roundtrips rational snap targets through structured cloning without snapping to chords',async()=>{
 const {resolveModelingSnap}=await import('../src/services/modelingSnaps')
 const geometry=bodySnapGeometry(body(createBrepCylinder(5,10)))
 const copied=structuredClone(geometry)
 expect(copied).toEqual(geometry)
 const curved=copied.segments.find(segment=>segment.nurbs&&segment.a[2]===0&&segment.b[2]===0)!
 expect(curved).toBeDefined()
 const chord=curved.a.map((v,i)=>(v+curved.b[i])/2) as [number,number,number]
 const result=resolveModelingSnap(chord,{points:[],segments:[curved]}, {project:p=>[p[0]*100,p[1]*100],grid:0,geometry:true,radius:10,anchor:[0,0,0]})
 expect(result.kind).toBe('edge')
 expect(Math.hypot(result.point[0],result.point[1])).toBeCloseTo(5,10)
 expect(Math.hypot(chord[0],chord[1])).toBeLessThan(5-1e-5)
 expect(result.point).not.toEqual(chord)
 expect(resolveModelingSnap(chord,{points:[],segments:[curved]}, {project:p=>[p[0]*100,p[1]*100],grid:0,geometry:false}).kind).toBeNull()
})

it('matches the frozen geometry oracle and retains cache and exact curve references', async () => {
 const {referenceBodySnapGeometry}=await import('../benchmarks/rush/solidSnapGeometry-reference')
 const fixtures=[body(createBrepBox([-2,3,1],[8,15,20])),body(createBrepCylinder(5,10),8),body(createBrepSphere(5),4)]
 fixtures.push({...fixtures[0],brep:undefined} as unknown as typeof fixtures[number])
 for(const fixture of fixtures){
  const before=structuredClone(fixture),actual=bodySnapGeometry(fixture),expected=referenceBodySnapGeometry(fixture)
  expect(actual.points.map(p=>p.kind)).toEqual(expected.points.map(p=>p.kind))
  expect(actual.segments).toHaveLength(expected.segments.length)
  const close=(a:readonly number[],b:readonly number[])=>a.forEach((v,i)=>expect(v).toBeCloseTo(b[i],9))
  actual.points.forEach((p,i)=>close(p.point,expected.points[i].point))
  actual.segments.forEach((s,i)=>{
   const e=expected.segments[i];close(s.a,e.a);close(s.b,e.b)
   expect(Boolean(s.nurbs)).toBe(Boolean(e.nurbs))
   if(s.nurbs&&e.nurbs){expect(s.nurbs.curve).toBe(e.nurbs.curve);expect(s.nurbs.start).toBe(e.nurbs.start);expect(s.nurbs.end).toBe(e.nurbs.end)}
  })
  expect(bodySnapGeometry(fixture)).toBe(actual)
  expect(fixture).toEqual(before)
 }
})

it('preserves suppression of coplanar internal edges and rejects ellipse center hints', async () => {
 const {referenceBodySnapGeometry}=await import('../benchmarks/rush/solidSnapGeometry-reference')
 const box=body(createBrepBox([0,0,0],[10,20,30]))
 const split=structuredClone(box)
 split.brep.faces=[split.brep.faces[0],structuredClone(split.brep.faces[0])]
 const native=bodySnapGeometry(split),reference=referenceBodySnapGeometry(split)
 expect(native.points.filter(p=>p.kind==='vertex'||p.kind==='midpoint')).toEqual(reference.points.filter(p=>p.kind==='vertex'||p.kind==='midpoint'))
 expect(native.segments.length).toBe(reference.segments.length)
 expect(native.segments.length).toBeLessThan(bodySnapGeometry(box).segments.length)
 const ellipse=body(createBrepCylinder(5,10))
 for(const edge of ellipse.brep.edges)for(const p of edge.curve.controlPoints)p[1]*=2
 ellipse.brep.faces=[]
 const actual=bodySnapGeometry(ellipse),expected=referenceBodySnapGeometry(ellipse)
 expect(actual.points.filter(p=>p.kind==='center')).toEqual(expected.points.filter(p=>p.kind==='center'))
 expect(actual.points.filter(p=>p.kind==='center')).toHaveLength(0)
})
