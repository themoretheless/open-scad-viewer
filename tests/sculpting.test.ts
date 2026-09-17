import {describe,it,expect} from 'vitest'
import {extrudePolygonProfile,brushPolygonMesh,sculptPolygonMesh,inspectPolygonMesh} from '../src/services/geometry/polygon'
import {extrudeSubdivision,brushSubdivision,sculptSubdivision,tessellateSubdivision} from '../src/services/geometry/subdivision'
import {evaluateSdf,sculptSdf,sculptSdfSphere,tessellateSdf,type SdfField,type SdfStroke} from '../src/services/geometry/sdf'
import {brushNurbsCurve,brushNurbsSurface,sculptNurbsCurve,sculptNurbsSurface,extrudeNurbsCurve} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {brushDisplace,sculptMesh} from '../src/services/meshEditing'
import {SCULPT_FALLOFFS,SCULPT_KINDS,validateSculptBrush,type GeometryBrush,type SculptBrush} from '../src/services/geometryEditing'

const square=[[-1,-1],[1,-1],[1,1],[-1,1]]
const cube=()=>extrudePolygonProfile({outer:square,holes:[]},[0,0,2])
const circle:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1]}
const points=(positions:number[])=>{const out:number[][]=[];for(let i=0;i<positions.length;i+=3)out.push(positions.slice(i,i+3));return out}
const invalidBrushes:GeometryBrush[]=[
 {center:[0,0,0],radius:0,displacement:[0,0,1]},
 {center:[0,0,0],radius:-1,displacement:[0,0,1]},
 {center:[0,0,0],radius:1e7,displacement:[0,0,1]},
 {center:[0,0,0],radius:1,displacement:[1e7,0,0]},
]

describe('sculpting through WASM',()=>{
 it('polygon brush moves only vertices inside the radius with smooth falloff',()=>{
  const mesh=cube()
  const out=brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,1]})
  expect(out.indices).toEqual(mesh.indices)
  expect(out.report.closed).toBe(true);expect(out.report.degenerateTriangles).toBe(0)
  expect(out.report.signedVolumeMm3).toBeGreaterThan(mesh.report.signedVolumeMm3)
  const before=points(mesh.positions),after=points(out.positions)
  const moved=before.map((p,i)=>[p,after[i]] as const).filter(([a,b])=>a.some((v,k)=>v!==b[k]))
  expect(moved.length).toBeGreaterThanOrEqual(1);expect(moved.length).toBeLessThan(before.length)
  for(const [a,b] of moved){expect(b[0]).toBe(a[0]);expect(b[1]).toBe(a[1]);expect(b[2]-a[2]).toBeGreaterThan(0);expect(b[2]-a[2]).toBeLessThanOrEqual(1)}
  const half=brushPolygonMesh(mesh,{center:[1,1,2.5],radius:1,displacement:[0,0,4]})
  const corner=points(half.positions).find(p=>p[0]===1&&p[1]===1&&p[2]>2)!
  expect(corner[2]).toBeCloseTo(4,12)
 })
 it('polygon brush carves with negative displacement and is idempotent outside the radius',()=>{
  const mesh=cube()
  const carved=brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,-0.5]})
  expect(carved.report.closed).toBe(true);expect(carved.report.signedVolumeMm3).toBeLessThan(mesh.report.signedVolumeMm3)
  const miss=brushPolygonMesh(mesh,{center:[0.5,0.5,2],radius:0.1,displacement:[0,0,-0.5]})
  expect(miss.positions).toEqual(mesh.positions)
  expect(inspectPolygonMesh(carved).closed).toBe(true)
 })
 it('brushDisplace validates inputs before calling the kernel',()=>{
  const mesh=cube()
  expect(brushDisplace(mesh,[1,1,2],0.5,[0,0,1]).positions).toEqual(brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,1]}).positions)
  expect(()=>brushDisplace(mesh,[1,1,2],0,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[1,1,2],-1,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[1,1,2],Number.NaN,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[Number.POSITIVE_INFINITY,1,2],1,[0,0,1])).toThrow('Invalid brush.')
  expect(()=>brushDisplace(mesh,[1,1,2],1,[0,Number.NaN,1])).toThrow('Invalid brush.')
 })
 it('kernel rejects invalid brushes and degenerate results for every representation',()=>{
  const mesh=cube(),cage=extrudeSubdivision(square.map(p=>[...p,0]),[0,0,2]),surface=extrudeNurbsCurve(circle,[0,0,3])
  for(const brush of invalidBrushes){
   expect(()=>brushPolygonMesh(mesh,brush)).toThrow()
   expect(()=>brushSubdivision(cage,brush)).toThrow()
   expect(()=>brushNurbsCurve(circle,brush)).toThrow()
   expect(()=>brushNurbsSurface(surface,brush)).toThrow()
  }
  expect(()=>brushPolygonMesh({positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},{center:[0,0,0],radius:3,displacement:[0,0,0]})).not.toThrow()
  expect(()=>brushPolygonMesh({positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},{center:[0,0,0],radius:0.1,displacement:[1,0,0]})).toThrow()
 })
 it('subdivision brush edits the cage and keeps it subdividable',()=>{
  const cage=extrudeSubdivision(square.map(p=>[...p,0]),[0,0,2])
  const out=brushSubdivision(cage,{center:[1,1,2],radius:0.5,displacement:[0,0,1]})
  expect(out.faces).toEqual(cage.faces)
  const moved=cage.vertices.map((p,i)=>[p,out.vertices[i]] as const).filter(([a,b])=>a.some((v,k)=>v!==b[k]))
  expect(moved).toHaveLength(1);expect(moved[0][0]).toEqual([1,1,2]);expect(moved[0][1]).toEqual([1,1,3])
  const tess=tessellateSubdivision(out,2)
  expect(tess.report.closed).toBe(true)
  expect(tess.report.signedVolumeMm3).toBeGreaterThan(tessellateSubdivision(cage,2).report.signedVolumeMm3)
 })
 it('nurbs brush edits control points and preserves knots, weights and untouched ends',()=>{
  const curve=brushNurbsCurve(circle,{center:[1,1,0],radius:0.5,displacement:[0,0,2]})
  expect(curve.knots).toEqual(circle.knots);expect(curve.weights).toEqual(circle.weights);expect(curve.degree).toBe(2)
  expect(curve.controlPoints[0]).toEqual([1,0,0]);expect(curve.controlPoints[2]).toEqual([0,1,0])
  expect(curve.controlPoints[1][2]).toBeCloseTo(2,12)
  expect(evaluateNurbsCurve(curve,0).point).toEqual([1,0,0])
  expect(evaluateNurbsCurve(curve,0.5).point[2]).toBeGreaterThan(0)
  expect(evaluateNurbsCurve(circle,0.5).point[2]).toBe(0)
  const surface=extrudeNurbsCurve(circle,[0,0,3])
  const sculpted=brushNurbsSurface(surface,{center:[1,1,3],radius:0.5,displacement:[1,0,0]})
  expect(sculpted.knotsU).toEqual(surface.knotsU);expect(sculpted.knotsV).toEqual(surface.knotsV);expect(sculpted.weights).toEqual(surface.weights)
  const moved=surface.controlPoints.flat().map((p,i)=>[p,sculpted.controlPoints.flat()[i]] as const).filter(([a,b])=>a.some((v,k)=>v!==b[k]))
  expect(moved).toHaveLength(1);expect(moved[0][1]).toEqual([2,1,3])
  expect(evaluateNurbsSurface(sculpted,0,0).point).toEqual(evaluateNurbsSurface(surface,0,0).point)
 })
 it('sdf sculpt strokes add and remove material and remain polygonizable',()=>{
  const base:SdfField={kind:'sphere',center:[0,0,0],radius:1}
  const added=sculptSdfSphere(base,[1.5,0,0],0.5)
  expect(added.kind).toBe('union');expect(evaluateSdf(base,[1.5,0,0])).toBe(0.5);expect(evaluateSdf(added,[1.5,0,0])).toBe(-0.5)
  const removed=sculptSdfSphere(base,[0,0,0],0.5,true)
  expect(removed.kind).toBe('difference');expect(evaluateSdf(removed,[0,0,0])).toBe(0.5);expect(evaluateSdf(removed,[0.75,0,0])).toBeLessThan(0)
  const strokes=sculptSdfSphere(sculptSdfSphere(removed,[0,1,0],0.3),[0,-1,0],0.3,true)
  expect(evaluateSdf(strokes,[0,1,0])).toBeLessThan(0);expect(evaluateSdf(strokes,[0,-1,0])).toBeGreaterThan(0)
  const mesh=tessellateSdf(strokes,{min:[-2,-2,-2],max:[2,2,2],cells:[16,16,16]})
  expect(mesh.report.closed).toBe(true);expect(mesh.report.degenerateTriangles).toBe(0)
  for(const radius of [0,-1,Number.NaN,Number.POSITIVE_INFINITY])expect(()=>sculptSdfSphere(base,[0,0,0],radius)).toThrow()
  expect(()=>sculptSdfSphere(base,[Number.NaN,0,0],1)).toThrow()
  expect(()=>sculptSdfSphere({kind:'sphere',center:[0,0,0],radius:-1},[0,0,0],1)).toThrow()
 })
 it('sculpt grab reproduces the legacy brush and every kind keeps topology',()=>{
  const mesh=cube()
  const legacy=brushPolygonMesh(mesh,{center:[1,1,2],radius:0.5,displacement:[0,0,1]})
  const grab=sculptPolygonMesh(mesh,{kind:'grab',displacement:[0,0,1],center:[1,1,2],radius:0.5})
  expect(grab.positions).toEqual(legacy.positions)
  expect(sculptMesh(mesh,{kind:'grab',displacement:[0,0,1],center:[1,1,2],radius:0.5}).positions).toEqual(legacy.positions)
  for(const kind of SCULPT_KINDS){
   const brush:SculptBrush=kind==='grab'?{kind,displacement:[0,0,0.5],center:[0,0,2],radius:1.5}:{kind,strength:kind==='draw'||kind==='inflate'?0.5:0.5,center:[0,0,2],radius:1.5}
   const out=sculptPolygonMesh(mesh,brush)
   expect(out.indices).toEqual(mesh.indices);expect(out.report.closed).toBe(true);expect(out.report.degenerateTriangles).toBe(0)
  }
 })
 it('draw, inflate and pinch change volume in the expected direction; smooth and flatten converge',()=>{
  const mesh=cube(),v0=mesh.report.signedVolumeMm3
  expect(sculptPolygonMesh(mesh,{kind:'draw',strength:0.5,center:[0,0,2],radius:1.5}).report.signedVolumeMm3).toBeGreaterThan(v0)
  expect(sculptPolygonMesh(mesh,{kind:'draw',strength:-0.5,center:[0,0,2],radius:1.5}).report.signedVolumeMm3).toBeLessThan(v0)
  expect(sculptPolygonMesh(mesh,{kind:'inflate',strength:0.5,center:[0,0,1],radius:100,falloff:'constant'}).report.signedVolumeMm3).toBeGreaterThan(v0)
  expect(sculptPolygonMesh(mesh,{kind:'pinch',strength:0.5,center:[0,0,2],radius:1.5,falloff:'constant'}).report.signedVolumeMm3).toBeLessThan(v0)
  const spiked=sculptPolygonMesh(mesh,{kind:'grab',displacement:[0,0,1],center:[1,1,2],radius:0.5})
  const smoothed=sculptPolygonMesh(spiked,{kind:'smooth',strength:1,center:[1,1,3],radius:0.5})
  const spike=(m:{positions:number[]})=>Math.max(...points(m.positions).map(p=>p[2]))
  expect(spike(smoothed)).toBeLessThan(spike(spiked));expect(smoothed.report.closed).toBe(true)
  // Radius 1.9 reaches every top vertex (spike at distance sqrt(3)) but no bottom vertex (distance sqrt(6)).
  const flattened=sculptPolygonMesh(spiked,{kind:'flatten',strength:1,center:[0,0,2],radius:1.9,falloff:'constant'})
  const top=points(flattened.positions).filter(p=>p[2]>1.5)
  expect(top.length).toBeGreaterThanOrEqual(4)
  // All affected vertices land on one (tilted) plane: distance to the plane through the first three is ~0.
  const [p0,p1,p2]=top,u=p1.map((v,k)=>v-p0[k]),w=p2.map((v,k)=>v-p0[k])
  const n=[u[1]*w[2]-u[2]*w[1],u[2]*w[0]-u[0]*w[2],u[0]*w[1]-u[1]*w[0]],len=Math.hypot(...n)
  for(const p of top)expect(Math.abs(p.reduce((acc,v,k)=>acc+(v-p0[k])*n[k],0))/len).toBeLessThan(1e-9)
  expect(Math.max(...top.map(p=>p[2]))-Math.min(...top.map(p=>p[2]))).toBeLessThan(1)
  expect(flattened.report.closed).toBe(true)
 })
 it('falloff profiles order the displacement and symmetry mirrors the stroke',()=>{
  const mesh=cube()
  const cornerZ=(f:SculptBrush['falloff'])=>points(sculptPolygonMesh(mesh,{kind:'grab',displacement:[0,0,1],center:[0,0,2],radius:2,falloff:f}).positions).find(p=>p[0]===1&&p[1]===1&&p[2]>2)![2]-2
  const z=Object.fromEntries(SCULPT_FALLOFFS.map(f=>[f,cornerZ(f)]))
  expect(z.constant).toBeCloseTo(1,12);expect(z.root).toBeGreaterThan(z.linear);expect(z.linear).toBeGreaterThan(z.sharp);expect(z.sphere).toBeGreaterThan(z.smooth)
  const mirrored=sculptPolygonMesh(mesh,{kind:'grab',displacement:[0,0,1],center:[1,1,2],radius:0.5,symmetry:{axes:[true,true,false]}})
  const raised=points(mirrored.positions).filter(p=>Math.abs(p[2]-3)<1e-9)
  expect(raised.length).toBeGreaterThanOrEqual(4)
  expect(new Set(raised.map(p=>`${p[0]},${p[1]}`)).size).toBe(4)
  const single=sculptPolygonMesh(mesh,{kind:'grab',displacement:[0,0,1],center:[1,1,2],radius:0.5})
  expect(points(single.positions).filter(p=>Math.abs(p[2]-3)<1e-9).map(p=>`${p[0]},${p[1]}`)).toEqual(expect.arrayContaining(['1,1']))
  expect(new Set(points(single.positions).filter(p=>Math.abs(p[2]-3)<1e-9).map(p=>`${p[0]},${p[1]}`)).size).toBe(1)
 })
 it('sculpt brush validation runs client-side and in the kernel',()=>{
  const mesh=cube()
  expect(()=>validateSculptBrush({kind:'draw',strength:1,center:[0,0],radius:1} as unknown as SculptBrush)).toThrow('center')
  expect(()=>validateSculptBrush({kind:'draw',strength:1,center:[0,0,0],radius:0})).toThrow('radius')
  expect(()=>validateSculptBrush({kind:'draw',strength:Number.NaN,center:[0,0,0],radius:1})).toThrow('strength')
  expect(()=>validateSculptBrush({kind:'smooth',strength:1.5,center:[0,0,0],radius:1})).toThrow('[0, 1]')
  expect(()=>validateSculptBrush({kind:'pinch',strength:-0.1,center:[0,0,0],radius:1})).toThrow('[0, 1]')
  expect(()=>validateSculptBrush({kind:'grab',displacement:[1e7,0,0],center:[0,0,0],radius:1})).toThrow('displacement')
  expect(()=>validateSculptBrush({kind:'draw',strength:1,center:[0,0,0],radius:1,falloff:'gaussian' as never})).toThrow('falloff')
  expect(()=>validateSculptBrush({kind:'twirl' as never,strength:1,center:[0,0,0],radius:1})).toThrow('kind')
  expect(()=>sculptPolygonMesh(mesh,{kind:'draw',strength:1,center:[0,0,0],radius:1e7})).toThrow()
  // A kernel-side failure: sculpt result with degenerate triangles is rejected.
  expect(()=>sculptPolygonMesh({positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},{kind:'pinch',strength:1,center:[1/3,1/3,0],radius:10,falloff:'constant'})).toThrow()
 })
 it('subdivision and nurbs sculpt through the bridge',()=>{
  const cage=extrudeSubdivision(square.map(p=>[...p,0]),[0,0,2])
  const inflated=sculptSubdivision(cage,{kind:'inflate',strength:1,center:[0,0,1],radius:100,falloff:'constant'})
  expect(inflated.faces).toEqual(cage.faces)
  for(const [a,b] of cage.vertices.map((p,i)=>[p,inflated.vertices[i]] as const))expect(Math.hypot(...b.map((v,k)=>v-a[k]))).toBeCloseTo(1,9)
  expect(tessellateSubdivision(inflated,2).report.signedVolumeMm3).toBeGreaterThan(tessellateSubdivision(cage,2).report.signedVolumeMm3)
  expect(()=>sculptSubdivision(cage,{kind:'flatten',strength:2,center:[0,0,0],radius:1})).toThrow()
  const drawn=sculptNurbsCurve(circle,{kind:'draw',strength:1,center:[1,1,0],radius:0.5})
  expect(drawn.knots).toEqual(circle.knots);expect(drawn.weights).toEqual(circle.weights)
  expect(drawn.controlPoints[1][0]).toBeCloseTo(1+Math.SQRT1_2,9);expect(drawn.controlPoints[1][1]).toBeCloseTo(1+Math.SQRT1_2,9)
  expect(drawn.controlPoints[0]).toEqual([1,0,0])
  expect(sculptNurbsCurve(circle,{kind:'smooth',strength:1,center:[1,1,0],radius:0.5}).controlPoints[1]).toEqual([0.5,0.5,0])
  const surface=extrudeNurbsCurve(circle,[0,0,3])
  const sculpted=sculptNurbsSurface(surface,{kind:'inflate',strength:0.5,center:[0,0,0],radius:100,falloff:'constant'})
  expect(sculpted.knotsU).toEqual(surface.knotsU);expect(sculpted.weights).toEqual(surface.weights)
  for(const [a,b] of surface.controlPoints.flat().map((p,i)=>[p,sculpted.controlPoints.flat()[i]] as const))expect(Math.hypot(...b.map((v,k)=>v-a[k]))).toBeCloseTo(0.5,9)
  expect(evaluateNurbsCurve(drawn,0).point).toEqual([1,0,0])
  expect(evaluateNurbsSurface(sculpted,0,0).point).not.toEqual(evaluateNurbsSurface(surface,0,0).point)
 })
 it('sdf strokes support box and capsule tools with hard and smooth blending',()=>{
  const base:SdfField={kind:'sphere',center:[0,0,0],radius:1}
  const box=sculptSdf(base,{tool:{shape:'box',center:[1.2,0,0],half_size:[0.3,0.3,0.3]}})
  expect(box.kind).toBe('union');expect(evaluateSdf(box,[1.2,0,0])).toBeLessThan(0)
  const capsule=sculptSdf(base,{tool:{shape:'capsule',a:[0,0,0.8],b:[0,0,1.6],radius:0.2},blend:0.2})
  expect(capsule.kind).toBe('smooth_union');expect(evaluateSdf(capsule,[0,0,1.6])).toBeLessThan(0)
  expect(evaluateSdf({kind:'capsule',a:[0,0,0],b:[2,0,0],radius:0.5},[1,1,0])).toBe(0.5)
  const hard=sculptSdf(base,{tool:{shape:'sphere',center:[1,0,0],radius:0.5},remove:true})
  const smooth=sculptSdf(base,{tool:{shape:'sphere',center:[1,0,0],radius:0.5},remove:true,blend:0.25})
  expect(hard.kind).toBe('difference');expect(smooth.kind).toBe('smooth_difference')
  expect(evaluateSdf(smooth,[-0.9,0,0])).toBeCloseTo(evaluateSdf(hard,[-0.9,0,0]),12)
  // On the seam circle both distances vanish; the blend lifts the value by k*h*(1-h)=k/4.
  const seam=[0.875,Math.sqrt(1-0.875**2),0]
  expect(evaluateSdf(hard,seam)).toBeCloseTo(0,9);expect(evaluateSdf(smooth,seam)).toBeCloseTo(0.25/4,9)
  const mesh=tessellateSdf(smooth,{min:[-2,-2,-2],max:[2,2,2],cells:[16,16,16]})
  expect(mesh.report.closed).toBe(true)
  const bad:SdfStroke[]=[
   {tool:{shape:'sphere',center:[0,0,0],radius:0}},
   {tool:{shape:'box',center:[0,0,0],half_size:[1,-1,1]}},
   {tool:{shape:'capsule',a:[Number.NaN,0,0],b:[0,0,0],radius:1}},
   {tool:{shape:'sphere',center:[0,0,0],radius:1},blend:0},
   {tool:{shape:'sphere',center:[0,0,0],radius:1},blend:-1},
  ]
  for(const stroke of bad)expect(()=>sculptSdf(base,stroke)).toThrow()
 })
})
