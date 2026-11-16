import {sketchProfile} from './retainedSketchProfile'
import { DirectInputError } from '../i18n/kernelErrors'
import { extrudeSketchBrep, parseDirectDocument, type DirectBody, type DirectDocument, type DirectSketch } from './directModeling'
import { cross3, xyPlane, type SketchPlane } from './directSketchGeometry'
import { booleanNurbsBrep, extrudeBrepCurves, tessellateNurbsBrep, transformNurbsBrep, type NurbsBrep } from '../kernel/geometry/brep'
import { authorBrepProfile, transformBrepProfile, validateBrepProfile } from '../kernel/geometry/brepProfile'
import { booleanPolygonMeshes } from '../kernel/geometry/polygon'
import { stringifyMeshJson } from '../exchange/meshJson'

export function sameSketchPlane(a: SketchPlane = xyPlane(), b: SketchPlane = xyPlane()) {
  const ao = a.origin, au = a.u, av = a.v
  const bo = b.origin, bu = b.u, bv = b.v
  return Math.abs(ao[0] - bo[0]) < 1e-7 && Math.abs(ao[1] - bo[1]) < 1e-7 && Math.abs(ao[2] - bo[2]) < 1e-7
    && Math.abs(au[0] - bu[0]) < 1e-7 && Math.abs(au[1] - bu[1]) < 1e-7 && Math.abs(au[2] - bu[2]) < 1e-7
    && Math.abs(av[0] - bv[0]) < 1e-7 && Math.abs(av[1] - bv[1]) < 1e-7 && Math.abs(av[2] - bv[2]) < 1e-7
}
/** True when two non-adjacent edges of a closed polygon cross or touch. */
export function polygonSelfIntersects(points: readonly (readonly number[])[]): boolean {
  const n=points.length
  if(n<4||n>4096)return false
  const orient=(a:readonly number[],b:readonly number[],c:readonly number[])=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
  const eps=1e-12
  for(let i=0;i<n;i++) {
    const a=points[i],b=points[(i+1)%n]
    for(let j=i+2;j<n;j++) {
      if(i===0&&j===n-1)continue
      const c=points[j],d=points[(j+1)%n]
      const o1=orient(a,b,c),o2=orient(a,b,d),o3=orient(c,d,a),o4=orient(c,d,b)
      if(((o1>eps&&o2<-eps)||(o1<-eps&&o2>eps))&&((o3>eps&&o4<-eps)||(o3<-eps&&o4>eps)))return true
    }
  }
  return false
}
/** Nested contours use even/odd fill; circles remain rational curves. */
export function extrudeSketchProfile(sketches: readonly DirectSketch[], height: number, offset=0): NurbsBrep {
  if (!sketches.length || sketches.some(s=>!s.closed)) throw new DirectInputError('DIRECT_PROFILE_MISSING','Select at least one closed contour.',undefined,sketches.filter(s=>!s.closed).map(s=>s.id))
  if (!Number.isFinite(height)||Math.abs(height)<.00001||!Number.isFinite(offset)) throw new DirectInputError('DIRECT_EXTRUDE_HEIGHT','Extrusion height must be finite and nonzero.','height')
  const plane=sketches[0].plane??xyPlane()
  if(sketches.some(s=>!sameSketchPlane(s.plane,plane))) throw new DirectInputError('DIRECT_PROFILE_PLANE','All profile contours must lie on the same workplane.',undefined,sketches.filter(s=>!sameSketchPlane(s.plane,plane)).map(s=>s.id))
  for(const sketch of sketches)if(!sketch.retainedProfile&&!sketch.analytic&&polygonSelfIntersects(sketch.points))
    throw new DirectInputError('DIRECT_SKETCH_SELF_INTERSECTION','Profile contour intersects itself.',undefined,[sketch.id])
  if(sketches.length===1&&!sketches[0].retainedProfile)return extrudeSketchBrep(sketches[0],height,offset)
  const loops=sketches.flatMap(sketch=>{
    if(sketch.retainedProfile)return sketchProfile(sketch).loops
    if(sketch.analytic?.kind==='circle') {
      const {center,radius}=sketch.analytic
      return transformBrepProfile(authorBrepProfile({kind:'circle',radius}),[1,0,0,0,0,1,0,0,0,0,1,0,center[0],center[1],0,1]).loops
    }
    return authorBrepProfile({kind:'polygon',rings:[sketch.points]}).loops
  })
  const profile=validateBrepProfile(loops,'even-odd')
  const solid=extrudeBrepCurves(profile.loops,offset+Math.min(0,height),offset+Math.max(0,height))
  const n=cross3(plane.u,plane.v)
  return transformNurbsBrep(solid,[
    [plane.u[0],plane.v[0],n[0],plane.origin[0]],
    [plane.u[1],plane.v[1],n[1],plane.origin[1]],
    [plane.u[2],plane.v[2],n[2],plane.origin[2]], [0,0,0,1],
  ])
}
export interface DirectExtrusionOptions { sketchIds: string[]; height: number; offset: number; operation: 'new'|'union'|'difference'; targetId: string; id: string; segments?: number }
/** Shared preview/commit path; the input document is never changed. */
export function buildDirectExtrusion(document: DirectDocument, options: DirectExtrusionOptions): DirectBody | null {
  const sketches=options.sketchIds.map(id=>{const s=document.sketches.find(s=>s.id===id);if(!s)throw new DirectInputError('DIRECT_PROFILE_MISSING','Profile contour no longer exists.',undefined,[id]);return s})
  const tool=extrudeSketchProfile(sketches,options.height,options.offset)
  const target=document.bodies.find(b=>b.id===options.targetId)
  if(options.operation!=='new'&&!target)throw new DirectInputError('DIRECT_TARGET_BODY','Select a target body.')
  let brep:NurbsBrep|undefined=tool
  if(options.operation!=='new'&&target?.brep)brep=booleanNurbsBrep(target.brep,tool,options.operation)
  if(!brep.bodies.length)return null
  const mesh=tessellateNurbsBrep(brep,options.segments??8)
  let positions:Float64Array=mesh.positions.slice(),indices:Uint32Array=mesh.indices.slice()
  if(options.operation!=='new'&&target&&!target.brep) {
    const result=booleanPolygonMeshes(target.mesh,{positions,indices},options.operation)
    positions=result.positions;indices=result.indices;brep=undefined
    if(!indices.length)return null
  }
  return {...(options.operation==='new'?{id:options.id,name:`${sketches[0].name} · 3D`}:target!),mesh:{positions,indices},...(brep?{brep}:{})}
}
export function applyDirectExtrusionProfile(document: DirectDocument, options: DirectExtrusionOptions): DirectDocument {
  const body=buildDirectExtrusion(document,options)
  const next={...document,bodies:options.operation==='new'?[...document.bodies,...(body?[body]:[])]:document.bodies.flatMap(b=>b.id===options.targetId?(body?[body]:[]):[b])}
  return parseDirectDocument(stringifyMeshJson(next))
}
