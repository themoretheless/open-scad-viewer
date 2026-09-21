import { describe, expect, it } from 'vitest'
import { resolveModelingSnap, sketchSnapGeometry, type SnapGeometry } from '../src/services/modelingSnaps'
import { convertGridSize, validGridStep } from '../src/services/modelingGrid'
import type { Vec3 } from '../src/services/directSketchGeometry'
const empty: SnapGeometry = {points:[],segments:[]}
const options = {project: (p: Vec3): [number,number] => [p[0]*10,p[1]*10], grid:0, geometry:true}
describe('modeling snap targets',()=>{
  it('uses a ten pixel capture radius at different zoom levels',()=>{
    const geometry: SnapGeometry={points:[{point:[10,10,0],kind:'vertex'}],segments:[]}
    expect(resolveModelingSnap([10.9,10,0],geometry,options).kind).toBe('vertex')
    expect(resolveModelingSnap([11.1,10,0],geometry,options).kind).toBeNull()
    expect(resolveModelingSnap([10.9,10,0],geometry,{...options,project:p=>[p[0]*100,p[1]*100]}).kind).toBeNull()
  })
  it('prefers a vertex to an edge or grid and supports free positioning',()=>{
    const geometry: SnapGeometry={points:[{point:[1,1,0],kind:'vertex'}],segments:[{a:[0,1.2,0],b:[3,1.2,0]}]}
    expect(resolveModelingSnap([1.2,1.2,0],geometry,{...options,grid:10})).toMatchObject({point:[1,1,0],kind:'vertex'})
    expect(resolveModelingSnap([1.2,1.2,0],geometry,{...options,geometry:false})).toMatchObject({point:[1.2,1.2,0],kind:null})
  })
  it('finds segment midpoints, edge projections and intersections',()=>{
    const g=sketchSnapGeometry([{id:'a',name:'a',closed:false,points:[[0,0],[20,0]]}])
    expect(resolveModelingSnap([10,.4,0],g,options)).toMatchObject({point:[10,0,0],kind:'midpoint'})
    expect(resolveModelingSnap([7,.4,0],g,options)).toMatchObject({point:[7,0,0],kind:'edge'})
    g.segments.push({a:[7,-10,0],b:[7,10,0]})
    expect(resolveModelingSnap([7.1,.4,0],g,options)).toMatchObject({point:[7,0,0],kind:'intersection'})
  })
  it('snaps to analytic circles, excluding tessellation vertices and the missing part of arcs',()=>{
    const g=sketchSnapGeometry([{id:'c',name:'c',closed:true,points:[[99,99]],analytic:{kind:'circle',center:[0,0],radius:10,start:0,sweep:360}}])
    expect(g.points).toHaveLength(5)
    expect(resolveModelingSnap([.3,.4,0],g,options)).toMatchObject({point:[0,0,0],kind:'center'})
    const result=resolveModelingSnap([7.2,7.2,0],g,options)
    expect(result.kind).toBe('edge');expect(Math.hypot(...result.point)).toBeCloseTo(10)
    g.circles![0].sweep=90
    expect(resolveModelingSnap([-7.2,-7.2,0],g,options).kind).toBeNull()
  })
  it('does not snap to a feature outside the locked axis',()=>{
    const g: SnapGeometry={points:[{point:[10,.2,0],kind:'vertex'}],segments:[]}
    const result=resolveModelingSnap([9.8,0,0],g,{...options,constrain:p=>[p[0],0,0]})
    expect(result.kind).toBeNull()
  })
  it('aligns to the anchor and quantizes only unlocked grid coordinates',()=>{
    expect(resolveModelingSnap([7,.4,0],empty,{...options,guides:true,anchor:[0,0,0],gridAxes:[0,1]})).toMatchObject({point:[7,0,0],kind:'axis'})
    expect(resolveModelingSnap([7.4,0,0],empty,{...options,grid:1,guides:true,anchor:[0,0,0],gridAxes:[0]}).point).toEqual([7,0,0])
    expect(resolveModelingSnap([-27,2,7],empty,{...options,grid:25.4,gridAxes:[0]})).toMatchObject({point:[-25.4,2,7],kind:'grid'})
  })
})
describe('cell units',()=>{
  it('converts display units without changing model dimensions',()=>{
    expect(convertGridSize(1,'in','mm')).toBe(25.4)
    expect(convertGridSize(25.4,'mm','in')).toBe(1)
    expect(convertGridSize(1,'m','cm')).toBe(100)
  })
  it('rejects invalid and unbounded grid sizes',()=>{
    for(const value of [NaN,Infinity,0,-1,1e7])expect(validGridStep(value)).toBe(false)
    expect(validGridStep(.01)).toBe(true)
  })
})

describe('construction key points',()=>{
 it('finds an area centroid and a distinct bounding center of a concave polygon',()=>{
  const g=sketchSnapGeometry([{id:'l',name:'L',closed:true,points:[[0,0],[4,0],[4,1],[1,1],[1,4],[0,4]]}])
  const center=g.points.find(p=>p.kind==='center')!.point
  expect(center[0]).toBeCloseTo(19/14);expect(center[1]).toBeCloseTo(19/14)
  expect(g.points.find(p=>p.kind==='bounds-center')!.point).toEqual([2,2,0])
  expect(resolveModelingSnap([center[0]+.01,center[1],0],g,{...options,radius:2}).kind).toBe('center')
 })
 it('exposes circle quadrants and labels the arc midpoint separately from endpoints',()=>{
  const make=(kind:'circle'|'arc',sweep:number)=>sketchSnapGeometry([{id:'c',name:'c',closed:kind==='circle',points:[],analytic:{kind,center:[0,0],radius:10,start:0,sweep}}])
  expect(make('circle',360).points.filter(p=>p.kind==='quadrant')).toHaveLength(4)
  expect(make('arc',90).points.filter(p=>p.kind==='midpoint')).toHaveLength(1)
 })
 it('finds tangent points from an external anchor and the perpendicular foot on a segment',()=>{
  const circle:SnapGeometry={points:[],segments:[],circles:[{center:[0,0,0],radius:5,start:0,sweep:360}]}
  const target:[number,number,number]=[2.5,Math.sqrt(18.75),0]
  const tangent=resolveModelingSnap(target,circle,{...options,anchor:[10,0,0]});expect(tangent.kind).toBe('tangent');tangent.point.forEach((v,i)=>expect(v).toBeCloseTo(target[i]))
  const line:SnapGeometry={points:[],segments:[{a:[0,0,0],b:[10,0,0]}]}
  expect(resolveModelingSnap([3,.2,0],line,{...options,anchor:[3,8,0]})).toMatchObject({kind:'perpendicular',point:[3,0,0]})
  expect(resolveModelingSnap([3,.2,0],line,{...options,anchor:[3,8,0],relations:false}).kind).toBe('edge')
 })
 it('finds circle-line and circle-circle intersections without snapping to missing arc sections',()=>{
  const g:SnapGeometry={points:[],segments:[{a:[-10,3,0],b:[10,3,0]}],circles:[{center:[0,0,0],radius:5,start:0,sweep:360}]}
  expect(resolveModelingSnap([4.05,3.02,0],g,options)).toMatchObject({kind:'intersection',point:[4,3,0]})
  g.segments=[];g.circles!.push({center:[8,0,0],radius:5,start:0,sweep:360})
  expect(resolveModelingSnap([4.05,3.02,0],g,options)).toMatchObject({kind:'intersection',point:[4,3,0]})
  g.circles![0].sweep=-90
  expect(resolveModelingSnap([4.05,3.02,0],g,options).kind).not.toBe('intersection')
 })
 it('allows keypoint snaps to be disabled separately from edge snapping',()=>{
  const g:SnapGeometry={points:[{point:[5,0,0],kind:'midpoint'}],segments:[{a:[0,0,0],b:[10,0,0]}]}
  expect(resolveModelingSnap([5,.1,0],g,{...options,keypoints:false})).toMatchObject({kind:'edge',point:[5,0,0]})
 })
})
it('selects the closest center instead of a farther vertex at small screen sizes',()=>{
 const g:SnapGeometry={points:[{point:[0,0,0],kind:'vertex'},{point:[.4,.4,0],kind:'center'}],segments:[]}
 expect(resolveModelingSnap([.4,.4,0],g,options)).toMatchObject({kind:'center',point:[.4,.4,0]})
})
it('finds intersections in vertical planes but rejects merely projected crossings',()=>{
 const g:SnapGeometry={points:[],segments:[{a:[0,0,0],b:[10,0,10]},{a:[0,0,10],b:[10,0,0]}]},project=(p:Vec3):[number,number]=>[p[0]*10,p[2]*10]
 expect(resolveModelingSnap([5,0,5],g,{...options,project})).toMatchObject({kind:'intersection',point:[5,0,5]})
 g.segments[1]={a:[0,2,10],b:[10,2,0]}
 expect(resolveModelingSnap([5,0,5],g,{...options,project}).kind).not.toBe('intersection')
})
